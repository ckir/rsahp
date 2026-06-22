//! Represents the document entity in the database.
//!
//! This module contains the `Model` struct which maps to the `document` table,
//! representing an AHP project or document, and its relations to users, folders, nodes, and assignments.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a document.
///
/// A document corresponds to a specific AHP (Analytic Hierarchy Process) project.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "document")]
pub struct Model {
    /// The unique identifier for the document.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The name or title of the document.
    pub name: String,
    /// The user ID of the document's owner.
    pub owner_id: i32,
    /// The version number of the document.
    pub version: i32,
    /// The aggregation method used, e.g., "AIJ" or "AIP".
    pub aggregation_method: String,
    /// The optional identifier of the folder containing this document.
    pub folder_id: Option<i32>,
    /// The timestamp when the document was created.
    pub created_at: DateTimeUtc,
}

/// Defines the relationships of the `document` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the `user` entity (the owner of the document).
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Relationship to the `folder` entity where the document resides.
    #[sea_orm(
        belongs_to = "super::folder::Entity",
        from = "Column::FolderId",
        to = "super::folder::Column::Id"
    )]
    Folder,
    /// Relationship to the `node` entities associated with this document.
    #[sea_orm(has_many = "super::node::Entity")]
    Node,
    /// Relationship to the user assignments for this document.
    #[sea_orm(has_many = "super::document_user_assignment::Entity")]
    UserAssignment,
    /// Relationship to the group assignments for this document.
    #[sea_orm(has_many = "super::document_group_assignment::Entity")]
    GroupAssignment,
}

/// Defines how the `document` entity relates to the `user` entity.
impl Related<super::user::Entity> for Entity {
    /// Returns the relation definition to `User`.
    fn to() -> RelationDef {
        // Return the definition of the User relation
        Relation::User.def()
    }
}

/// Defines how the `document` entity relates to the `folder` entity.
impl Related<super::folder::Entity> for Entity {
    /// Returns the relation definition to `Folder`.
    fn to() -> RelationDef {
        // Return the definition of the Folder relation
        Relation::Folder.def()
    }
}

/// Defines how the `document` entity relates to the `node` entity.
impl Related<super::node::Entity> for Entity {
    /// Returns the relation definition to `Node`.
    fn to() -> RelationDef {
        // Return the definition of the Node relation
        Relation::Node.def()
    }
}

/// Defines how the `document` entity relates to the `document_user_assignment` entity.
impl Related<super::document_user_assignment::Entity> for Entity {
    /// Returns the relation definition to `UserAssignment`.
    fn to() -> RelationDef {
        // Return the definition of the UserAssignment relation
        Relation::UserAssignment.def()
    }
}

/// Defines how the `document` entity relates to the `document_group_assignment` entity.
impl Related<super::document_group_assignment::Entity> for Entity {
    /// Returns the relation definition to `GroupAssignment`.
    fn to() -> RelationDef {
        // Return the definition of the GroupAssignment relation
        Relation::GroupAssignment.def()
    }
}

/// Defines the active model behavior for the `document` entity.
impl ActiveModelBehavior for ActiveModel {}
