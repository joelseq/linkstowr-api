use axum::{
    http::{Method, Uri},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use linkshelf::{
    configuration::Settings,
    ctx::Ctx,
    error::Error,
    log::log_request,
    middlewares,
    routes::{auth, health_check, link_routes, token},
    types::AppState,
};
use serde_json::json;
use shuttle_secrets::SecretStore;
use surrealdb::{
    engine::remote::http::{Http, Https},
    opt::auth::Root,
    Surreal,
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[shuttle_runtime::main]
async fn axum(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> shuttle_axum::ShuttleAxum {
    let configuration: Settings = (&secret_store)
        .try_into()
        .expect("Failed to read configuration from secret store.");

    let connection_string = configuration.database.get_connection_string();
    let db = if configuration.database.secure {
        Surreal::new::<Https>(connection_string)
            .await
            .expect("Could not connect to SurrealDB")
    } else {
        Surreal::new::<Http>(connection_string)
            .await
            .expect("Could not connect to SurrealDB")
    };
    db.signin(Root {
        username: &configuration.database.username,
        password: &configuration.database.password,
    })
    .await
    .expect("Could not sign into SurrealDB");

    db.use_ns("dev").use_db("dev").await.unwrap();

    let state = AppState::new(db, secret_store);

    let token_routes = token::routes(state.clone());
    let api_routes = link_routes::routes(state.clone())
        .merge(token_routes)
        .route_layer(middleware::from_fn(middlewares::auth::mw_require_auth));

    let auth_routes = auth::routes(state.clone());

    let router = Router::new()
        .route("/health_check", get(health_check))
        .merge(auth_routes)
        .nest("/api", api_routes)
        .layer(middleware::map_response(main_response_mapper))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::mw_ctx_resolver,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([Method::POST, Method::GET]),
        );

    Ok(router.into())
}

async fn main_response_mapper(
    ctx: Option<Ctx>,
    uri: Uri,
    req_method: Method,
    res: Response,
) -> Response {
    println!("->> {:<12} - main_response_mapper", "RES_MAPPER");
    let uuid = Uuid::new_v4();

    let service_error = res.extensions().get::<Error>();
    let client_status_error = service_error.map(|se| se.client_status_and_error());

    let error_response = client_status_error
        .as_ref()
        .map(|(status_code, client_error)| {
            let client_error_body = json!({
                "error": {
                    "type": client_error.as_ref(),
                    "req_uuid": uuid.to_string(),
                }
            });

            println!("      ->> client_error_body: {client_error_body}");

            // Build the new response from the client_error_body
            (*status_code, Json(client_error_body)).into_response()
        });

    // Build and log the server log line.
    let client_error = client_status_error.unzip().1;

    let _ = log_request(uuid, req_method, uri, ctx, service_error, client_error).await;

    println!();
    error_response.unwrap_or(res)
}
