pub mod ahp;
pub mod api;
pub mod api_docs;
pub mod config;
pub mod entity;

use sea_orm::{ConnectionTrait, DbErr, Schema};

pub async fn setup_schema(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

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
    ];

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

    // Insert mock user group and user for development
    let _ = db
        .execute(sea_orm::Statement::from_string(
            builder,
            "INSERT OR IGNORE INTO \"user_group\" (id, name) VALUES (1, 'Admin Group');"
                .to_string(),
        ))
        .await;

    let _ = db.execute(sea_orm::Statement::from_string(
        builder,
        "INSERT OR IGNORE INTO \"user\" (id, username, password_hash, group_id, is_admin) VALUES (1, 'admin', 'hash', 1, true);".to_string()
    )).await;

    tracing::info!("Database schema initialized.");
    Ok(())
}

pub fn create_router(db: sea_orm::DatabaseConnection) -> axum::Router {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async { "rsahp backend running" }),
        )
        .nest("/api/ahp", api::router())
        .nest("/api/documents", api_docs::router().with_state(db))
}
