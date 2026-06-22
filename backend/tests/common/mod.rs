use axum_test::TestServer;
use sea_orm::{Database, DatabaseConnection};
use backend::{create_router, setup_schema};

pub mod golden_data;

pub struct TestContext {
    pub server: TestServer,
    pub db: DatabaseConnection,
}

impl TestContext {
    pub async fn new() -> Self {
        // Use an in-memory SQLite database unique to this test
        let db = Database::connect("sqlite::memory:").await.unwrap();
        
        // Run setup and migrations
        setup_schema(&db).await.unwrap();

        // Create the Axum router
        let app = create_router(db.clone());

        // Wrap in axum_test's TestServer
        let server = TestServer::new(app);

        TestContext { server, db }
    }
}
