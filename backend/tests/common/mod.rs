// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Shared testing utilities and common setup functions.

use axum_test::TestServer;
use backend::{create_router, setup_schema};
use sea_orm::{Database, DatabaseConnection};

/// Module containing golden data matrices for AHP tests.
pub mod golden_data;

/// Test context encapsulating the test server and database connection.
pub struct TestContext {
    /// The Axum test server instance.
    pub server: TestServer,
    /// The database connection used by the tests.
    pub db: DatabaseConnection,
}

impl TestContext {
    /// Creates a new `TestContext` by initializing an in-memory database and configuring the app router.
    pub async fn new() -> Self {
        // Use an in-memory SQLite database unique to this test
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // Run setup and migrations on the in-memory database
        setup_schema(&db).await.unwrap();

        // Create the Axum router with the established database connection
        let app = create_router(db.clone());

        // Wrap the router in axum_test's TestServer for making HTTP requests
        let server = TestServer::new(app);

        // Return the constructed test context
        TestContext { server, db }
    }
}
