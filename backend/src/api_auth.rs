use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header::AUTHORIZATION},
    routing::post,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use bcrypt::{hash, verify};

use crate::entity::user;

const JWT_SECRET: &[u8] = b"super-secret-key-change-in-production";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32, // user_id
    pub exp: usize,
}

pub fn router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[derive(Deserialize)]
pub struct AuthPayload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

#[derive(Serialize)]
pub struct UserDto {
    pub id: i32,
    pub email: String,
    pub is_admin: bool,
}

pub async fn register(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Missing email or password".into()));
    }

    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.email))
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Email already taken".into()));
    }

    let hashed = hash(payload.password, 4)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let new_user = user::ActiveModel {
        username: Set(payload.email.clone()),
        password_hash: Set(hashed),
        is_admin: Set(false),
        is_deleted: Set(false),
        ..Default::default()
    };

    let inserted = new_user
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let claims = Claims {
        sub: inserted.id,
        exp: (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: UserDto {
            id: inserted.id,
            email: inserted.username,
            is_admin: inserted.is_admin,
        },
    }))
}

pub async fn login(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let user_opt = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.email))
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = match user_opt {
        Some(u) => u,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into())),
    };

    if user.is_deleted {
        return Err((StatusCode::FORBIDDEN, "Account blocked".into()));
    }

    // "hash" is the dummy placeholder we used in setup_schema for admin
    let valid = if user.password_hash == "hash" {
        payload.password == "admin"
    } else {
        verify(&payload.password, &user.password_hash).unwrap_or(false)
    };

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
    }

    let claims = Claims {
        sub: user.id,
        exp: (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        let token = if let Some(header) = auth_header {
            if let Some(stripped) = header.strip_prefix("Bearer ") {
                stripped.to_string()
            } else {
                return Err((StatusCode::UNAUTHORIZED, "Invalid Auth header".into()));
            }
        } else {
            return Err((StatusCode::UNAUTHORIZED, "Missing Auth header".into()));
        };

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)))?;

        Ok(token_data.claims)
    }
}
