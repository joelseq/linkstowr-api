use axum::{
    http::Method,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::error;
use uuid::Uuid;

use crate::{
    error::Error,
    middlewares,
    routes::{auth, health_check, link_routes, token},
    types::AppState,
};

pub fn get_app(state: &AppState) -> Router {
    let token_routes = token::routes(state.clone());
    let api_routes = link_routes::routes(state.clone())
        .merge(token_routes)
        .route_layer(middleware::from_fn(middlewares::auth::mw_require_auth));

    let auth_routes = auth::routes(state.clone());

    Router::new()
        .merge(auth_routes)
        .nest("/api", api_routes)
        .layer(middleware::map_response(main_response_mapper))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::mw_ctx_resolver,
        ))
        // include trace context as header into the response
        .layer(OtelInResponseLayer)
        // start OpenTelemetry trace on incoming request
        .layer(OtelAxumLayer::default())
        .route("/health_check", get(health_check))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([Method::POST, Method::GET]),
        )
}

async fn main_response_mapper(res: Response) -> Response {
    let uuid = Uuid::new_v4();

    // -- Get the eventual response error.
    let service_error = res.extensions().get::<Error>();
    let client_status_error = service_error.map(|se| se.client_status_and_error());

    // -- If client error, build the new reponse.
    let error_response = client_status_error
        .as_ref()
        .map(|(status_code, client_error)| {
            let client_error_body = json!({
                "error": {
                    "type": client_error.as_ref(),
                    "req_uuid": uuid.to_string(),
                }
            });

            error!("    ->> client_error_body: {client_error_body}");

            // Build the new response from the client_error_body
            (*status_code, Json(client_error_body)).into_response()
        });

    error_response.unwrap_or(res)
}
