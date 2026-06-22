use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put, delete},
    Json, Router,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::entity::{document, node, comparison, folder};

pub fn router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/tree", get(get_tree))
        .route("/{id}", get(get_document).put(update_document).delete(delete_document))
        .route("/{id}/full", post(save_full_document))
        .route("/{id}/duplicate", post(duplicate_document))
        .route("/{id}/move", post(move_document))
        .route("/{id}/nodes", get(list_nodes).post(create_node))
        .route("/{id}/nodes/{node_id}", delete(delete_node))
        .route("/{id}/comparisons", get(list_comparisons).post(create_comparison))
        .route("/folders", get(list_folders).post(create_folder))
        .route("/folders/{id}", post(update_folder).delete(delete_folder))
        // Export/Import endpoints
        .route("/{id}/export", get(export_document))
        .route("/import", post(import_document))
}

#[derive(Serialize, Deserialize)]
pub struct DocumentDto {
    pub name: String,
    pub owner_id: i32,
    pub aggregation_method: String,
    pub folder_id: Option<i32>,
}

// 1. List Documents
pub async fn list_documents(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<document::Model>>, (StatusCode, String)> {
    let docs = document::Entity::find().all(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(docs))
}

// 2. Create Document
pub async fn create_document(
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let payload: DocumentDto = serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    
    let doc = document::ActiveModel {
        name: Set(payload.name),
        owner_id: Set(payload.owner_id),
        version: Set(1),
        aggregation_method: Set(payload.aggregation_method),
        folder_id: Set(payload.folder_id),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    let result = doc.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(result))
}

// 3. Get Document
async fn get_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let doc = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;
    Ok(Json(doc))
}

// 4. Update Document
pub async fn update_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<DocumentDto>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let mut doc: document::ActiveModel = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?
        .into();

    doc.name = Set(payload.name);
    doc.aggregation_method = Set(payload.aggregation_method);
    // Version is intentionally NOT incremented here; it is incremented only upon duplication.

    let updated = doc.update(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updated))
}

// 5. Delete Document
async fn delete_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<String, (StatusCode, String)> {
    let res = document::Entity::delete_by_id(id).exec(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if res.rows_affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Document not found".to_string()));
    }
    Ok("Deleted".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct NodeDto {
    pub parent_node_id: Option<i32>,
    pub name: String,
    pub node_type: String, // "Goal", "Criteria", "Alternative"
}

async fn list_nodes(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<node::Model>>, (StatusCode, String)> {
    let nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(nodes))
}

async fn create_node(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<NodeDto>,
) -> Result<Json<node::Model>, (StatusCode, String)> {
    let new_node = node::ActiveModel {
        document_id: Set(id),
        parent_node_id: Set(payload.parent_node_id),
        name: Set(payload.name),
        node_type: Set(payload.node_type),
        ..Default::default()
    };
    let result = new_node.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(result))
}

async fn delete_node(
    State(db): State<DatabaseConnection>,
    Path((_id, node_id)): Path<(i32, i32)>,
) -> Result<String, (StatusCode, String)> {
    let res = node::Entity::delete_by_id(node_id).exec(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if res.rows_affected == 0 {
        return Err((StatusCode::NOT_FOUND, "Node not found".to_string()));
    }
    Ok("Deleted".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct ComparisonDto {
    pub respondent_email: String,
    pub parent_node_id: i32,
    pub node_a_id: i32,
    pub node_b_id: i32,
    pub saaty_value: f64,
}

async fn list_comparisons(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<comparison::Model>>, (StatusCode, String)> {
    let comps = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(comps))
}

async fn create_comparison(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<ComparisonDto>,
) -> Result<Json<comparison::Model>, (StatusCode, String)> {
    let new_comp = comparison::ActiveModel {
        document_id: Set(id),
        respondent_email: Set(payload.respondent_email),
        parent_node_id: Set(payload.parent_node_id),
        node_a_id: Set(payload.node_a_id),
        node_b_id: Set(payload.node_b_id),
        saaty_value: Set(payload.saaty_value),
        ..Default::default()
    };
    let result = new_comp.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(result))
}

// --- JSON Export/Import Data Structures ---
#[derive(Serialize, Deserialize)]
pub struct ExportedDocument {
    pub document: document::Model,
    pub nodes: Vec<node::Model>,
    pub comparisons: Vec<comparison::Model>,
}

// 6. Export Document to JSON
async fn export_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<ExportedDocument>, (StatusCode, String)> {
    let doc = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;

    let nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comparisons = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ExportedDocument {
        document: doc,
        nodes,
        comparisons,
    }))
}

// 7. Import Document from JSON
async fn import_document(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<ExportedDocument>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let mut doc_am = payload.document.into_active_model();
    doc_am.id = sea_orm::ActiveValue::NotSet; 
    let new_doc = doc_am.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut node_id_map = std::collections::HashMap::new();
    let mut nodes_to_insert = payload.nodes;
    
    while !nodes_to_insert.is_empty() {
        let mut inserted_any = false;
        let mut remaining = Vec::new();

        for node in nodes_to_insert {
            let old_id = node.id;
            let can_insert = match node.parent_node_id {
                Some(pid) => node_id_map.contains_key(&pid),
                None => true,
            };

            if can_insert {
                let pid_opt = node.parent_node_id;
                let mut am = node.into_active_model();
                am.id = sea_orm::ActiveValue::NotSet;
                am.document_id = Set(new_doc.id);
                if let Some(pid) = pid_opt {
                    if let Some(&new_pid) = node_id_map.get(&pid) {
                        am.parent_node_id = Set(Some(new_pid));
                    }
                }
                
                let inserted_node = am.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                node_id_map.insert(old_id, inserted_node.id);
                inserted_any = true;
            } else {
                remaining.push(node);
            }
        }

        if !inserted_any {
            return Err((StatusCode::BAD_REQUEST, "Invalid node hierarchy in import".to_string()));
        }
        nodes_to_insert = remaining;
    }

    for comp in payload.comparisons {
        let mut am = comp.into_active_model();
        am.id = sea_orm::ActiveValue::NotSet;
        am.document_id = Set(new_doc.id);
        
        if let Some(&new_pid) = node_id_map.get(am.parent_node_id.as_ref()) {
            am.parent_node_id = Set(new_pid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison parent_node_id".to_string()));
        }

        if let Some(&new_aid) = node_id_map.get(am.node_a_id.as_ref()) {
            am.node_a_id = Set(new_aid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison node_a_id".to_string()));
        }

        if let Some(&new_bid) = node_id_map.get(am.node_b_id.as_ref()) {
            am.node_b_id = Set(new_bid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison node_b_id".to_string()));
        }

        am.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(new_doc))
}

// 8. Save Full Document (Overwrites nodes and comparisons)
async fn save_full_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let payload: ExportedDocument = serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    
    let doc_opt = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

    doc.name = Set(payload.document.name);
    doc.aggregation_method = Set(payload.document.aggregation_method);
    doc.version = Set(payload.document.version);
    
    let updated_doc = if doc_opt.is_some() {
        doc.update(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        doc.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    // Delete existing comparisons and nodes in correct order to respect foreign keys
    let _ = comparison::Entity::delete_many()
        .filter(comparison::Column::DocumentId.eq(id))
        .exec(&db).await;
    let _ = node::Entity::delete_many()
        .filter(node::Column::DocumentId.eq(id))
        .exec(&db).await;

    // Insert new nodes
    let mut node_id_map = std::collections::HashMap::new();
    let mut nodes_to_insert = payload.nodes;
    
    while !nodes_to_insert.is_empty() {
        let mut inserted_any = false;
        let mut remaining = Vec::new();

        for node in nodes_to_insert {
            let old_id = node.id;
            let can_insert = match node.parent_node_id {
                Some(pid) => node_id_map.contains_key(&pid),
                None => true,
            };

            if can_insert {
                let pid_opt = node.parent_node_id;
                let mut am = node.into_active_model();
                am.id = sea_orm::ActiveValue::NotSet;
                am.document_id = Set(id);
                if let Some(pid) = pid_opt {
                    if let Some(&new_pid) = node_id_map.get(&pid) {
                        am.parent_node_id = Set(Some(new_pid));
                    }
                }
                
                let inserted_node = am.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                node_id_map.insert(old_id, inserted_node.id);
                inserted_any = true;
            } else {
                remaining.push(node);
            }
        }

        if !inserted_any {
            return Err((StatusCode::BAD_REQUEST, "Invalid node hierarchy".to_string()));
        }
        nodes_to_insert = remaining;
    }

    // Insert new comparisons
    for comp in payload.comparisons {
        let mut am = comp.into_active_model();
        am.id = sea_orm::ActiveValue::NotSet;
        am.document_id = Set(id);
        
        if let Some(&new_pid) = node_id_map.get(am.parent_node_id.as_ref()) {
            am.parent_node_id = Set(new_pid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison parent_node_id".to_string()));
        }

        if let Some(&new_aid) = node_id_map.get(am.node_a_id.as_ref()) {
            am.node_a_id = Set(new_aid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison node_a_id".to_string()));
        }

        if let Some(&new_bid) = node_id_map.get(am.node_b_id.as_ref()) {
            am.node_b_id = Set(new_bid);
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid comparison node_b_id".to_string()));
        }

        am.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(updated_doc))
}

pub async fn duplicate_document(
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let txn = db.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // 1. Fetch original document
    let orig_doc = document::Entity::find_by_id(id)
        .one(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document not found".to_string()))?;
        
    // 2. Create new document
    let new_version = orig_doc.version + 1;
    
    // Strip existing (vX) from the name if present, to avoid "MyDoc (v2) (v3)"
    let mut base_name = orig_doc.name.clone();
    if let Some(idx) = base_name.rfind(" (v") {
        if base_name.ends_with(')') {
            base_name.truncate(idx);
        }
    }
    
    let new_doc_name = format!("{} (v{})", base_name, new_version);
    let new_doc = document::ActiveModel {
        name: Set(new_doc_name),
        owner_id: Set(orig_doc.owner_id),
        version: Set(new_version),
        aggregation_method: Set(orig_doc.aggregation_method.clone()),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    let inserted_doc = new_doc.insert(&txn).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let new_doc_id = inserted_doc.id;
    
    // 3. Map Nodes
    let orig_nodes = node::Entity::find()
        .filter(node::Column::DocumentId.eq(id))
        .all(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    let mut node_id_map = std::collections::HashMap::new();
    
    for n in &orig_nodes {
        let mut new_node = node::ActiveModel {
            document_id: Set(new_doc_id),
            name: Set(n.name.clone()),
            node_type: Set(n.node_type.clone()),
            parent_node_id: Set(None), // We will fix parents in a second pass to avoid FK issues
            ..Default::default()
        };
        let inserted = new_node.insert(&txn).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        node_id_map.insert(n.id, inserted.id);
    }
    
    // Second pass: update parent IDs
    for n in &orig_nodes {
        if let Some(old_parent) = n.parent_node_id {
            if let (Some(&new_id), Some(&new_parent)) = (node_id_map.get(&n.id), node_id_map.get(&old_parent)) {
                let mut update_node: node::ActiveModel = node::Entity::find_by_id(new_id).one(&txn).await.unwrap().unwrap().into();
                update_node.parent_node_id = Set(Some(new_parent));
                update_node.update(&txn).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }
    
    // 4. Map Comparisons
    let orig_comps = comparison::Entity::find()
        .filter(comparison::Column::DocumentId.eq(id))
        .all(&txn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    for c in orig_comps {
        let new_parent_id = node_id_map.get(&c.parent_node_id).copied().unwrap_or(0);
        let new_node_a = node_id_map.get(&c.node_a_id).copied().unwrap_or(0);
        let new_node_b = node_id_map.get(&c.node_b_id).copied().unwrap_or(0);
        
        let new_comp = comparison::ActiveModel {
            document_id: Set(new_doc_id),
            respondent_email: Set(c.respondent_email.clone()),
            parent_node_id: Set(new_parent_id),
            node_a_id: Set(new_node_a),
            node_b_id: Set(new_node_b),
            saaty_value: Set(c.saaty_value),
            ..Default::default()
        };
        new_comp.insert(&txn).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    
    txn.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(inserted_doc))
}

#[derive(Serialize, Deserialize)]
pub struct FolderDto {
    pub name: String,
    pub owner_id: i32,
    pub parent_folder_id: Option<i32>,
}

pub async fn list_folders(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<folder::Model>>, (StatusCode, String)> {
    let folders = folder::Entity::find().all(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(folders))
}

pub async fn create_folder(
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<folder::Model>, (StatusCode, String)> {
    let payload: FolderDto = serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    
    let f = folder::ActiveModel {
        name: Set(payload.name),
        owner_id: Set(payload.owner_id),
        parent_folder_id: Set(payload.parent_folder_id),
        ..Default::default()
    };
    
    let inserted = f.insert(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inserted))
}

pub async fn update_folder(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<folder::Model>, (StatusCode, String)> {
    let payload: FolderDto = serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    
    let mut f: folder::ActiveModel = folder::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Folder not found".to_string()))?
        .into();
        
    f.name = Set(payload.name);
    f.parent_folder_id = Set(payload.parent_folder_id);
    
    let updated = f.update(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updated))
}

pub async fn delete_folder(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
) -> Result<StatusCode, (StatusCode, String)> {
    let _ = folder::Entity::delete_by_id(id).exec(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
pub struct MoveDocumentDto {
    pub folder_id: Option<i32>,
}

pub async fn move_document(
    Path(id): Path<i32>,
    State(db): State<DatabaseConnection>,
    body: axum::body::Bytes,
) -> Result<Json<document::Model>, (StatusCode, String)> {
    let payload: MoveDocumentDto = serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    
    let mut doc: document::ActiveModel = document::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Document not found".to_string()))?
        .into();
        
    doc.folder_id = Set(payload.folder_id);
    
    let updated = doc.update(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updated))
}

#[derive(Serialize, Deserialize)]
pub struct TreeDto {
    pub folders: Vec<folder::Model>,
    pub documents: Vec<document::Model>,
}

pub async fn get_tree(
    State(db): State<DatabaseConnection>,
) -> Result<Json<TreeDto>, (StatusCode, String)> {
    let folders = folder::Entity::find().all(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let documents = document::Entity::find().all(&db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(TreeDto { folders, documents }))
}
