use dotenv::dotenv;
use linkstowr::{
    app::get_app,
    configuration::{get_configuration, get_environment, Environment, Settings},
    telemetry::init_subscribers,
    types::AppState,
};
use surrealdb::{engine::any::Any, opt::auth::Root, Surreal};
use surrealdb_migrations::MigrationRunner;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let environment = get_environment();
    // We need to initialize the OTEL env vars in local before initializing subscribers
    if let Environment::Local = environment {
        dotenv().ok();
    }

    init_subscribers().expect("Unable to init tracing subscribers");

    let configuration = get_configuration().expect("Failed to read configuration.");

    let db = get_db(&configuration).await;

    let state = AppState::new(db);

    let app = get_app(&state);

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    info!("->> LISTENING on {address}\n");
    axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

#[tracing::instrument(
    name = "Setting up Database",
    skip(configuration),
    fields(
        namespace = %configuration.database.ns,
        database = %configuration.database.db,
    )
)]
async fn get_db(configuration: &Settings) -> Surreal<Any> {
    let connection_string = configuration.database.get_connection_string();
    let db = surrealdb::engine::any::connect(connection_string)
        .await
        .expect("Could not connect to SurrealDB");
    db.signin(Root {
        username: &configuration.database.username,
        password: &configuration.database.password,
    })
    .await
    .expect("Could not sign into SurrealDB");

    db.use_ns(&configuration.database.ns)
        .use_db(&configuration.database.db)
        .await
        .unwrap();

    MigrationRunner::new(&db)
        .up()
        .await
        .expect("Failed to apply migrations");

    info!(
        "Connected to namespace: {}, database: {}",
        &configuration.database.ns, &configuration.database.db
    );

    db
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
    opentelemetry::global::shutdown_tracer_provider();
}
