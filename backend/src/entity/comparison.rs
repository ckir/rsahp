//! Represents the comparison entity in the database.
//!
//! This module contains the `Model` struct which maps to the `comparison` table,
//! along with its relationships to other entities like `Document`, `ParentNode`, and `User`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a comparison.
///
/// A comparison represents an evaluation made by a user between two nodes (e.g., criteria or alternatives)
/// under a specific parent node within a document.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "comparison")]
pub struct Model {
    /// The unique identifier for the comparison.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The identifier of the document this comparison belongs to.
    pub document_id: i32,
    /// Identifies the user filling the evaluation.
    pub respondent_id: i32,
    /// The identifier of the parent node under which the comparison is made.
    pub parent_node_id: i32,
    /// The identifier of the first node being compared (Node A).
    pub node_a_id: i32,
    /// The identifier of the second node being compared (Node B).
    pub node_b_id: i32,
    /// The 1-9 value (or reciprocal 1/9 to 1) representing the relative importance.
    pub saaty_value: f64,
}

/// Defines the relationships of the `comparison` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the `document` entity.
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    /// Relationship to the `node` entity representing the parent node.
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::ParentNodeId",
        to = "super::node::Column::Id"
    )]
    ParentNode,
    /// Relationship to the `user` entity representing the respondent.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RespondentId",
        to = "super::user::Column::Id"
    )]
    User,
}

/// Defines how the `comparison` entity relates to the `document` entity.
impl Related<super::document::Entity> for Entity {
    /// Returns the relation definition to `Document`.
    fn to() -> RelationDef {
        // Return the definition of the Document relation
        Relation::Document.def()
    }
}

/// Defines how the `comparison` entity relates to the `user` entity.
impl Related<super::user::Entity> for Entity {
    /// Returns the relation definition to `User`.
    fn to() -> RelationDef {
        // Return the definition of the User relation
        Relation::User.def()
    }
}

/// Defines the active model behavior for the `comparison` entity.
impl ActiveModelBehavior for ActiveModel {}
