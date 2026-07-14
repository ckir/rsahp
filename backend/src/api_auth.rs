// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Authentication API endpoints.
//!
//! This module provides routes for user registration and login,
//! as well as JWT token generation and validation.

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header::AUTHORIZATION},
    routing::post,
};
use bcrypt::{hash, verify};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::entity::user;

/// The secret key used for signing JWT tokens.
const JWT_SECRET: &[u8] = b"super-secret-key-change-in-production";

/// JWT claims for authenticated users.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: i32,
    /// Expiration time
    pub exp: usize,
}

/// Returns the router for authentication endpoints.
pub fn router() -> Router<DatabaseConnection> {
    // Create and configure a router with registration and login routes
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

/// Data transfer object for login and registration payloads.
#[derive(Deserialize)]
pub struct AuthPayload {
    /// User's email or username
    pub email: String,
    /// User's raw password
    pub password: String,
}

/// Data transfer object for authentication responses.
#[derive(Serialize)]
pub struct AuthResponse {
    /// JWT token for the session
    pub token: String,
    /// Basic information about the authenticated user
    pub user: UserDto,
}

/// Data transfer object for basic user info.
#[derive(Serialize)]
pub struct UserDto {
    /// Unique user identifier
    pub id: i32,
    /// Email or username
    pub email: String,
    /// Whether the user has admin privileges
    pub is_admin: bool,
}

/// Registers a new user.
pub async fn register(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // Ensure email and password are provided
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Missing email or password".into()));
    }

    // Check if the email is already registered
    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.email))
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return conflict if the user already exists
    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Email already taken".into()));
    }

    // Hash the given password with a cost of 4
    let hashed = hash(payload.password, 4)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Prepare the new user model with the hashed password
    let new_user = user::ActiveModel {
        username: Set(payload.email.clone()),
        password_hash: Set(hashed),
        is_admin: Set(false),
        is_deleted: Set(false),
        ..Default::default()
    };

    // Insert the newly created user into the database
    let inserted = new_user
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Formulate JWT claims expiring in 30 days
    let claims = Claims {
        sub: inserted.id,
        exp: (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize,
    };

    // Encode the JWT token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the authentication response with token and user details
    Ok(Json(AuthResponse {
        token,
        user: UserDto {
            id: inserted.id,
            email: inserted.username,
            is_admin: inserted.is_admin,
        },
    }))
}

/// Authenticates an existing user and returns a token.
pub async fn login(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // Attempt to locate the user by their email
    let user_opt = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.email))
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Resolve the user option, failing with unauthorized if not found
    let user = match user_opt {
        Some(u) => u,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into())),
    };

    // Prevent blocked or deleted users from authenticating
    if user.is_deleted {
        return Err((StatusCode::FORBIDDEN, "Account blocked".into()));
    }

    // "hash" is the dummy placeholder we used in setup_schema for admin
    // Verify the password using bcrypt, falling back to false on error
    let valid = if user.password_hash == "hash" {
        payload.password == "admin"
    } else {
        verify(&payload.password, &user.password_hash).unwrap_or(false)
    };

    // Reject invalid credentials
    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
    }

    // Create JWT claims valid for 30 days
    let claims = Claims {
        sub: user.id,
        exp: (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize,
    };

    // Generate the JWT token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return successful authentication payload
    Ok(Json(AuthResponse {
        token,
        user: UserDto {
            id: user.id,
            email: user.username,
            is_admin: user.is_admin,
        },
    }))
}

// Extractor for auth (not strictly necessary to be in a route, but useful to have)
// In Axum we can implement FromRequestParts for Claims.
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    /// Extracts the JWT claims from the request parts
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Retrieve the authorization header value
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        // Extract the token string after the "Bearer " prefix
        let token = if let Some(header) = auth_header {
            if let Some(stripped) = header.strip_prefix("Bearer ") {
                stripped.to_string()
            } else {
                return Err((StatusCode::UNAUTHORIZED, "Invalid Auth header".into()));
            }
        } else {
            return Err((StatusCode::UNAUTHORIZED, "Missing Auth header".into()));
        };

        // Decode the token and perform validation against the secret
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)))?;

        // Return the successfully decoded claims
        Ok(token_data.claims)
    }
}
