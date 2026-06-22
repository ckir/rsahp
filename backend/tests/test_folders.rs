//! Integration tests for folder creation and organization.

/// Common setup and utilities for tests.
pub mod common;
use serde_json::json;

/// Tests the creation of a root folder, a child folder, and moving the child folder to the root.
#[tokio::test]
async fn test_create_and_move_folder() {
    // Initialize the test context with an in-memory database and test server
    let ctx = common::TestContext::new().await;

    // Register a new user to obtain credentials
    let _ = ctx
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": "testfolder@example.com",
            "password": "password123"
        }))
        .await;

    // Login using the registered credentials to receive an auth token
    let res_login = ctx
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": "testfolder@example.com",
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

    // Create a new root folder
    let res = ctx
        .server
        .post("/api/documents/folders")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "Root Folder",
            "owner_id": 2
        }))
        .await;
        
    // Assert that root folder creation was successful
    res.assert_status_ok();
    
    // Extract root folder details
    let root_folder = res.json::<serde_json::Value>();
    let root_id = root_folder["id"].as_i64().unwrap();

    // Create a child folder inside the root folder
    let res2 = ctx
        .server
        .post("/api/documents/folders")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "Child Folder",
            "owner_id": 2,
            "parent_folder_id": root_id
        }))
        .await;
        
    // Assert that child folder creation was successful
    res2.assert_status_ok();
    
    // Extract child folder details
    let child_folder = res2.json::<serde_json::Value>();
    let child_id = child_folder["id"].as_i64().unwrap();

    // Fetch the document tree to verify the folder hierarchy
    let res_tree = ctx
        .server
        .get("/api/documents/tree")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
        
    // Assert that fetching the tree was successful
    res_tree.assert_status_ok();
    
    // Parse the tree response
    let tree = res_tree.json::<serde_json::Value>();

    // In our tree structure, the flat folders list should have the child folder with parent_id
    let folders = tree["folders"].as_array().unwrap();
    
    // Find the child folder in the tree
    let child_node = folders
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == child_id)
        .unwrap();

    // Assert that the child folder's parent_folder_id matches the root folder ID
    assert_eq!(child_node["parent_folder_id"].as_i64().unwrap(), root_id);

    // Test moving the child folder out to the root level
    let res_move = ctx
        .server
        .post(&format!("/api/documents/folders/{}", child_id))
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "Child Folder",
            "owner_id": 2,
            "parent_folder_id": serde_json::Value::Null
        }))
        .await;
        
    // Assert that the move operation was successful
    res_move.assert_status_ok();

    // Fetch the updated document tree to verify both are now root folders
    let res_tree2 = ctx
        .server
        .get("/api/documents/tree")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
        
    // Parse the updated tree response
    let tree2 = res_tree2.json::<serde_json::Value>();
    let folders2 = tree2["folders"].as_array().unwrap();
    
    // Find both folders in the updated tree
    let r1 = folders2
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == root_id)
        .unwrap();
    let c1 = folders2
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == child_id)
        .unwrap();

    // Assert that both folders now have a null parent_folder_id
    assert!(r1["parent_folder_id"].is_null());
    assert!(c1["parent_folder_id"].is_null());
}
