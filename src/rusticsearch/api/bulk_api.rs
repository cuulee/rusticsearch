use std::io::Read;
use std::collections::HashMap;

use rustc_serialize::json::{self, Json};
use kite::document::DocRef;
use kite_rocksdb::segment_builder::SegmentBuilder;

use document::DocumentSource;

use api::persistent;
use api::iron::prelude::*;
use api::iron::status;
use api::utils::{json_response};


pub fn view_post_bulk(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);

    // Lock index array
    let indices = system.indices.read().unwrap();

    // Load data from body
    let mut payload = String::new();
    req.body.read_to_string(&mut payload).unwrap();

    let mut items = Vec::new();
    let mut segment_builder = SegmentBuilder::new();
    let mut key_docid_map = HashMap::new();

    // Iterate
    let mut payload_lines = payload.split('\n');
    loop {
        let action_line = payload_lines.next();

        // Check if end of input
        if action_line == None || action_line == Some("") {
            break;
        }

        // Parse action line
        let action_json = parse_json!(&action_line.unwrap());

        // Check action
        // Action should be an object with only one key, the key name indicates the action and
        // the value is the parameters for that action
        let action_name = action_json.as_object().unwrap().keys().nth(0).unwrap();
        let action_params = action_json.as_object()
                                       .unwrap()
                                       .get(action_name)
                                       .unwrap()
                                       .as_object()
                                       .unwrap();

        let doc_id = action_params.get("_id").unwrap().as_string().unwrap();
        let doc_type = action_params.get("_type").unwrap().as_string().unwrap();
        let doc_index = action_params.get("_index").unwrap().as_string().unwrap();

        match action_name.as_ref() {
            "index" => {
                let doc_line = payload_lines.next();
                let doc_json = parse_json!(&doc_line.unwrap());;

                // Find index
                let index = get_index_or_404!(indices, doc_index);

                let doc = {
                    // Find mapping
                    let mapping = match index.get_mapping_by_name(doc_type) {
                        Some(mapping) => mapping,
                        None => {
                            return Ok(json_response(status::NotFound, "{\"message\": \"Mapping not found\"}"));
                        }
                    };

                    // Create document
                    let document_source = DocumentSource {
                        key: doc_id.to_string(),
                        data: doc_json,
                    };
                    document_source.prepare(mapping)
                };

                let doc_internal_id = segment_builder.add_document(&doc).unwrap();
                key_docid_map.insert(doc_id.to_string(), doc_internal_id);

                // Insert into "items" array
                let mut item = HashMap::new();
                // TODO: "create" may not always be right
                item.insert("create", action_params.clone());
                items.push(item);
            }
            _ => {
                warn!("Unrecognised action! {}", action_name);
            }
        }
    }

    let index = get_index_or_404!(indices, "verdant");

    // Write segment data
    let segment = index.store.write_segment(&segment_builder).unwrap();

    // Update document index
    for (doc_key, doc_id) in key_docid_map {
        let doc_ref = DocRef::from_segment_ord(segment, doc_id);
        index.store.document_index.insert_or_replace_key(&index.store.db, &doc_key.as_bytes().iter().cloned().collect(), doc_ref).unwrap();
    }

    return Ok(json_response(status::Ok,
                            format!("{{\"took\": {}, \"items\": {}}}",
                                    items.len(),
                                    json::encode(&items).unwrap())));
}
