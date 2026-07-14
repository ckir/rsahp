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

use sea_orm::DbErr;

/// Sets up the database schema by applying versioned migrations, then (in debug
/// builds only) seeds development login data.
pub async fn setup_schema(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ConnectionTrait, Statement};

    // Cutover guard (UNCONDITIONAL — runs in release AND debug): an existing
    // pre-migration rsahp.db has the 9 app tables but no `seaql_migrations` tracking
    // table. `Migrator::up` would re-issue CREATE TABLE and fail with "table already
    // exists". `m0` uses plain `create_table` (no IF NOT EXISTS), so without this
    // guard a release build would panic with a cryptic DbErr. Detect the exact
    // condition and return a loud, human-readable error on every build profile.
    {
        let backend = db.get_database_backend();
        let has_user_table = db
            .query_one(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='user';".to_owned(),
            ))
            .await?
            .is_some();
        let has_migrations_table = db
            .query_one(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='seaql_migrations';"
                    .to_owned(),
            ))
            .await?
            .is_some();
        if has_user_table && !has_migrations_table {
            eprintln!(
                "\n============================================================\n\
                 MIGRATION CUTOVER: this database predates versioned migrations.\n\
                 It has the application tables but no `seaql_migrations` tracking\n\
                 table, so `Migrator::up` cannot run against it.\n\n\
                 This is dev-only, disposable data. BACK UP or EXPECT DATA LOSS,\n\
                 then delete the database file and restart:\n\n\
                 \x20   rm rsahp.db   (or delete rsahp.db in the project root)\n\n\
                 A fresh DB will be created and migrated automatically.\n\
                 ============================================================\n"
            );
            return Err(DbErr::Custom(
                "pre-migration database detected; delete rsahp.db and restart".to_owned(),
            ));
        }
    }

    // Apply all pending migrations (creates the schema on a fresh DB).
    Migrator::up(db, None)
        .await
        .map_err(|e| DbErr::Custom(format!("migration failed: {e}")))?;

    // Dev-only seed (admin group/user/membership) for out-of-box login. Fenced to
    // debug builds so it is physically absent from release binaries. Uses ActiveModel
    // existence-checked inserts: every column MUST be set explicitly, so a future
    // schema change (a new required column) breaks THIS at compile time — the
    // forcing function the S1-fenced decision relied on. Real insert errors propagate
    // (no silent swallow); an already-seeded DB is a clean no-op via the find check.
    #[cfg(debug_assertions)]
    {
        use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
        if entity::user_group::Entity::find_by_id(1).one(db).await?.is_none() {
            entity::user_group::ActiveModel {
                id: Set(1),
                name: Set("Admin Group".to_owned()),
                parent_id: Set(None),
            }
            .insert(db)
            .await?;
        }
        if entity::user::Entity::find_by_id(1).one(db).await?.is_none() {
            entity::user::ActiveModel {
                id: Set(1),
                username: Set("admin".to_owned()),
                password_hash: Set("hash".to_owned()),
                is_admin: Set(true),
                is_deleted: Set(false),
            }
            .insert(db)
            .await?;
        }
        if entity::user_group_membership::Entity::find_by_id(1)
            .one(db)
            .await?
            .is_none()
        {
            entity::user_group_membership::ActiveModel {
                id: Set(1),
                user_id: Set(1),
                group_id: Set(1),
            }
            .insert(db)
            .await?;
        }
    }

    tracing::info!("Database schema initialized via migrations.");
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

/// The canonical list of application tables, in creation order. This is the
/// single source of truth reused by the drift-guard test's entity-side builder
/// and its table-name-set assertion. A new entity MUST be added here AND given a
/// migration (see CONTRIBUTING).
#[cfg(test)]
pub const APP_TABLES: &[&str] = &[
    "user_group",
    "user",
    "folder",
    "document",
    "node",
    "comparison",
    "user_group_membership",
    "document_user_assignment",
    "document_group_assignment",
];

/// Builds the schema from the LIVE entities via `create_table_from_entity`.
/// Test-only: production startup uses `Migrator::up`. This is the entity side of
/// the drift-guard comparison (consumed by the inline `#[cfg(test)] mod` in Task 12)
/// — do NOT delete as "dead code". Mirrors the exact `builder.build(...if_not_exists())`
/// pattern the original `setup_schema` used (proven to compile).
#[cfg(test)]
pub async fn entity_schema_db(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, Schema};
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    let stmts = vec![
        builder.build(schema.create_table_from_entity(entity::user_group::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::user::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::folder::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::node::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::comparison::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::user_group_membership::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document_user_assignment::Entity).if_not_exists()),
        builder.build(schema.create_table_from_entity(entity::document_group_assignment::Entity).if_not_exists()),
    ];
    for stmt in stmts {
        db.execute(stmt).await?;
    }
    Ok(())
}
