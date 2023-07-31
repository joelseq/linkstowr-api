use axum::extract::Path;
use axum::routing::delete;
use axum::Json;
use axum::{extract::State, routing::post, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use surrealdb::sql::{thing, Thing};
use tracing::error;

use crate::ctx::Ctx;
use crate::error::{Error, Result};
use crate::prefixed_api_key::PrefixedApiKeyController;
use crate::types::{AppState, Token};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/tokens", post(gen_token).get(get_tokens))
        .route("/tokens/:id", delete(delete_token))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct CreateTokenPayload {
    name: String,
}

#[derive(Debug, Serialize)]
struct UpdateTokenContent {
    token_hash: String,
}

#[derive(Debug, Serialize)]
struct TokenResponse {
    token: String,
}

#[tracing::instrument(
    name = "Creating a new Token",
    skip(ctx, app_state, payload),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn gen_token(
    State(app_state): State<AppState>,
    ctx: Ctx,
    Json(payload): Json<CreateTokenPayload>,
) -> Result<Json<TokenResponse>> {
    let controller = PrefixedApiKeyController::new("lshelf".into(), 8, 24);
    let (pak, hash) = controller.generate_key_and_hash();

    let _result: Token = app_state
        .db
        .create("token")
        .content(Token {
            token_hash: hash.clone(),
            name: payload.name.clone(),
            short_token: pak.short_token().into(),
            user: thing(ctx.user_id()).expect("Failed to convert ctx user_id to thing"),
        })
        .await
        .map_err(|e| {
            error!("Encountered error {:?}", e);
            Error::GenTokenFail
        })?;

    let body = Json(TokenResponse {
        token: pak.to_string(),
    });

    Ok(body)
}

#[derive(Debug, Deserialize, Serialize)]
struct ListTokensItem {
    pub id: Thing,
    pub name: String,
    pub short_token: String,
}

#[tracing::instrument(
    name = "Get Tokens for user",
    skip(ctx, app_state),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn get_tokens(
    ctx: Ctx,
    State(app_state): State<AppState>,
) -> Result<Json<Vec<ListTokensItem>>> {
    let mut result = app_state
        .db
        .query("SELECT * FROM token WHERE user.id = $user_id;")
        .bind(("user_id", ctx.user_id()))
        .await
        .map_err(|_| Error::GetTokensFail)?;

    let tokens: Vec<ListTokensItem> = result.take(0).map_err(|_| Error::GetTokensFail)?;

    let body = Json(tokens);

    Ok(body)
}

#[tracing::instrument(
    name = "Deleting token",
    skip(ctx, app_state),
    fields(
        user_id = %ctx.user_id(),
    )
)]
async fn delete_token(
    ctx: Ctx,
    State(app_state): State<AppState>,
    Path(token_id): Path<String>,
) -> Result<Json<Value>> {
    let parts = token_id.split(':').collect::<Vec<&str>>();

    if parts.len() != 2 {
        return Err(Error::InvalidDeleteToken);
    }

    let mut result = app_state
        .db
        .query("DELETE token WHERE id = $token_id AND user = $user_id;")
        .bind(("token_id", token_id))
        .bind(("user_id", ctx.user_id()))
        .await
        .map_err(|_| Error::DeleteTokenFail)?;

    let deleted: surrealdb::Result<Vec<Token>> = result.take(0);

    match deleted {
        Ok(_) => {
            let body = Json(json!({
                "success": true,
            }));

            Ok(body)
        }
        Err(_) => Err(Error::DeleteTokenFail),
    }
}
