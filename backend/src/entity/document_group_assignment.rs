// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Document group assignment entity representation.
//!
//! Defines the database model and relations for linking groups to documents.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a group's assignment to a document.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "document_group_assignment")]
pub struct Model {
    /// The unique identifier of the assignment
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The document being assigned
    pub document_id: i32,
    /// The group being given access
    pub group_id: i32,
}

/// Defines the relationships for the DocumentGroupAssignment entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the associated Document entity
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    /// Relationship to the associated UserGroup entity
    #[sea_orm(
        belongs_to = "super::user_group::Entity",
        from = "Column::GroupId",
        to = "super::user_group::Column::Id"
    )]
    UserGroup,
}

impl Related<super::document::Entity> for Entity {
    /// Configures the relationship mapping to a Document
    fn to() -> RelationDef {
        // Return the definition for Document relationship
        Relation::Document.def()
    }
}

impl Related<super::user_group::Entity> for Entity {
    /// Configures the relationship mapping to a UserGroup
    fn to() -> RelationDef {
        // Return the definition for UserGroup relationship
        Relation::UserGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
