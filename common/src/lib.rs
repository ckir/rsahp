//! Module lib.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO representing document metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentDto {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub version: i32,
    pub aggregation_method: String,
    pub folder_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// DTO representing a single node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDto {
    pub id: i32,
    pub document_id: i32,
    pub parent_node_id: Option<i32>,
    pub name: String,
    pub node_type: String,
    pub cost: Option<f64>,
}

/// DTO representing a pairwise comparison result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonDto {
    pub id: i32,
    pub document_id: i32,
    pub respondent_id: i32,
    pub parent_node_id: i32,
    pub node_a_id: i32,
    pub node_b_id: i32,
    pub saaty_value: f64,
}

/// A full exported document containing the document metadata and all nodes/comparisons.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportedDocumentDto {
    pub document: DocumentDto,
    pub nodes: Vec<NodeDto>,
    pub comparisons: Vec<ComparisonDto>,
}

/// DTO for creating/updating document metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateDocumentDto {
    pub name: String,
    pub owner_id: i32,
    pub aggregation_method: String,
    pub folder_id: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Documentation for `TreeDto`.
pub struct TreeDto {
    pub folders: Vec<FolderDto>,
    pub documents: Vec<DocumentDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Documentation for `FolderDto`.
pub struct FolderDto {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub parent_folder_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Documentation for `AssignmentDto`.
pub struct AssignmentDto {
    pub id: i32,
    pub name: String,
    pub assigned_at: String, // Kept as string for simplicity unless needed otherwise
    pub is_group: bool,
}
