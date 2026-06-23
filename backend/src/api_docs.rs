//! Document API endpoints.
//!
//! This module provides routes and handlers for managing documents,
//! their node hierarchies, comparisons: comparisons.into_iter().map(Into::into).collect(), folders, and export/import functionality.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};

use sea_orm::Set;

impl From<document::Model> for common::DocumentDto {
    fn from(m: document::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            owner_id: m.owner_id,
            version: m.version,
            aggregation_method: m.aggregation_method,
            folder_id: m.folder_id,
            created_at: m.created_at,
        }
    }
}

impl From<node::Model> for common::NodeDto {
    fn from(m: node::Model) -> Self {
        Self {
            id: m.id,
            document_id: m.document_id,
            parent_node_id: m.parent_node_id,
            name: m.name,
            node_type: m.node_type,
            cost: m.cost,
        }
    }
}

impl From<comparison::Model> for common::ComparisonDto {
    fn from(m: comparison::Model) -> Self {
        Self {
            id: m.id,
            document_id: m.document_id,
            respondent_id: m.respondent_id,
            parent_node_id: m.parent_node_id,
            node_a_id: m.node_a_id,
            node_b_id: m.node_b_id,
            saaty_value: m.saaty_value,
        }
    }
}

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::api_auth::Claims;
use crate::entity::{
    comparison, document, document_group_assignment, document_user_assignment, folder, node,
    user_group_membership,
};

/// Returns the router for document-related endpoints.
pub fn router() -> Router<DatabaseConnection> {
    // Configure all document routes
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/tree", get(get_tree))
        .route(
            "/{id}",
            get(get_document)
                .put(update_document)
                .delete(delete_document),
        )
        .route("/{id}/full", post(save_full_document))
        .route("/{id}/duplicate", post(duplicate_document))
        .route("/{id}/move", post(move_document))
        .route(
            "/{id}/assignments",
            get(get_document_assignments).post(set_document_assignments),
        )
        .route("/{id}/nodes", get(list_nodes).post(create_node))
        .route(
            "/{id}/nodes/{node_id}",
            delete(delete_node).put(update_node),
        )
        .route(
            "/{id}/comparisons",
            get(list_comparisons).post(create_comparison),
        )
        .route("/folders", get(list_folders).post(create_folder))
        .route("/folders/{id}", post(update_folder).delete(delete_folder))
        // Export/Import endpoints
        .route("/{id}/export", get(export_document))
        .route("/import", post(import_document))
}

/// Data transfer object for document creation and updates.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DocumentDto {
    /// Document name
    pub name: String,
    /// ID of the user who owns this document
    pub owner_id: i32,
    /// The aggregation method to be used
    pub aggregation_method: String,
    /// Optional parent folder ID
    pub folder_id: Option<i32>,
}

/// Retrieves a list of all documents accessible by the current user.
pub async fn list_documents(
    claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<document::Model>>, (StatusCode, String)> {
    // Fetch documents using the claims' subject (user ID)
    let docs = fetch_allowed_documents(&db, claims.sub).await?;
    // Return documents wrapped in JSON
    Ok(Json(docs))
}

/// Creates a new document.
pub async fn create_document(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Parse the JSON payload
    let payload: common::CreateDocumentDto = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Prepare the active model for insertion
    let doc = document::ActiveModel {
        name: Set(payload.name),
        owner_id: Set(payload.owner_id),
        version: Set(1),
        aggregation_method: Set(payload.aggregation_method),
        folder_id: Set(payload.folder_id),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    // Insert the new document into the database
    let result = doc
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the inserted model
    Ok(Json(result))
}

/// Gets a specific document by its ID.
pub async fn get_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Query the document by ID
    let doc = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;

    // Return the found document
    Ok(Json(doc))
}

/// Updates an existing document's properties.
pub async fn update_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<DocumentDto>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Look up the document and convert to an ActiveModel for editing
    let mut doc: document::ActiveModel = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?
        .into();

    // Update the relevant fields
    doc.name = Set(payload.name);
    doc.aggregation_method = Set(payload.aggregation_method);
    // Version is intentionally NOT incremented here; it is incremented only upon duplication.

    // Commit changes to database
    let updated = doc
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return updated data
    Ok(Json(updated))
}

/// Deletes a document by its ID.
pub async fn delete_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<String, (StatusCode, String)> {
    // Execute deletion based on ID
    let res = document::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check if any row was affected to confirm deletion
    if res.rows_affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Document not found".to_string()));
    }

    // Return success message
    Ok("Deleted".to_string())
}

/// Data transfer object for document assignments (users and groups).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DocumentAssignmentsDto {
    /// Users directly assigned to the document
    pub user_ids: Vec<i32>,
    /// Groups assigned to the document
    pub group_ids: Vec<i32>,
}

/// Retrieves all user and group assignments for a specific document.
pub async fn get_document_assignments(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<DocumentAssignmentsDto>, (StatusCode, String)> {
    // Fetch all user assignments for this document
    let users = document_user_assignment::Entity::find()
        .filter(document_user_assignment::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch all group assignments for this document
    let groups = document_group_assignment::Entity::find()
        .filter(document_group_assignment::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Package IDs into a DTO and return
    Ok(Json(DocumentAssignmentsDto {
        user_ids: users.into_iter().map(|u| u.user_id).collect(),
        group_ids: groups.into_iter().map(|g| g.group_id).collect(),
    }))
}

/// Replaces the user and group assignments for a specific document.
pub async fn set_document_assignments(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<DocumentAssignmentsDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Delete all existing user assignments
    let _ = document_user_assignment::Entity::delete_many()
        .filter(document_user_assignment::Column::DocumentId.eq(id))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Delete all existing group assignments
    let _ = document_group_assignment::Entity::delete_many()
        .filter(document_group_assignment::Column::DocumentId.eq(id))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create new user assignments
    for uid in payload.user_ids {
        let membership = document_user_assignment::ActiveModel {
            document_id: Set(id),
            user_id: Set(uid),
            ..Default::default()
        };
        membership
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Create new group assignments
    for gid in payload.group_ids {
        let membership = document_group_assignment::ActiveModel {
            document_id: Set(id),
            group_id: Set(gid),
            ..Default::default()
        };
        membership
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Return success
    Ok(StatusCode::OK)
}

/// Data transfer object for node creation and updates.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NodeDto {
    /// Optional parent node ID for hierarchical nodes
    pub parent_node_id: Option<i32>,
    /// Name of the node
    pub name: String,
    /// Node type classification: "Goal", "Criteria", "Alternative"
    pub node_type: String,
    /// Cost parameter for AHP
    pub cost: Option<f64>,
}

/// Retrieves all nodes associated with a document.
pub async fn list_nodes(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<node::Model>>, (StatusCode, String)> {
    // Filter nodes by the specific document ID
    let nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the list of nodes
    Ok(Json(nodes))
}

/// Creates a new node within a document.
pub async fn create_node(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<NodeDto>,
) -> Result<Json<node::Model>, (StatusCode, String)> {
    // Formulate a new ActiveModel for the node
    let new_node = node::ActiveModel {
        document_id: Set(id),
        parent_node_id: Set(payload.parent_node_id),
        name: Set(payload.name),
        node_type: Set(payload.node_type),
        cost: Set(payload.cost),
        ..Default::default()
    };

    // Insert node to DB
    let result = new_node
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return created node
    Ok(Json(result))
}

/// Updates an existing node.
pub async fn update_node(
    State(db): State<DatabaseConnection>,
    Path((_doc_id, node_id)): Path<(i32, i32)>,
    Json(payload): Json<NodeDto>,
) -> Result<Json<node::Model>, (StatusCode, String)> {
    // Find the node by its primary ID
    let mut node_am: node::ActiveModel = node::Entity::find_by_id(node_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Node not found".to_string()))?
        .into();

    // Reassign new data from payload
    node_am.name = Set(payload.name);
    node_am.parent_node_id = Set(payload.parent_node_id);
    node_am.node_type = Set(payload.node_type);
    node_am.cost = Set(payload.cost);

    // Save changes
    let updated = node_am
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return updated node
    Ok(Json(updated))
}

/// Deletes a specific node from a document.
pub async fn delete_node(
    State(db): State<DatabaseConnection>,
    Path((_id, node_id)): Path<(i32, i32)>,
) -> Result<String, (StatusCode, String)> {
    // Delete the target node
    let res = node::Entity::delete_by_id(node_id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Validate if deletion actually found a row
    if res.rows_affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Node not found".to_string()));
    }

    // Send confirmation
    Ok("Deleted".to_string())
}

/// Data transfer object for comparison creations and updates.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ComparisonDto {
    /// ID of the user giving the rating
    pub respondent_id: i32,
    /// Parent node ID (context for comparison)
    pub parent_node_id: i32,
    /// First node in comparison
    pub node_a_id: i32,
    /// Second node in comparison
    pub node_b_id: i32,
    /// The rating scale value
    pub saaty_value: f64,
}

/// Retrieves all comparisons within a given document.
pub async fn list_comparisons(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<comparison::Model>>, (StatusCode, String)> {
    // Filter comparisons associated with this document ID
    let comps = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the retrieved list
    Ok(Json(comps))
}

/// Creates a new comparison entry.
pub async fn create_comparison(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<ComparisonDto>,
) -> Result<Json<comparison::Model>, (StatusCode, String)> {
    // Map DTO contents into an ActiveModel representation
    let new_comp = comparison::ActiveModel {
        document_id: Set(id),
        respondent_id: Set(payload.respondent_id),
        parent_node_id: Set(payload.parent_node_id),
        node_a_id: Set(payload.node_a_id),
        node_b_id: Set(payload.node_b_id),
        saaty_value: Set(payload.saaty_value),
        ..Default::default()
    };

    // Insert into DB
    let result = new_comp
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return result
    Ok(Json(result))
}

/// Data structure representing a fully exported document tree.
/* pub struct common::ExportedDocumentDto {
    /// Core document record
    pub document: document::Model,
    /// Flat list of nodes comprising the structure
    pub nodes: Vec<node::Model>,
    /// Set of evaluation comparisons
    pub comparisons: Vec<comparison::Model>,
} */

/// Exports a document to JSON including all nested nodes and comparisons.
pub async fn export_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<common::ExportedDocumentDto>, (StatusCode, String)> {
    // Find the main document object
    let doc = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;

    // Load nodes pertaining to it
    let nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Load comparison data associated with it
    let comparisons = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Collect into common::ExportedDocumentDto and output
    Ok(Json(common::ExportedDocumentDto {
        document: doc.into(),
        nodes: nodes.into_iter().map(Into::into).collect(),
        comparisons: comparisons.into_iter().map(Into::into).collect(),
    }))
}

/// Imports a full document payload, preserving hierarchical relationships.
pub async fn import_document(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<common::ExportedDocumentDto>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Start constructing the imported document
    let mut doc_am = document::ActiveModel {
        name: Set(payload.document.name),
        owner_id: Set(payload.document.owner_id),
        version: Set(payload.document.version),
        aggregation_method: Set(payload.document.aggregation_method),
        folder_id: Set(payload.document.folder_id),
        created_at: Set(payload.document.created_at),
        ..Default::default()
    };
    // Do not preserve the primary key
    doc_am.id = sea_orm::ActiveValue::NotSet;

    // Write new document row
    let new_doc = doc_am
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Prepare mappings to adjust IDs referencing nodes
    let mut node_id_map = std::collections::HashMap::new();
    let mut nodes_to_insert = payload.nodes;

    // We must resolve parent dependencies correctly without foreign key errors.
    // Loop until nodes list is empty.
    while !nodes_to_insert.is_empty() {
        let mut inserted_any = false;
        let mut remaining = Vec::new();

        // Check which nodes have valid parent contexts mapped
        for node in nodes_to_insert {
            let old_id = node.id;
            let can_insert = match node.parent_node_id {
                Some(pid) => node_id_map.contains_key(&pid),
                None => true,
            };

            if can_insert {
                let pid_opt = node.parent_node_id;
                let mut am = node::ActiveModel {
                    document_id: Set(node.document_id),
                    parent_node_id: Set(node.parent_node_id),
                    name: Set(node.name),
                    node_type: Set(node.node_type),
                    cost: Set(node.cost),
                    ..Default::default()
                };

                // Clear the original ID, set the new document ID reference
                am.id = sea_orm::ActiveValue::NotSet;
                am.document_id = Set(new_doc.id);

                // Resolve any parent link to its new inserted ID
                if let Some(pid) = pid_opt
                    && let Some(&new_pid) = node_id_map.get(&pid)
                {
                    am.parent_node_id = Set(Some(new_pid));
                }

                // Insert the node
                let inserted_node = am
                    .insert(&db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                // Record the mapping
                node_id_map.insert(old_id, inserted_node.id);
                inserted_any = true;
            } else {
                // If dependencies are missing, keep to try again
                remaining.push(node);
            }
        }

        // If cycle or broken references exist, halt import
        if !inserted_any {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid node hierarchy in import".to_string(),
            ));
        }

        // Loop again with leftover
        nodes_to_insert = remaining;
    }

    // Follow up inserting comparisons mapping references
    for comp in payload.comparisons {
        let mut am = comparison::ActiveModel {
            document_id: Set(comp.document_id),
            respondent_id: Set(comp.respondent_id),
            parent_node_id: Set(comp.parent_node_id),
            node_a_id: Set(comp.node_a_id),
            node_b_id: Set(comp.node_b_id),
            saaty_value: Set(comp.saaty_value),
            ..Default::default()
        };

        // Wipe original IDs and map to new document ID
        am.id = sea_orm::ActiveValue::NotSet;
        am.document_id = Set(new_doc.id);

        // Resolve reference or fail if undefined
        if let Some(&new_pid) = node_id_map.get(am.parent_node_id.as_ref()) {
            am.parent_node_id = Set(new_pid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison parent_node_id".to_string(),
            ));
        }

        // Resolve reference or fail if undefined
        if let Some(&new_aid) = node_id_map.get(am.node_a_id.as_ref()) {
            am.node_a_id = Set(new_aid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison node_a_id".to_string(),
            ));
        }

        // Resolve reference or fail if undefined
        if let Some(&new_bid) = node_id_map.get(am.node_b_id.as_ref()) {
            am.node_b_id = Set(new_bid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison node_b_id".to_string(),
            ));
        }

        // Apply inserts
        am.insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Success response with the base document object
    Ok(Json(new_doc))
}

/// Saves full document data, overwriting all existing nodes and comparisons.
pub async fn save_full_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Parse the full payload
    let payload: common::ExportedDocumentDto = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Look for existing document record
    let doc_opt = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create an active model tracking updates or creation
    let mut doc: document::ActiveModel = if let Some(existing) = doc_opt.clone() {
        existing.into()
    } else {
        let mut new_doc = document::ActiveModel {
            owner_id: Set(payload.document.owner_id),
            version: Set(0),
            created_at: Set(chrono::Utc::now()),
            ..Default::default()
        };
        new_doc.id = Set(id);
        new_doc
    };

    // Propagate fields
    doc.name = Set(payload.document.name);
    doc.aggregation_method = Set(payload.document.aggregation_method);
    doc.version = Set(payload.document.version);

    // Apply DB update or insert if lacking
    let updated_doc = if doc_opt.is_some() {
        doc.update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        doc.insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    // Delete existing comparisons to respect foreign key cascades on replacement
    let _ = comparison::Entity::delete_many()
        .filter(comparison::Column::DocumentId.eq(id))
        .exec(&db)
        .await;

    // Delete existing nodes
    let _ = node::Entity::delete_many()
        .filter(node::Column::DocumentId.eq(id))
        .exec(&db)
        .await;

    // Track ID translations
    let mut node_id_map = std::collections::HashMap::new();
    let mut nodes_to_insert = payload.nodes;

    // Loop through nodes dynamically mapping tree logic
    while !nodes_to_insert.is_empty() {
        let mut inserted_any = false;
        let mut remaining = Vec::new();

        for node in nodes_to_insert {
            let old_id = node.id;
            // A node is ready to insert if it's the root, or if its parent is already processed
            let can_insert = match node.parent_node_id {
                Some(pid) => node_id_map.contains_key(&pid),
                None => true,
            };

            if can_insert {
                let pid_opt = node.parent_node_id;
                let mut am = node::ActiveModel {
                    document_id: Set(node.document_id),
                    parent_node_id: Set(node.parent_node_id),
                    name: Set(node.name),
                    node_type: Set(node.node_type),
                    cost: Set(node.cost),
                    ..Default::default()
                };
                am.id = sea_orm::ActiveValue::NotSet;
                am.document_id = Set(id);

                // Tie properly to inserted parent context
                if let Some(pid) = pid_opt
                    && let Some(&new_pid) = node_id_map.get(&pid)
                {
                    am.parent_node_id = Set(Some(new_pid));
                }

                // Insert into DB
                let inserted_node = am
                    .insert(&db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                // Record translation
                node_id_map.insert(old_id, inserted_node.id);
                inserted_any = true;
            } else {
                remaining.push(node);
            }
        }

        // Protect from cyclic relations
        if !inserted_any {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid node hierarchy".to_string(),
            ));
        }
        nodes_to_insert = remaining;
    }

    // Run over comparisons using constructed translations
    for comp in payload.comparisons {
        let mut am = comparison::ActiveModel {
            document_id: Set(comp.document_id),
            respondent_id: Set(comp.respondent_id),
            parent_node_id: Set(comp.parent_node_id),
            node_a_id: Set(comp.node_a_id),
            node_b_id: Set(comp.node_b_id),
            saaty_value: Set(comp.saaty_value),
            ..Default::default()
        };
        am.id = sea_orm::ActiveValue::NotSet;
        am.document_id = Set(id);

        if let Some(&new_pid) = node_id_map.get(am.parent_node_id.as_ref()) {
            am.parent_node_id = Set(new_pid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison parent_node_id".to_string(),
            ));
        }

        if let Some(&new_aid) = node_id_map.get(am.node_a_id.as_ref()) {
            am.node_a_id = Set(new_aid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison node_a_id".to_string(),
            ));
        }

        if let Some(&new_bid) = node_id_map.get(am.node_b_id.as_ref()) {
            am.node_b_id = Set(new_bid);
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid comparison node_b_id".to_string(),
            ));
        }

        am.insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Done replacing state
    Ok(Json(updated_doc))
}

/// Duplicates a document, including its entire node and comparison structure.
pub async fn duplicate_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Initiate transaction for data consistency
    let txn = db
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 1. Fetch original document inside transaction
    let orig_doc = document::Entity::find_by_id(id)
        .one(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;

    // 2. Compute new version number
    let new_version = orig_doc.version + 1;

    // 3. Strip existing (vX) from the name if present, to avoid nested version titles
    let mut base_name = orig_doc.name.clone();
    if let Some(idx) = base_name.rfind(" (v")
        && base_name.ends_with(')')
    {
        base_name.truncate(idx);
    }

    // Create the updated name with incremented version tracking
    let new_doc_name = format!("{} (v{})", base_name, new_version);

    // Construct new duplicate document entry
    let new_doc = document::ActiveModel {
        name: Set(new_doc_name),
        owner_id: Set(orig_doc.owner_id),
        version: Set(new_version),
        aggregation_method: Set(orig_doc.aggregation_method.clone()),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    // Perform database insertion and track primary key
    let inserted_doc = new_doc
        .insert(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let new_doc_id = inserted_doc.id;

    // 4. Read original nodes collection
    let orig_nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut node_id_map = std::collections::HashMap::new();

    // Iterate duplicating nodes but leaving parent IDs empty temporarily
    for n in &orig_nodes {
        let new_node = node::ActiveModel {
            document_id: Set(new_doc_id),
            name: Set(n.name.clone()),
            node_type: Set(n.node_type.clone()),
            parent_node_id: Set(None), // We will fix parents in a second pass to avoid FK issues
            ..Default::default()
        };
        // Commit initial row
        let inserted = new_node
            .insert(&txn)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Map old node ID to the new duplicate ID
        node_id_map.insert(n.id, inserted.id);
    }

    // 5. Second pass: update parent IDs to connect tree
    for n in &orig_nodes {
        if let Some(old_parent) = n.parent_node_id
            && let (Some(&new_id), Some(&new_parent)) =
                (node_id_map.get(&n.id), node_id_map.get(&old_parent))
        {
            // Fetch the new node to modify it
            let mut update_node: node::ActiveModel = node::Entity::find_by_id(new_id)
                .one(&txn)
                .await
                .unwrap()
                .unwrap()
                .into();

            // Adjust parent binding
            update_node.parent_node_id = Set(Some(new_parent));

            // Execute modification
            update_node
                .update(&txn)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    // 6. Fetch original comparisons
    let orig_comps = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Reconstruct valid comparison pointers relative to new node space
    for c in orig_comps {
        let new_parent_id = node_id_map.get(&c.parent_node_id).copied().unwrap_or(0);
        let new_node_a = node_id_map.get(&c.node_a_id).copied().unwrap_or(0);
        let new_node_b = node_id_map.get(&c.node_b_id).copied().unwrap_or(0);

        // Fill out model struct
        let new_comp = comparison::ActiveModel {
            document_id: Set(new_doc_id),
            respondent_id: Set(c.respondent_id),
            parent_node_id: Set(new_parent_id),
            node_a_id: Set(new_node_a),
            node_b_id: Set(new_node_b),
            saaty_value: Set(c.saaty_value),
            ..Default::default()
        };

        // Finalise entry recording
        new_comp
            .insert(&txn)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Close and apply transaction state updates
    txn.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Respond back to caller with copied structure reference
    Ok(Json(inserted_doc))
}

/// Data transfer object for folder creation and updates.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FolderDto {
    /// Folder name
    pub name: String,
    /// Identifier indicating owner context
    pub owner_id: i32,
    /// ID pointing to nesting context (if this exists within another folder)
    pub parent_folder_id: Option<i32>,
}

/// Retrieves a list of all folders.
pub async fn list_folders(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<folder::Model>>, (StatusCode, String)> {
    // Run an unfiltered query returning all folder collections
    let folders = folder::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Send standard JSON response
    Ok(Json(folders))
}

/// Creates a new folder.
pub async fn create_folder(
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<folder::Model>, (StatusCode, String)> {
    // Load schema contents handling input errors
    let payload: FolderDto = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Create a new representation based on values provided
    let f = folder::ActiveModel {
        name: Set(payload.name),
        owner_id: Set(payload.owner_id),
        parent_folder_id: Set(payload.parent_folder_id),
        ..Default::default()
    };

    // Store in DB context mapping back to client
    let inserted = f
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inserted))
}

/// Updates an existing folder.
pub async fn update_folder(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<folder::Model>, (StatusCode, String)> {
    // Resolve structure
    let payload: FolderDto = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Access specific target database item
    let mut f: folder::ActiveModel = folder::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Folder not found".to_string()))?
        .into();

    // Adjust specific features based on payload
    f.name = Set(payload.name);
    f.parent_folder_id = Set(payload.parent_folder_id);

    // Apply DB write saving change execution output
    let updated = f
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Send final entity
    Ok(Json(updated))
}

/// Deletes a folder by its ID.
pub async fn delete_folder(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Delete entry discarding outcome except checking code logic validation
    let _ = folder::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Standard response returning no content upon execution
    Ok(StatusCode::NO_CONTENT)
}

/// Data transfer object for moving a document to a different folder.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MoveDocumentDto {
    /// Desired folder ID or None to move to root level
    pub folder_id: Option<i32>,
}

/// Moves a document to a different folder.
pub async fn move_document(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    // Cast and parse target parameter
    let payload: MoveDocumentDto = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // Check specific target
    let mut doc: document::ActiveModel = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Document not found".to_string()))?
        .into();

    // Point field definition to intended placement reference
    doc.folder_id = Set(payload.folder_id);

    // Lock update call to target storage
    let updated = doc
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Render feedback
    Ok(Json(updated))
}

/// Data transfer object containing the full folder and document tree.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TreeDto {
    /// Comprehensive set of registered folders
    pub folders: Vec<folder::Model>,
    /// Available document listings
    pub documents: Vec<document::Model>,
}

/// Retrieves the complete folder and document tree accessible by the user.
pub async fn get_tree(
    claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<TreeDto>, (StatusCode, String)> {
    // Retrieve root elements mapping structural elements
    let folders = folder::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Execute filter determining accessible user contents
    let documents = fetch_allowed_documents(&db, claims.sub).await?;

    // Serve DTO instance tying context sets together
    Ok(Json(TreeDto { folders, documents }))
}

/// Helper function to retrieve all documents a user has permission to view.
async fn fetch_allowed_documents(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<document::Model>, (StatusCode, String)> {
    // Collect direct document associations where they denote ownership
    let owned_docs = document::Entity::find()
        .filter(document::Column::OwnerId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Search permissions tied directly to individual user assignments
    let user_assignments = document_user_assignment::Entity::find()
        .filter(document_user_assignment::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Discover related structural contexts connected specifically through group bindings
    let memberships = user_group_membership::Entity::find()
        .filter(user_group_membership::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Remap membership references to exact context identifiers
    let group_ids: Vec<i32> = memberships.into_iter().map(|m| m.group_id).collect();

    // Map out assignments pointing specifically to the groups determined earlier
    let group_assignments = if !group_ids.is_empty() {
        document_group_assignment::Entity::find()
            .filter(document_group_assignment::Column::GroupId.is_in(group_ids))
            .all(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        vec![]
    };

    // Combine access listings together to define a complete accessible identity set
    let mut doc_ids: Vec<i32> = owned_docs.iter().map(|d| d.id).collect();

    // Add specifically bound context
    doc_ids.extend(user_assignments.into_iter().map(|a| a.document_id));

    // Incorporate group bound documents to accessible collection
    doc_ids.extend(group_assignments.into_iter().map(|a| a.document_id));

    // Ensure all context IDs remain sequential and avoid duplications to preserve processing
    doc_ids.sort();
    doc_ids.dedup();

    // End function fast indicating lacking matching output conditions entirely
    if doc_ids.is_empty() {
        return Ok(vec![]);
    }

    // Access raw documents using aggregated allowed listings to gather resulting array mappings
    let docs = document::Entity::find()
        .filter(document::Column::Id.is_in(doc_ids))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Conclude internal filtering execution responding accessible document output representations
    Ok(docs)
}
