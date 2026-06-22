//! Main library module for the application.
//! This module exports various submodules for APIs, configuration, and entities.
//! It also provides functions for initializing the database schema and routing.

pub mod ahp;
pub mod api;
pub mod api_admin;
pub mod api_auth;
pub mod api_docs;
pub mod config;
pub mod entity;

use sea_orm::{ConnectionTrait, DbErr, Schema};

/// Sets up the initial database schema and applies any required migrations.
pub async fn setup_schema(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // Get the database backend specific builder
    let builder = db.get_database_backend();
    // Initialize the schema builder
    let schema = Schema::new(builder);

    // Create a list of statements for creating tables
    let stmts = vec![
        builder.build(
            schema
                .create_table_from_entity(entity::user_group::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::user::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::folder::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::document::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::node::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::comparison::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::user_group_membership::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::document_user_assignment::Entity)
                .if_not_exists(),
        ),
        builder.build(
            schema
                .create_table_from_entity(entity::document_group_assignment::Entity)
                .if_not_exists(),
        ),
    ];

    // Execute each statement to create tables if they do not exist
    for stmt in stmts {
        db.execute(stmt).await?;
    }

    // Add folder_id to document if it doesn't exist
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "ALTER TABLE \"document\" ADD COLUMN \"folder_id\" integer;".to_string(),
        ))
        .await;

    // Add cost to node if it doesn't exist
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "ALTER TABLE \"node\" ADD COLUMN \"cost\" REAL;".to_string(),
        ))
        .await;

    // Add is_deleted to user if it doesn't exist
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "ALTER TABLE \"user\" ADD COLUMN \"is_deleted\" BOOLEAN NOT NULL DEFAULT 0;"
                .to_string(),
        ))
        .await;

    // Add parent_id to user_group if it doesn't exist
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "ALTER TABLE \"user_group\" ADD COLUMN \"parent_id\" INTEGER;".to_string(),
        ))
        .await;

    // Change respondent_email to respondent_id in comparison if needed
    // SQLite doesn't easily support drop column without version 3.35.0, so we just add the new column.
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "ALTER TABLE \"comparison\" ADD COLUMN \"respondent_id\" INTEGER NOT NULL DEFAULT 1;"
                .to_string(),
        ))
        .await;

    // Insert mock user group and user for development
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "INSERT OR IGNORE INTO \"user_group\" (id, name) VALUES (1, 'Admin Group');"
                .to_string(),
        ))
        .await;

    // Use bcrypt or a dummy hash for now. The string 'admin' hashed will be needed later,
    // but for now let's just insert 'hash'.
    let _ = db.execute(sea_orm::Statement::from_string(
        builder,
        "INSERT OR IGNORE INTO \"user\" (id, username, password_hash, is_admin, is_deleted) VALUES (1, 'admin', 'hash', 1, 0);".to_string()
    )).await;

    // Also add admin to user_group_membership
    let _ = db.execute(sea_orm::Statement::from_string(
        builder,
        "INSERT OR IGNORE INTO \"user_group_membership\" (id, user_id, group_id) VALUES (1, 1, 1);".to_string()
    )).await;

    // Log the successful initialization
    tracing::info!("Database schema initialized.");
    Ok(())
}

/// Creates and configures the main Axum application router.
pub fn create_router(db: sea_orm::DatabaseConnection) -> axum::Router {
    // Build and return the Router instance
    axum::Router::new()
        // Define root route
        .route(
            "/",
            axum::routing::get(|| async { "rsahp backend running" }),
        )
        // Nest authentication endpoints
        .nest("/api/auth", api_auth::router().with_state(db.clone()))
        // Nest admin endpoints
        .nest("/api/admin", api_admin::router().with_state(db.clone()))
        // Nest AHP calculation endpoints
        .nest("/api/ahp", api::router())
        // Nest document management endpoints
        .nest("/api/documents", api_docs::router().with_state(db))
        // Apply middleware for request logging
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                // Log method and URI
                tracing::info!("-> {} {}", req.method(), req.uri());
                // Log request headers
                tracing::info!("Headers: {:#?}", req.headers());
                // Continue to the next handler
                next.run(req).await
            },
        ))
}
