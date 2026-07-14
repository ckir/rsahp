// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Represents the node entity in the database.
//!
//! This module contains the `Model` struct which maps to the `node` table.
//! A node typically represents a Goal, Criteria, or Alternative in the AHP hierarchy.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a node in the AHP hierarchy.
///
/// Nodes are part of a document and can form a tree structure via a parent node reference.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "node")]
pub struct Model {
    /// The unique identifier for the node.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The identifier of the document this node belongs to.
    pub document_id: i32,
    /// The optional identifier of the parent node.
    pub parent_node_id: Option<i32>,
    /// The name or label of the node.
    pub name: String,
    /// The type of the node, e.g., "Goal", "Criteria", or "Alternative".
    pub node_type: String,
    /// An optional cost value associated with the node.
    pub cost: Option<f64>,
}

/// Defines the relationships of the `node` entity to other entities.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relationship to the `document` entity this node is part of.
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    /// Self-referential relationship representing the parent node in the hierarchy.
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentNodeId",
        to = "Column::Id"
    )]
    ParentNode,
}

/// Defines how the `node` entity relates to the `document` entity.
impl Related<super::document::Entity> for Entity {
    /// Returns the relation definition to `Document`.
    fn to() -> RelationDef {
        // Return the definition of the Document relation
        Relation::Document.def()
    }
}

/// Defines the active model behavior for the `node` entity.
impl ActiveModelBehavior for ActiveModel {}
