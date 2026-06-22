pub mod common;
use serde_json::json;

#[tokio::test]
async fn test_create_and_move_folder() {
    let ctx = common::TestContext::new().await;

    // Register and login to get a token
    let _ = ctx
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": "testfolder@example.com",
            "password": "password123"
        }))
        .await;

    let res_login = ctx
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": "testfolder@example.com",
            "password": "password123"
        }))
        .await;

    res_login.assert_status_ok();
    let token = res_login.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Create root folder
    let res = ctx
        .server
        .post("/api/documents/folders")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "Root Folder",
            "owner_id": 2
        }))
        .await;
    res.assert_status_ok();
    let root_folder = res.json::<serde_json::Value>();
    let root_id = root_folder["id"].as_i64().unwrap();

    // Create child folder
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
    res2.assert_status_ok();
    let child_folder = res2.json::<serde_json::Value>();
    let child_id = child_folder["id"].as_i64().unwrap();

    // Verify Tree
    let res_tree = ctx
        .server
        .get("/api/documents/tree")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
    res_tree.assert_status_ok();
    let tree = res_tree.json::<serde_json::Value>();

    // In our tree structure, the flat folders list should have the child folder with parent_id
    let folders = tree["folders"].as_array().unwrap();
    let child_node = folders
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == child_id)
        .unwrap();

    assert_eq!(child_node["parent_folder_id"].as_i64().unwrap(), root_id);

    // Test moving folder out to root
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
    res_move.assert_status_ok();

    // Verify both are now roots (parent_folder_id is null)
    let res_tree2 = ctx
        .server
        .get("/api/documents/tree")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
    let tree2 = res_tree2.json::<serde_json::Value>();
    let folders2 = tree2["folders"].as_array().unwrap();
    let r1 = folders2
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == root_id)
        .unwrap();
    let c1 = folders2
        .iter()
        .find(|f| f["id"].as_i64().unwrap() == child_id)
        .unwrap();

    assert!(r1["parent_folder_id"].is_null());
    assert!(c1["parent_folder_id"].is_null());
}
