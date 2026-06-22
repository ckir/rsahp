//! Contains all the database entity models and their relationships.
//!
//! This module exposes submodules for each database table, defining
//! structs and enumerations required by SeaORM.

/// The comparison entity module.
pub mod comparison;
/// The document entity module.
pub mod document;
/// The document group assignment entity module.
pub mod document_group_assignment;
/// The document user assignment entity module.
pub mod document_user_assignment;
/// The folder entity module.
pub mod folder;
/// The node entity module.
pub mod node;
/// The user entity module.
pub mod user;
/// The user group entity module.
pub mod user_group;
/// The user group membership entity module.
pub mod user_group_membership;
