//! Represents the folder entity in the database.
//!
//! This module contains the `Model` struct which maps to the `folder` table,
//! representing a container for documents or other folders, and its relationships.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a folder.
///
/// Folders can be used to organize documents hierarchically.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "folder")]
pub struct Model {
    /// The unique identifier for the folder.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The name of the folder.
    pub name: String,
    /// The identifier of the user who owns the folder.
    pub owner_id: i32,
    /// The optional identifier of the parent folder, establishing a hierarchy.
    pub parent_folder_id: Option<i32>,
}

/// Defines the relationships of the `folder` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the `user` entity (the owner of the folder).
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Self-referential relationship representing the parent folder.
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentFolderId",
        to = "Column::Id"
    )]
    ParentFolder,
    /// Relationship to the `document` entities contained in this folder.
    #[sea_orm(has_many = "super::document::Entity")]
    Document,
}

/// Defines how the `folder` entity relates to the `user` entity.
impl Related<super::user::Entity> for Entity {
    /// Returns the relation definition to `User`.
    fn to() -> RelationDef {
        // Return the definition of the User relation
        Relation::User.def()
    }
}

/// Defines how the `folder` entity relates to the `document` entity.
impl Related<super::document::Entity> for Entity {
    /// Returns the relation definition to `Document`.
    fn to() -> RelationDef {
        // Return the definition of the Document relation
        Relation::Document.def()
    }
}

/// Defines the active model behavior for the `folder` entity.
impl ActiveModelBehavior for ActiveModel {}
