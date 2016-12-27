use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use index::Index;


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct IndexRef(u32);


#[derive(Debug)]
enum Name {
    /// This is the canonical name of an index
    Canonical(IndexRef),

    /// This is an alias
    Alias(String),
}


#[derive(Debug)]
pub struct NameRegistry {
    names: HashMap<String, Name>,
}


impl NameRegistry {
    pub fn insert_canonical(&mut self, name: String, index_ref: IndexRef) -> Result<(), ()> {
        if let Some(_) = self.names.get(&name) {
            return Err(());
        }

        self.names.insert(name, Name::Canonical(index_ref));
        Ok(())
    }

    pub fn delete_canonical(&mut self, name: &str, index_ref: IndexRef) -> Result<(), ()> {
        if let Some(&Name::Canonical(actual_index_ref)) = self.names.get(name) {
            if actual_index_ref != index_ref {
                return Err(());
            }
        } else {
            return Err(());
        }

        self.names.remove(name);
        Ok(())
    }

    pub fn insert_alias(&mut self, name: String, selector: String) -> Result<(), ()> {
        if let Some(_) = self.names.get(&name) {
            return Err(());
        }

        self.names.insert(name, Name::Alias(selector));
        Ok(())
    }

    pub fn insert_or_replace_alias(&mut self, name: String, selector: String) -> Result<Option<String>, ()> {
        if let Some(&Name::Canonical(_)) = self.names.get(&name) {
            // Cannot replace if it is a canonical name
            return Err(());
        }

        let old_alias = self.names.insert(name, Name::Alias(selector));
        match old_alias {
            Some(Name::Alias(old_alias)) => {
                 Ok(Some(old_alias))
            }
            Some(Name::Canonical(_)) => {
                unreachable!();
            }
            None => {
                Ok(None)
            }
        }
    }

    pub fn find(&self, selector: &str) -> Vec<IndexRef> {
        let mut indices = Vec::new();

        // Find name
        let name = self.names.get(selector);

        // Resolve the name if we have one
        if let Some(name) = name {
            let mut exclusion_list = Vec::new();

            exclusion_list.push(selector.to_string());
            self.resolve_name(name, &mut exclusion_list, &mut indices);
            exclusion_list.pop();

            debug_assert!(exclusion_list.len() == 0);
        }

        indices
    }

    pub fn find_one(&self, selector: &str) -> Option<IndexRef> {
        let index_refs = self.find(selector);

        if index_refs.is_empty() {
            None
        } else {
            Some(index_refs[0])
        }
    }

    fn resolve_name(&self, name: &Name, mut exclusion_list: &mut Vec<String>, mut indices: &mut Vec<IndexRef>) {
        match *name {
            Name::Canonical(ref index_ref) => indices.push(*index_ref),
            Name::Alias(ref selector) => {
                // Find name
                let name = self.names.get(selector);

                // Resolve the name if we have one
                if let Some(name) = name {
                    exclusion_list.push(selector.to_string());
                    self.resolve_name(name, exclusion_list, &mut indices);
                    exclusion_list.pop();
                }
            }
        }
    }
}


#[derive(Debug)]
pub struct IndexRegistry {
    ref_counter: u32,
    indices: HashMap<IndexRef, Index>,
    pub names: NameRegistry,
}


impl IndexRegistry {
    pub fn new() -> IndexRegistry {
        IndexRegistry {
            ref_counter: 1,
            indices: HashMap::new(),
            names: NameRegistry {
                names: HashMap::new(),
            },
        }
    }

    pub fn insert(&mut self, index: Index) -> IndexRef {
        let index_ref = IndexRef(self.ref_counter);
        self.ref_counter += 1;

        self.indices.insert(index_ref, index);

        index_ref
    }
}


impl Deref for IndexRegistry {
    type Target = HashMap<IndexRef, Index>;

    fn deref(&self) -> &HashMap<IndexRef, Index> {
        &self.indices
    }
}


impl DerefMut for IndexRegistry {
    fn deref_mut(&mut self) -> &mut HashMap<IndexRef, Index> {
        &mut self.indices
    }
}