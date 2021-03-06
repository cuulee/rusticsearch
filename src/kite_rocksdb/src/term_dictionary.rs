use std::str;
use std::sync::{Mutex, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::BTreeMap;

use rocksdb::{self, DB};
use kite::{Term, TermRef};
use kite::query::term_selector::TermSelector;

use key_builder::KeyBuilder;


/// Manages the index's "term dictionary"
///
/// Because terms can be very long, we don't use their byte-representations as
/// keys. We generate a unique number for each one to use instead.
///
/// The term dictionary is a mapping between terms and their internal IDs
/// (aka. TermRef). It is entirely held in memory and persisted to the disk.
pub struct TermDictionaryManager {
    next_term_ref: AtomicUsize,
    terms: RwLock<BTreeMap<Term, TermRef>>,
    write_lock: Mutex<i32>,
}


impl TermDictionaryManager {
    /// Generates a new term dictionary
    pub fn new(db: &DB) -> Result<TermDictionaryManager, rocksdb::Error> {
        // TODO: Raise error if .next_term_ref already exists
        // Next term ref
        try!(db.put(b".next_term_ref", b"1"));

        Ok(TermDictionaryManager {
            next_term_ref: AtomicUsize::new(1),
            terms: RwLock::new(BTreeMap::new()),
            write_lock: Mutex::new(0),
        })
    }

    /// Loads the term dictionary from an index
    pub fn open(db: &DB) -> Result<TermDictionaryManager, rocksdb::Error> {
        let next_term_ref = match try!(db.get(b".next_term_ref")) {
            Some(next_term_ref) => {
                next_term_ref.to_utf8().unwrap().parse::<u32>().unwrap()
            }
            None => 1,  // TODO: error
        };

        // Read dictionary
        let mut terms = BTreeMap::new();
        let mut iter = db.iterator();
        iter.seek(b"t");
        while iter.next() {
            let k = iter.key().unwrap();

            if k[0] != b't' {
                break;
            }

            let term_ref = TermRef::new(str::from_utf8(&iter.value().unwrap()).unwrap().parse::<u32>().unwrap());
            terms.insert(Term::from_bytes(&k[1..]), term_ref);
        }

        Ok(TermDictionaryManager {
            next_term_ref: AtomicUsize::new(next_term_ref as usize),
            terms: RwLock::new(terms),
            write_lock: Mutex::new(0),
        })
    }

    /// Retrieves the TermRef for the given term
    pub fn get(&self, term: &Term) -> Option<TermRef> {
        self.terms.read().unwrap().get(term).cloned()
    }

    /// Iterates over terms in the dictionary which match the selector
    pub fn select(&self, term_selector: &TermSelector) -> Vec<TermRef> {
        self.terms.read().unwrap().iter()
            .filter(|&(term, _term_ref)| {
                term_selector.matches(term)
            })
            .map(|(_term, term_ref)| *term_ref)
            .collect()
    }

    /// Retrieves the TermRef for the given term, adding the term to the
    /// dictionary if it doesn't exist
    pub fn get_or_create(&self, db: &DB, term: &Term) -> Result<TermRef, rocksdb::Error> {
        if let Some(term_ref) = self.get(term) {
            return Ok(term_ref);
        }

        // Term doesn't exist in the term dictionary

        // Increment next_term_ref
        let next_term_ref = self.next_term_ref.fetch_add(1, Ordering::SeqCst) as u32;
        try!(db.put(b".next_term_ref", (next_term_ref + 1).to_string().as_bytes()));

        // Create term ref
        let term_ref = TermRef::new(next_term_ref);

        // Get write lock
        // Note: We have a separate lock so we don't need to keep an exclusive
        // lock on the in-memory term dictionary while writing to disk, as this
        // blocks readers.
        let _guard = self.write_lock.lock().unwrap();

        // It's possible that another thread has written the term to the dictionary
        // since we checked earlier. If this is the case, We should forget about
        // writing our TermRef and use the one that has been inserted already.
        if let Some(term_ref) = self.terms.read().unwrap().get(term) {
            return Ok(*term_ref);
        }

        // Write it to the on-disk term dictionary
        let kb = KeyBuilder::term_dict_mapping(term.as_bytes());
        try!(db.put(kb.key(), next_term_ref.to_string().as_bytes()));

        // Write it to the term dictionary
        self.terms.write().unwrap().insert(term.clone(), term_ref);;

        Ok(term_ref)
    }
}
