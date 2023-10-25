use axum::{extract::State, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use surrealdb::sql::thing;
use tracing::error;

use crate::{
    ctx::Ctx,
    error::{Error, Result},
    types::{AppState, Link, LinkPayload},
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/links", post(create_link).get(get_links))
        .route("/links/clear", post(clear_links))
        .with_state(state)
}

#[tracing::instrument(
    name = "Creating a link",
    skip(ctx, app_state),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn create_link(
    ctx: Ctx,
    State(app_state): State<AppState>,
    Json(payload): Json<LinkPayload>,
) -> Result<Json<Value>> {
    let created: Vec<Link> = app_state
        .db
        .create("link")
        .content(Link {
            url: payload.url.clone(),
            title: payload.title.clone(),
            note: payload.note.clone(),
            user: thing(ctx.user_id()).expect("Failed to convert ctx user_id to thing"),
        })
        .await
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::CreateLinkFail
        })?;

    let created = created.first().ok_or(Error::CreateLinkFail)?;

    let body = Json(json!({
        "result": {
            "url": created.url,
            "success": true,
        }
    }));

    Ok(body)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LinkResponse {
    pub url: String,
    pub title: String,
    pub note: String,
    pub bookmarked_at: DateTime<Utc>,
}

#[tracing::instrument(
    name = "Getting links",
    skip(ctx, app_state),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn get_links(ctx: Ctx, State(app_state): State<AppState>) -> Result<Json<Vec<LinkResponse>>> {
    let mut result = app_state
        .db
        .query("SELECT * FROM link WHERE user.id = $user_id;")
        .bind(("user_id", thing(ctx.user_id()).unwrap()))
        .await
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::GetLinksFail
        })?;

    let links: Vec<LinkResponse> = result.take(0).map_err(|e| {
        error!("Encountered error {:?}", e);
        Error::GetLinksFail
    })?;

    let body = Json(links);

    Ok(body)
}

#[tracing::instrument(
    name = "Clearing links",
    skip(ctx, app_state),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn clear_links(ctx: Ctx, State(app_state): State<AppState>) -> Result<Json<Value>> {
    let mut result = app_state
        .db
        .query("DELETE link WHERE user.id = $user_id;")
        .bind(("user_id", ctx.user_id()))
        .await
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::ClearLinksFail
        })?;

    let deleted: surrealdb::Result<Vec<Link>> = result.take(0);

    match deleted {
        Ok(_) => {
            let body = Json(json!({
                "success": true,
            }));

            Ok(body)
        }
        Err(_) => Err(Error::ClearLinksFail),
    }
}
