//! Versioned, immutable database migrations for rsahp.
//!
//! `m0_initial` is the immutable baseline snapshot of the schema that the old
//! `create_table_from_entity` startup loop used to build imperatively. Future
//! schema changes are added as new migrations — never by editing `m0`.

// sea-orm-migration pulls transitive dependencies (heck, syn, webpki-roots,
// windows-sys, etc.) at versions that differ from other workspace crates'
// dependency trees. This is a dependency-graph fact of the sea-orm-migration
// crate itself, not something fixable by editing this crate's source.
#![allow(clippy::multiple_crate_versions)]

pub use sea_orm_migration::prelude::*;

mod m0_initial;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m0_initial::Migration)]
    }
}
