pub mod common;
use serde_json::json;

#[tokio::test]
async fn test_create_and_move_document() {
    let ctx = common::TestContext::new().await;

    // Create a folder
    let res_folder = ctx
        .server
        .post("/api/documents/folders")
        .json(&json!({
            "name": "Project A",
            "owner_id": 1
        }))
        .await;
    res_folder.assert_status_ok();
    let folder_id = res_folder.json::<serde_json::Value>()["id"]
        .as_i64()
        .unwrap();

    // Create a document via normal route first to get a valid document model
    let res_init = ctx
        .server
        .post("/api/documents")
        .json(&json!({
            "name": "AHP Model",
            "owner_id": 1,
            "aggregation_method": "AIJ",
            "folder_id": serde_json::Value::Null
        }))
        .await;
    res_init.assert_status_ok();
    let doc_init = res_init.json::<serde_json::Value>();
    let doc_id = doc_init["id"].as_i64().unwrap();

    // Now save full document
    let mut export_doc = json!({
        "document": doc_init,
        "nodes": [
            {
                "id": 1,
                "document_id": doc_id,
                "parent_node_id": serde_json::Value::Null,
                "name": "Goal",
                "node_type": "Goal"
            }
        ],
        "comparisons": []
    });

    let res_doc = ctx
        .server
        .post(&format!("/api/documents/{}/full", doc_id))
        .json(&export_doc)
        .await;
    res_doc.assert_status_ok();
    let doc_id = res_doc.json::<serde_json::Value>()["id"].as_i64().unwrap();

    // Move document into folder
    let res_move = ctx
        .server
        .post(&format!("/api/documents/{}/move", doc_id))
        .json(&json!({
            "folder_id": folder_id
        }))
        .await;
    res_move.assert_status_ok();

    // Verify it moved in the tree
    let res_tree = ctx.server.get("/api/documents/tree").await;
    res_tree.assert_status_ok();
    let tree = res_tree.json::<serde_json::Value>();

    let docs = tree["documents"].as_array().unwrap();
    let moved_doc = docs
        .iter()
        .find(|d| d["id"].as_i64().unwrap() == doc_id)
        .unwrap();

    assert_eq!(moved_doc["folder_id"].as_i64().unwrap(), folder_id);
}
