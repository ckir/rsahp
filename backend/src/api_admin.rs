//! Administrative API endpoints.
//!
//! This module provides administrative routes for managing users, groups,
//! and user group memberships within the system.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::api_auth::Claims;
use crate::entity::{user, user_group, user_group_membership};

/// Returns the router for administrative endpoints.
pub fn router() -> Router<DatabaseConnection> {
    // Create and return a new axum Router configured with admin routes
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/block", put(toggle_block_user))
        .route(
            "/users/{id}/groups",
            get(get_user_groups).post(set_user_groups),
        )
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", put(update_group).delete(delete_group))
}

/// Retrieves a list of all users and their group memberships.
pub async fn list_users(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<UserAdminDto>>, (StatusCode, String)> {
    // Only allow admin? We can enforce this if needed, but for now we just return all users.
    // Ideally we fetch users and strip passwords.

    // Query all users from the database
    let users = user::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Query all user group memberships to attach them to users
    let memberships = user_group_membership::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Initialize a vector to hold the resulting data transfer objects
    let mut dtos = Vec::new();

    // Iterate over each user to construct their DTO
    for u in users {
        // Find group IDs associated with the current user
        let u_groups = memberships
            .iter()
            .filter(|m| m.user_id == u.id)
            .map(|m| m.group_id)
            .collect();

        // Append the constructed DTO to the results list
        dtos.push(UserAdminDto {
            id: u.id,
            email: u.username,
            is_admin: u.is_admin,
            is_deleted: u.is_deleted,
            groups: u_groups,
        });
    }

    // Return the JSON serialized vector of UserAdminDtos
    Ok(Json(dtos))
}

/// Data transfer object for admin user information.
#[derive(Serialize)]
pub struct UserAdminDto {
    /// The unique identifier of the user
    pub id: i32,
    /// The email or username of the user
    pub email: String,
    /// Flag indicating if the user has administrative privileges
    pub is_admin: bool,
    /// Flag indicating if the user's account is marked as deleted/blocked
    pub is_deleted: bool,
    /// List of group IDs the user belongs to
    pub groups: Vec<i32>,
}

/// Toggles the blocked (deleted) status of a user.
pub async fn toggle_block_user(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<UserAdminDto>, (StatusCode, String)> {
    // Fetch the user to toggle by their ID
    let mut u: user::ActiveModel = user::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?
        .into();

    // Determine the current deleted status
    let current_deleted = u.is_deleted.clone().unwrap();

    // Flip the deleted status
    u.is_deleted = Set(!current_deleted);

    // Save the updated user back to the database
    let updated = u
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the updated user as a DTO
    Ok(Json(UserAdminDto {
        id: updated.id,
        email: updated.username,
        is_admin: updated.is_admin,
        is_deleted: updated.is_deleted,
        groups: vec![],
    }))
}

/// Data transfer object for group creation and updates.
#[derive(Serialize, Deserialize)]
pub struct GroupDto {
    /// The name of the group
    pub name: String,
    /// The optional parent group ID, if this is a nested group
    pub parent_id: Option<i32>,
}

/// Retrieves a list of all user groups.
pub async fn list_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<user_group::Model>>, (StatusCode, String)> {
    // Query all user groups from the database
    let groups = user_group::Entity::find()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the retrieved groups wrapped in JSON
    Ok(Json(groups))
}

/// Creates a new user group.
pub async fn create_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<GroupDto>,
) -> Result<Json<user_group::Model>, (StatusCode, String)> {
    // Construct a new active model from the payload data
    let new_group = user_group::ActiveModel {
        name: Set(payload.name),
        parent_id: Set(payload.parent_id),
        ..Default::default()
    };

    // Insert the new group into the database
    let inserted = new_group
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the created group
    Ok(Json(inserted))
}

/// Updates an existing user group.
pub async fn update_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<GroupDto>,
) -> Result<Json<user_group::Model>, (StatusCode, String)> {
    // Fetch the existing group by ID
    let mut group: user_group::ActiveModel = user_group::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Group not found".to_string()))?
        .into();

    // Update the group's properties
    group.name = Set(payload.name);
    group.parent_id = Set(payload.parent_id);

    // Save the changes to the database
    let updated = group
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the updated group
    Ok(Json(updated))
}

/// Deletes a user group by its ID.
pub async fn delete_group(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Execute a delete operation for the specified group ID
    let _ = user_group::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return a No Content status indicating successful deletion
    Ok(StatusCode::NO_CONTENT)
}

/// Retrieves the group memberships for a specific user.
pub async fn get_user_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<user_group_membership::Model>>, (StatusCode, String)> {
    // Query group memberships filtering by the specified user ID
    let memberships = user_group_membership::Entity::find()
        .filter(user_group_membership::Column::UserId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the list of memberships
    Ok(Json(memberships))
}

/// Data transfer object for assigning groups to a user.
#[derive(Deserialize)]
pub struct SetGroupsDto {
    /// List of group IDs to assign to the user
    pub group_ids: Vec<i32>,
}

/// Replaces the group memberships for a user.
pub async fn set_user_groups(
    _claims: Claims,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<SetGroupsDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Delete existing memberships for the user
    let _ = user_group_membership::Entity::delete_many()
        .filter(user_group_membership::Column::UserId.eq(id))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Insert new memberships as specified in the payload
    for gid in payload.group_ids {
        // Construct the new membership model
        let membership = user_group_membership::ActiveModel {
            user_id: Set(id),
            group_id: Set(gid),
            ..Default::default()
        };

        // Insert the membership into the database
        membership
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Return OK status upon successful updates
    Ok(StatusCode::OK)
}
