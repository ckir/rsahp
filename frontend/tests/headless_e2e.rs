// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
use backend::{create_router, setup_schema};
use egui_kittest::{Harness, kittest::Queryable};
use frontend::{
    config::AppConfig,
    ui::{
        RsahpApp,
        auth::AuthState,
        document_window::{CriteriaNode, DocumentState, DocumentTab},
    },
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use tokio::net::TcpListener;

// 1. Backend bootstrapper
async fn start_test_backend() -> (u16, DatabaseConnection) {
    // In-memory sqlite
    let db = Database::connect("sqlite::memory:").await.unwrap();
    setup_schema(&db).await.unwrap();

    // Insert mock document to avoid 404 on save
    let builder = db.get_database_backend();
    let _ = db.execute(sea_orm::Statement::from_string(
        builder,
        "INSERT INTO \"document\" (id, name, owner_id, version, aggregation_method, created_at, folder_id) VALUES (1, 'Test Document', 1, 1, 'AIJ', '2026-06-21T00:00:00Z', NULL);".to_string()
    )).await;

    // Create the router
    let app = create_router(db.clone());

    // Bind to random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (port, db)
}

use std::cell::RefCell;
use std::rc::Rc;

#[tokio::test]
async fn test_save_document_payload() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_test_writer()
        .try_init();

    let (port, db) = start_test_backend().await;
    let api_url = format!("http://127.0.0.1:{}/api/documents", port);

    // Setup default configuration with dynamic API URL.
    let mut config = AppConfig::default();
    config.api_url = Some(api_url.clone());

    // Initialize the app with the config.
    let app = Rc::new(RefCell::new(RsahpApp::new(config)));

    // Seed Auth State
    app.borrow_mut().auth_state = AuthState {
        jwt_token: Some("fake_token_for_test".to_string()),
        logged_in_user_id: Some(1),
        is_admin: false,
        ..Default::default()
    };

    // Seed Document State
    let mut doc = DocumentState::new(1, "Test Document");
    doc.version = 1;
    doc.is_loaded = true;
    doc.goal = "Test Goal".to_string();
    doc.criteria = CriteriaNode {
        id: 1,
        name: "Goal".to_string(),
        cost: None,
        node_type: "Goal".to_string(),
        children: vec![
            CriteriaNode {
                id: 2,
                name: "Crit1".to_string(),
                cost: None,
                node_type: "Criteria".to_string(),
                children: vec![],
            },
            CriteriaNode {
                id: 3,
                name: "Crit2".to_string(),
                cost: None,
                node_type: "Criteria".to_string(),
                children: vec![],
            },
        ],
    };
    doc.next_id = 4;
    doc.active_tab = DocumentTab::Comparisons;
    // Add a comparison
    doc.saaty_values.insert((2, 3), 3.0);

    app.borrow_mut().open_documents.push(doc);

    // Build the egui kittest harness
    let app_clone = app.clone();
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(1200.0, 800.0))
        .build_ui(move |ctx| {
            // Call the app render loop once
            app_clone.borrow_mut().render(ctx);
        });

    // Step the harness to draw the UI.
    harness.step();

    // Find the Save button and click it
    let save_btn = harness.get_by_label("💾 Save");
    save_btn.click();

    // Wait until the save status changes from "Saving..."
    let mut max_wait = 100; // 100 * 50ms = 5s
    while max_wait > 0 {
        harness.step();
        if app.borrow().open_documents[0].save_status.as_deref() != Some("Saving...") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        max_wait -= 1;
    }

    let status = app.borrow().open_documents[0].save_status.clone();
    // Verify the document's save status
    assert!(
        status.as_ref().unwrap().contains("✅ Saved"),
        "Save failed or status was not updated properly: {:?}",
        status
    );

    // Final DB Assertion
    use backend::entity::comparison;
    use sea_orm::EntityTrait;
    let comparisons = comparison::Entity::find().all(&db).await.unwrap();
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].node_a_id, 2);
    assert_eq!(comparisons[0].node_b_id, 3);
    assert_eq!(comparisons[0].respondent_id, 1);
    assert_eq!(comparisons[0].saaty_value, 3.0);
}
