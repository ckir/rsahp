//! Represents the user entity in the database.
//!
//! This module contains the `Model` struct which maps to the `user` table,
//! storing user credentials, flags, and relationships to groups and documents.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a user.
///
/// Contains authentication and authorization details for a user.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    /// The unique identifier for the user.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The chosen username for the user.
    pub username: String,
    /// The hashed password of the user.
    pub password_hash: String,
    /// Flag indicating whether the user has administrative privileges.
    pub is_admin: bool,
    /// Flag indicating if the user's account has been softly deleted.
    pub is_deleted: bool,
}

/// Defines the relationships of the `user` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the user's group memberships.
    #[sea_orm(has_many = "super::user_group_membership::Entity")]
    UserGroupMembership,
    /// Relationship to the document assignments for this user.
    #[sea_orm(has_many = "super::document_user_assignment::Entity")]
    DocumentAssignment,
    /// Relationship to the comparisons evaluated by this user.
    #[sea_orm(has_many = "super::comparison::Entity")]
    Comparison,
}

/// Defines how the `user` entity relates to the `user_group_membership` entity.
impl Related<super::user_group_membership::Entity> for Entity {
    /// Returns the relation definition to `UserGroupMembership`.
    fn to() -> RelationDef {
        // Return the definition of the UserGroupMembership relation
        Relation::UserGroupMembership.def()
    }
}

/// Defines how the `user` entity relates to the `document_user_assignment` entity.
impl Related<super::document_user_assignment::Entity> for Entity {
    /// Returns the relation definition to `DocumentAssignment`.
    fn to() -> RelationDef {
        // Return the definition of the DocumentAssignment relation
        Relation::DocumentAssignment.def()
    }
}

/// Defines how the `user` entity relates to the `comparison` entity.
impl Related<super::comparison::Entity> for Entity {
    /// Returns the relation definition to `Comparison`.
    fn to() -> RelationDef {
        // Return the definition of the Comparison relation
        Relation::Comparison.def()
    }
}

/// Defines the active model behavior for the `user` entity.
impl ActiveModelBehavior for ActiveModel {}
