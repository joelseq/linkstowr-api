use std::sync::Arc;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{extract::State, routing::post, Json, Router};
use lazy_regex::regex_captures;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::error;

use crate::auth::{create_jwt, validate_jwt};
use crate::error::{Error, Result};
use crate::types::{AppState, CreateUserContent, User, UserDBResult, DB};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/signin", post(signin))
        .route("/signup", post(signup))
        .route("/me", get(get_user_info))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct SigninPayload {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    // Making these public for test assertions
    pub id: String,
    pub username: String,
    token: String,
}

async fn signin(
    State(app_state): State<AppState>,
    Json(payload): Json<SigninPayload>,
) -> Result<Json<UserResponse>> {
    let mut result = app_state
        .db
        .query("SELECT * FROM user WHERE username = $username")
        .bind(("username", payload.username))
        .await
        .map_err(|_| Error::SignInFail)?;

    let user: Option<UserDBResult> = result.take(0).map_err(|_| Error::SignInFail)?;
    let user = user.ok_or(Error::InvalidCredentials)?;

    let parsed_hash = PasswordHash::new(&user.password).map_err(|_| Error::SignInFail)?;
    match Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash) {
        Ok(_) => {
            let user: User = user.into();
            let token = create_jwt(&user)?;

            let body = Json(UserResponse {
                id: user.id.to_string(),
                username: user.username.clone(),
                token,
            });

            Ok(body)
        }
        Err(e) => {
            error!("Failed with error {e:?}");
            Err(Error::InvalidCredentials)
        }
    }
}

#[derive(Debug, Deserialize)]
struct SignupPayload {
    username: String,
    password: String,
    password_confirm: String,
}

async fn signup(
    State(app_state): State<AppState>,
    Json(payload): Json<SignupPayload>,
) -> Result<Json<UserResponse>> {
    if payload.password != payload.password_confirm {
        return Err(Error::PasswordConfirmMismatch);
    }
    let mut result = app_state
        .db
        .query("SELECT * FROM user WHERE username = $username")
        .bind(("username", &payload.username))
        .await
        .map_err(|_| Error::SignUpFail)?;

    let user: Option<UserDBResult> = result.take(0).map_err(|_| Error::SignUpFail)?;

    if user.is_some() {
        return Err(Error::UsernameExists);
    }

    let user = create_user(payload.username, payload.password, app_state.db).await?;
    let token = create_jwt(&user)?;

    let body = Json(UserResponse {
        id: user.id.to_string(),
        username: user.username,
        token,
    });

    Ok(body)
}

pub async fn create_user(username: String, password: String, db: Arc<DB>) -> Result<User> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(&password.into_bytes(), &salt)
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::SignUpFail
        })?
        .to_string();

    let result: Vec<User> = db
        .create("user")
        .content(CreateUserContent {
            username,
            password: password_hash,
        })
        .await
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::SignUpFail
        })?;
    let user = result.first().ok_or(Error::SignUpFail)?.to_owned();

    Ok(user)
}

async fn get_user_info(headers: HeaderMap) -> Result<Json<Value>> {
    let auth_header = headers.get(AUTHORIZATION).ok_or(Error::InvalidAuthHeader)?;

    let auth_header = std::str::from_utf8(auth_header.as_bytes())
        .ok()
        .ok_or(Error::InvalidAuthHeader)?;
    let pattern = regex_captures!(r#"^Bearer (.+)"#, auth_header);

    let claims = match pattern {
        Some((_, bearer_token)) => validate_jwt(bearer_token),
        None => Err(Error::InvalidAuthHeader),
    }?;

    let body = Json(json!({
        "id": claims.sub,
        "username": claims.username,
    }));

    Ok(body)
}
