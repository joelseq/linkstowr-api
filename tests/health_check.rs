use std::net::TcpListener;

use linkshelf::{app::get_app, types::AppState};

pub struct TestApp {
    pub address: String,
    pub state: AppState,
}

async fn spawn_app() -> TestApp {
    let db = surrealdb::engine::any::connect("mem://")
        .await
        .expect("Failed to initialize test db");
    db.use_ns("test").use_db("test").await.unwrap();

    let state = AppState::new(db);

    let app = get_app(&state);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = axum::Server::from_tcp(listener)
        .expect("Failed to start server from TCP listener")
        .serve(app.into_make_service());

    let _ = tokio::spawn(server);
    let address = format!("http://127.0.0.1:{}", port);
    println!("->> LISTENING on {address}\n");

    TestApp { address, state }
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute reqwest.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
