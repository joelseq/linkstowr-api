use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use lazy_regex::regex_captures;
use shuttle_secrets::SecretStore;

use crate::{
    auth::validate_jwt,
    ctx::Ctx,
    error::{Error, Result},
    prefixed_api_key::PrefixedApiKey,
    types::{AppState, Token},
};

pub async fn mw_require_auth<B>(
    ctx: Result<Ctx>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    println!("->> {:<12} - mw_require_auth - {ctx:?}", "MIDDLEWARE");

    ctx?;

    Ok(next.run(req).await)
}

pub async fn mw_ctx_resolver<B>(
    app_state: State<AppState>,
    headers: HeaderMap,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    println!("->> {:<12} - mw_ctx_resolver", "MIDDLEWARE");

    // We need to get the Ctx as Result<Ctx> because it may not always be set
    let result_ctx = get_result_ctx(app_state, headers).await;

    req.extensions_mut().insert(result_ctx);

    Ok(next.run(req).await)
}

async fn get_result_ctx(app_state: State<AppState>, headers: HeaderMap) -> Result<Ctx> {
    let auth_header = headers.get(AUTHORIZATION);
    let token_header = headers.get("X-Api-Token");

    let user_id = match (auth_header, token_header) {
        // Prefer to use the Authorization header if it is available
        (Some(auth_header), _) => get_user_from_auth_header(auth_header, &app_state.secret_store),
        (_, Some(token_header)) => get_user_from_token_header(token_header, &app_state).await,
        (_, _) => Err(Error::MissingAuth),
    }?;

    Ok(Ctx::new(user_id))
}

// region:    --- Ctx Extractor
#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
        println!("->> {:<12} - Ctx", "EXTRACTOR");

        parts
            .extensions
            .get::<Result<Ctx>>()
            .ok_or(Error::AuthFailCtxNotInRequestExt)?
            .clone()
    }
}

// endregion: --- Ctx Extractor

fn get_user_from_auth_header(header: &HeaderValue, secret_store: &SecretStore) -> Result<String> {
    let auth_header = std::str::from_utf8(header.as_bytes())
        .ok()
        .ok_or(Error::MissingAuth)?;
    let pattern = regex_captures!(r#"^Bearer (.+)"#, auth_header);

    let user = match pattern {
        Some((_, bearer_token)) => validate_jwt(bearer_token, secret_store),
        None => Err(Error::InvalidAuthHeader),
    }?;

    Ok(user.sub)
}

async fn get_user_from_token_header(header: &HeaderValue, app_state: &AppState) -> Result<String> {
    let token = std::str::from_utf8(header.as_bytes())
        .ok()
        .ok_or(Error::MissingAuth)?;

    let user_id = validate_api_token(token, app_state).await?;

    Ok(user_id)
}

async fn validate_api_token(token: &str, app_state: &AppState) -> Result<String> {
    let pak: PrefixedApiKey = token.try_into().map_err(|_| Error::InvalidToken)?;
    let hash = pak.long_token_hashed();

    let mut result = app_state
        .db
        .query("SELECT * FROM token WHERE token_hash = $token_hash")
        .bind(("token_hash", hash))
        .await
        .map_err(|e| {
            println!("Encountered error {:?}", e);
            Error::InvalidToken
        })?;
    let token: Option<Token> = result.take(0).map_err(|e| {
        println!("Encountered error {:?}", e);
        Error::InvalidToken
    })?;

    match token {
        Some(token) => Ok(token.user.to_string()),
        None => {
            println!("Failing at the match statement");
            Err(Error::InvalidToken)
        }
    }
}
