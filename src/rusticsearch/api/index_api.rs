use std::fs;

use kite_rocksdb::RocksDBIndexStore;

use analysis::AnalyzerSpec;
use analysis::tokenizers::TokenizerSpec;
use analysis::filters::FilterSpec;
use analysis::ngram_generator::Edge;
use index::Index;

use api::persistent;
use api::iron::prelude::*;
use api::iron::status;
use api::router::Router;
use api::utils::json_response;


pub fn view_get_index(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    let indices = system.indices.read().unwrap();

    // Check index exists
    get_index_or_404!(indices, *index_name);

    // TODO
    return Ok(json_response(status::Ok, "{}"));
}


pub fn view_put_index(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    let mut indices = system.indices.write().unwrap();

    // Load data from body
    // let data = json_from_request_body!(req);

    // Create index
    let mut indices_dir = system.get_indices_dir();
    indices_dir.push(index_name);
    indices_dir.set_extension("rsi");
    let mut index = Index::new(index_name.clone().to_owned(), RocksDBIndexStore::create(indices_dir).unwrap());

    // Insert standard and edgengram analyzers
    // TODO: Load these from index settings
    index.analyzers.insert("standard".to_string(), AnalyzerSpec {
        tokenizer: TokenizerSpec::Standard,
        filters: vec![
            FilterSpec::Lowercase,
            FilterSpec::ASCIIFolding,
        ]
    });
    index.analyzers.insert("edgengram_analyzer".to_string(), AnalyzerSpec {
        tokenizer: TokenizerSpec::Standard,
        filters: vec![
            FilterSpec::Lowercase,
            FilterSpec::ASCIIFolding,
            FilterSpec::NGram {
                min_size: 2,
                max_size: 15,
                edge: Edge::Left
            },
        ]
    });

    // TODO: load settings

    let index_ref = indices.insert(index);
    indices.names.insert_canonical(index_name.clone().to_owned(), index_ref).unwrap();

    system.log.info("[api] created index", b!("index" => *index_name));

    return Ok(json_response(status::Ok, "{\"acknowledged\": true}"));
}


pub fn view_delete_index(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    let mut indices = system.indices.write().unwrap();

    // Make sure the index exists
    get_index_or_404!(indices, *index_name);

    // Remove index from array
    let index_ref = indices.names.find_one(*index_name);

    if let Some(index_ref) = index_ref {
        indices.remove(&index_ref);

        // Delete canonical name
        indices.names.delete_canonical(&index_name, index_ref).unwrap();
    }

    // Delete file
    let mut indices_dir = system.get_indices_dir();
    indices_dir.push(index_name);
    indices_dir.set_extension("rsi");
    match fs::remove_dir_all(&indices_dir) {
        Ok(()) => {},
        Err(e) => {
            system.log.warn("[api] failed to delete index data", b!("index" => *index_name, "error" => format!("{}", e)));
        }
    }

    system.log.info("[api] deleted index", b!("index" => *index_name));

    return Ok(json_response(status::Ok, "{\"acknowledged\": true}"));
}


pub fn view_post_refresh_index(_req: &mut Request) -> IronResult<Response> {
    // let ref system = get_system!(req);
    // let ref index_name = read_path_parameter!(req, "index").unwrap_or("");

    // Lock index array
    // TODO
    // let mut indices = system.indices.write().unwrap();

    // TODO: {"_shards":{"total":10,"successful":5,"failed":0}}
    return Ok(json_response(status::Ok, "{\"acknowledged\": true}"));
}