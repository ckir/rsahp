// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Represents the user group entity in the database.
//!
//! This module contains the `Model` struct which maps to the `user_group` table,
//! allowing users to be organized hierarchically into groups.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a user group.
///
/// User groups can be organized in a hierarchy and have members and document assignments.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_group")]
pub struct Model {
    /// The unique identifier for the user group.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The name of the user group.
    pub name: String,
    /// The optional identifier of the parent user group.
    pub parent_id: Option<i32>,
}

/// Defines the relationships of the `user_group` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Self-referential relationship representing the parent user group.
    #[sea_orm(
        belongs_to = "super::user_group::Entity",
        from = "Column::ParentId",
        to = "super::user_group::Column::Id"
    )]
    ParentGroup,
    /// Relationship to the memberships associated with this group.
    #[sea_orm(has_many = "super::user_group_membership::Entity")]
    UserGroupMembership,
    /// Relationship to the document assignments for this group.
    #[sea_orm(has_many = "super::document_group_assignment::Entity")]
    DocumentAssignment,
}

/// Defines how the `user_group` entity relates to the `user_group_membership` entity.
impl Related<super::user_group_membership::Entity> for Entity {
    /// Returns the relation definition to `UserGroupMembership`.
    fn to() -> RelationDef {
        // Return the definition of the UserGroupMembership relation
        Relation::UserGroupMembership.def()
    }
}

/// Defines how the `user_group` entity relates to the `document_group_assignment` entity.
impl Related<super::document_group_assignment::Entity> for Entity {
    /// Returns the relation definition to `DocumentAssignment`.
    fn to() -> RelationDef {
        // Return the definition of the DocumentAssignment relation
        Relation::DocumentAssignment.def()
    }
}

/// Defines the active model behavior for the `user_group` entity.
impl ActiveModelBehavior for ActiveModel {}
