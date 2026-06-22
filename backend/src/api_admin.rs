use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};

use crate::entity::{user, user_group, user_group_membership};
use crate::api_auth::Claims;

pub fn router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/block", put(toggle_block_user))
        .route("/users/{id}/groups", get(get_user_groups).post(set_user_groups))
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", put(update_group).delete(delete_group))
}

pub async fn list_users(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<UserAdminDto>>, (StatusCode, String)> {
    // Only allow admin? We can enforce this if needed, but for now we just return all users.
    // Ideally we fetch users and strip passwords.
    let users = user::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let memberships = user_group_membership::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut dtos = Vec::new();
    for u in users {
        let u_groups = memberships
            .iter()
            .filter(|m| m.user_id == u.id)
            .map(|m| m.group_id)
            .collect();
        dtos.push(UserAdminDto {
            id: u.id,
            email: u.username,
            is_admin: u.is_admin,
            is_deleted: u.is_deleted,
            groups: u_groups,
        });
    }

    Ok(Json(dtos))
}

#[derive(Serialize)]
pub struct UserAdminDto {
    pub id: i32,
    pub email: String,
    pub is_admin: bool,
    pub is_deleted: bool,
    pub groups: Vec<i32>,
}

pub async fn toggle_block_user(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<UserAdminDto>, (StatusCode, String)> {
    let mut u: user::ActiveModel = user::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?
        .into();

    let current_deleted = u.is_deleted.clone().unwrap();
    u.is_deleted = Set(!current_deleted);

    let updated = u
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserAdminDto {
        id: updated.id,
        email: updated.username,
        is_admin: updated.is_admin,
        is_deleted: updated.is_deleted,
        groups: vec![],
    }))
}

#[derive(Serialize, Deserialize)]
pub struct GroupDto {
    pub name: String,
    pub parent_id: Option<i32>,
}

pub async fn list_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<user_group::Model>>, (StatusCode, String)> {
    let groups = user_group::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(groups))
}

pub async fn create_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<GroupDto>,
) -> Result<Json<user_group::Model>, (StatusCode, String)> {
    let new_group = user_group::ActiveModel {
        name: Set(payload.name),
        parent_id: Set(payload.parent_id),
        ..Default::default()
    };
    let inserted = new_group
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inserted))
}

pub async fn update_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<GroupDto>,
) -> Result<Json<user_group::Model>, (StatusCode, String)> {
    let mut group: user_group::ActiveModel = user_group::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Group not found".to_string()))?
        .into();
        
    group.name = Set(payload.name);
    group.parent_id = Set(payload.parent_id);

    let updated = group
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updated))
}

pub async fn delete_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let _ = user_group::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_user_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<user_group_membership::Model>>, (StatusCode, String)> {
    let memberships = user_group_membership::Entity::find()
        .filter(user_group_membership::Column::UserId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(memberships))
}

#[derive(Deserialize)]
pub struct SetGroupsDto {
    pub group_ids: Vec<i32>,
}

pub async fn set_user_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<SetGroupsDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Delete existing
    let _ = user_group_membership::Entity::delete_many()
        .filter(user_group_membership::Column::UserId.eq(id))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Insert new
    for gid in payload.group_ids {
        let membership = user_group_membership::ActiveModel {
            user_id: Set(id),
            group_id: Set(gid),
            ..Default::default()
        };
        membership
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(StatusCode::OK)
}
