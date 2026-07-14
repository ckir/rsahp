// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Integration tests for document management functionality.

/// Common setup and utilities for tests.
pub mod common;
use serde_json::json;

/// Tests the creation of a document and moving it into a folder.
#[tokio::test]
async fn test_create_and_move_document() {
    // Initialize the test context with an in-memory database and test server
    let ctx = common::TestContext::new().await;

    // Register a new user to obtain credentials
    let _ = ctx
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    // Login using the registered credentials to receive an auth token
    let res_login = ctx
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    // Assert that the login was successful
    res_login.assert_status_ok();

    // Extract the token from the login response
    let token = res_login.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a new folder named 'Project A'
    let res_folder = ctx
        .server
        .post("/api/documents/folders")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "Project A",
            "owner_id": 2
        }))
        .await;

    // Assert that folder creation was successful
    res_folder.assert_status_ok();

    // Extract the folder ID from the response
    let folder_id = res_folder.json::<serde_json::Value>()["id"]
        .as_i64()
        .unwrap();

    // Create a document via the normal route first to get a valid document model
    let res_init = ctx
        .server
        .post("/api/documents")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "AHP Model",
            "owner_id": 2,
            "aggregation_method": "AIJ",
            "folder_id": serde_json::Value::Null
        }))
        .await;

    // Assert that initial document creation was successful
    res_init.assert_status_ok();

    // Extract document data and ID
    let doc_init = res_init.json::<serde_json::Value>();
    let doc_id = doc_init["id"].as_i64().unwrap();

    // Prepare a full document payload including nodes
    let export_doc = json!({
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

    // Save the full document using the full endpoint
    let res_doc = ctx
        .server
        .post(&format!("/api/documents/{}/full", doc_id))
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&export_doc)
        .await;

    // Assert that saving the full document was successful
    res_doc.assert_status_ok();

    // Extract the updated document ID
    let doc_id = res_doc.json::<serde_json::Value>()["id"].as_i64().unwrap();

    // Move the document into the previously created folder
    let res_move = ctx
        .server
        .post(&format!("/api/documents/{}/move", doc_id))
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "folder_id": folder_id
        }))
        .await;

    // Assert that the move operation was successful
    res_move.assert_status_ok();

    // Fetch the document tree to verify the move
    let res_tree = ctx
        .server
        .get("/api/documents/tree")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    // Assert that fetching the tree was successful
    res_tree.assert_status_ok();

    // Parse the tree response
    let tree = res_tree.json::<serde_json::Value>();

    // Extract documents from the tree structure
    let docs = tree["documents"].as_array().unwrap();

    // Find the document we just moved
    let moved_doc = docs
        .iter()
        .find(|d| d["id"].as_i64().unwrap() == doc_id)
        .unwrap();

    // Assert that the document's folder_id matches the folder we moved it to
    assert_eq!(moved_doc["folder_id"].as_i64().unwrap(), folder_id);
}
