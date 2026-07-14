// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Document user assignment entity representation.
//!
//! Defines the database model and relations for linking users to documents.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a user's assignment to a document.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "document_user_assignment")]
pub struct Model {
    /// The unique identifier of the assignment
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The document being assigned
    pub document_id: i32,
    /// The user being given access
    pub user_id: i32,
}

/// Defines the relationships for the DocumentUserAssignment entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the associated Document entity
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    /// Relationship to the associated User entity
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::document::Entity> for Entity {
    /// Configures the relationship mapping to a Document
    fn to() -> RelationDef {
        // Return the definition for Document relationship
        Relation::Document.def()
    }
}

impl Related<super::user::Entity> for Entity {
    /// Configures the relationship mapping to a User
    fn to() -> RelationDef {
        // Return the definition for User relationship
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
