use std::io::Read;
use std::collections::BTreeMap;

use rustc_serialize::json::{self, Json};

use system::System;
use search::document::DocumentSource;

use api::persistent;
use api::iron::prelude::*;
use api::iron::status;
use api::router::Router;
use api::utils::json_response;


pub fn view_get_doc(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");
    let ref mapping_name = read_path_parameter!(req, "mapping").unwrap_or("");
    let ref doc_key = read_path_parameter!(req, "doc").unwrap_or("");

    // Lock index array
    let indices = system.indices.read().unwrap();

    // Get index
    let index = get_index_or_404!(indices, *index_name);

    // Find mapping
    let mapping = match index.get_mapping_by_name(mapping_name) {
        Some(mapping) => mapping,
        None => {
            return Ok(json_response(status::NotFound, "{\"message\": \"Mapping not found\"}"));
        }
    };

    // Find document
    let doc = match index.store.get_document_by_key(doc_key) {
        Some(doc) => doc,
        None => {
            return Ok(json_response(status::NotFound, "{\"message\": \"Document not found\"}"));
        }
    };


    // Build JSON document
    // TODO: This is probably completely wrong
    let mut json_object = BTreeMap::new();
    for (field_name, field_value) in doc.fields.iter() {
        json_object.insert(field_name.clone(), Json::Array(field_value.iter().map(|v| v.term.as_json()).collect::<Vec<_>>()));
    }

    let json = Json::Object(json_object);
    return Ok(json_response(status::Ok, json::encode(&json).unwrap()));
}


pub fn view_put_doc(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");
    let ref mapping_name = read_path_parameter!(req, "mapping").unwrap_or("");
    let ref doc_key = read_path_parameter!(req, "doc").unwrap_or("");

    // Lock index array
    let mut indices = system.indices.write().unwrap();

    // Get index
    let mut index = get_index_or_404_mut!(indices, *index_name);

    let doc = {
        // Find mapping
        let mapping = match index.get_mapping_by_name(mapping_name) {
            Some(mapping) => mapping,
            None => {
                return Ok(json_response(status::NotFound, "{\"message\": \"Mapping not found\"}"));
            }
        };

        // Create document
        if let Some(data) = json_from_request_body!(req) {
            let document_source = DocumentSource {
                key: doc_key.to_string(),
                data: data,
            };
            document_source.prepare(mapping)
        } else {
            return Ok(json_response(status::NotFound, "{\"message\": \"No data\"}"));
        }
    };

    index.store.insert_or_update_document(doc);

    // TODO: {"_index":"wagtail","_type":"searchtests_searchtest","_id":"searchtests_searchtest:5378","_version":1,"created":true}
    return Ok(json_response(status::Ok, "{}"));
}


pub fn view_delete_doc(req: &mut Request) -> IronResult<Response> {
    let ref system = get_system!(req);
    let ref index_name = read_path_parameter!(req, "index").unwrap_or("");
    let ref mapping_name = read_path_parameter!(req, "mapping").unwrap_or("");
    let ref doc_key = read_path_parameter!(req, "doc").unwrap_or("");

    // Lock index array
    let mut indices = system.indices.write().unwrap();

    // Get index
    let mut index = get_index_or_404_mut!(indices, *index_name);

    // Make sure the document exists
    if !index.store.contains_document_key(doc_key) {
        return Ok(json_response(status::NotFound, "{\"message\": \"Document not found\"}"));
    }

    // Delete document
    index.store.remove_document_by_key(doc_key);

    return Ok(json_response(status::Ok, "{}"));
}
