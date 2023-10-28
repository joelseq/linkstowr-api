use std::sync::Arc;

use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::any::Any,
    sql::{Datetime, Thing},
    Surreal,
};

pub type DB = Surreal<Any>;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DB>,
}

impl AppState {
    pub fn new(db: DB) -> Self {
        AppState { db: Arc::new(db) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    pub title: String,
    pub note: String,
    pub bookmarked_at: Datetime,
    pub user: Thing,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LinkPayload {
    pub url: String,
    pub title: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserContent {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Thing,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDBResult {
    pub id: Thing,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token_hash: String,
    pub name: String,
    pub short_token: String,
    pub user: Thing,
}

impl From<UserDBResult> for User {
    fn from(db_result: UserDBResult) -> Self {
        Self {
            id: db_result.id,
            username: db_result.username,
        }
    }
}
