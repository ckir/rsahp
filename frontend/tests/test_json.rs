use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportedDocument {
    pub document: DocumentModel,
    pub nodes: Vec<NodeModel>,
    pub comparisons: Vec<ComparisonModel>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentModel {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub version: i32,
    pub aggregation_method: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeModel {
    pub id: i32,
    pub document_id: i32,
    pub parent_node_id: Option<i32>,
    pub name: String,
    pub node_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComparisonModel {
    pub id: i32,
    pub document_id: i32,
    pub respondent_email: String,
    pub parent_node_id: i32,
    pub node_a_id: i32,
    pub node_b_id: i32,
    pub saaty_value: f64,
}

#[test]
fn test_parse() {
    let json = r#"{"document":{"id":1,"name":"Test Doc","owner_id":1,"version":1,"aggregation_method":"GeometricMean","created_at":"2026-06-21T22:59:01.916741700Z"},"nodes":[],"comparisons":[]}"#;
    match serde_json::from_str::<ExportedDocument>(json) {
        Ok(doc) => println!("Success: {:?}", doc),
        Err(e) => panic!("Error: {}", e),
    }
}
