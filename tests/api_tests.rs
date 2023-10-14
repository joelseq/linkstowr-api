use std::{fs, net::TcpListener};

use linkstowr::{
    app::get_app,
    error::Result,
    prefixed_api_key::PrefixedApiKey,
    routes::{
        auth::{create_user, UserResponse},
        link_routes::LinkResponse,
        token::gen_pak,
    },
    types::AppState,
};
use serde_json::{json, Value};
use surrealdb::sql::thing;
use uuid::Uuid;

const TEST_USER_PASSWORD: &str = "password";
const JWT_ENCODING_SECRET: &str = "super-secret";

pub struct TestApp {
    pub address: String,
    pub state: AppState,
}

async fn spawn_app() -> TestApp {
    let db = surrealdb::engine::any::connect("mem://")
        .await
        .expect("Failed to initialize test db");
    db.use_ns("test").use_db("test").await.unwrap();

    // Initialize schema
    let schema =
        fs::read_to_string("db/schema.sql").expect("Failed to read schema file from db/schema.sql");

    db.query(schema)
        .await
        .expect("Failed to initialize the DB schema");

    // Setup env var for JWT
    std::env::set_var("JWT_ENCODING_SECRET", JWT_ENCODING_SECRET);

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

struct TestUser {
    id: String,
    username: String,
    pak: PrefixedApiKey,
}

async fn create_test_user(app_state: &AppState) -> Result<TestUser> {
    let user = create_user(
        format!("test_user_{}", Uuid::new_v4().to_string()),
        TEST_USER_PASSWORD.into(),
        app_state.db.clone(),
    )
    .await?;
    let user_id = user.id.to_string();
    let pak = gen_pak(app_state, &user_id, "test_token").await?;

    Ok(TestUser {
        id: user_id,
        username: user.username,
        pak,
    })
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

#[tokio::test]
async fn sign_up_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(&format!("{}/signup", &app.address))
        .header("Content-Type", "application/json")
        .body(
            r#"{
                "username": "test",
                "password": "test",
                "password_confirm": "test"
            }"#,
        )
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success(),);
    let sign_up_response = response
        .json::<UserResponse>()
        .await
        .expect("Failed to parse json body");
    assert_eq!(&sign_up_response.username, "test");
    assert!(!&sign_up_response.id.is_empty()); // should contain an id
}

#[tokio::test]
async fn sign_up_errors() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        (r#"{}"#, 422, "missing required fields"),
        (
            r#"{
                "username": "test",
                "password": "test",
            }"#,
            400,
            "missing the password_confirm field",
        ),
        (
            r#"{
                "username": "test",
                "password": "test",
                "password_confirm": "nottest"
            }"#,
            400,
            "password mismatch",
        ),
    ];

    // Act
    for (invalid_body, status_code, error_message) in test_cases {
        let response = client
            .post(&format!("{}/signup", &app.address))
            .header("Content-Type", "application/json")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            status_code,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with status code {} for the test case {}.",
            response.status().as_str(),
            error_message
        );
    }
}

#[tokio::test]
async fn sign_in_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_user = create_test_user(&app.state)
        .await
        .expect("Failed to create test user");

    // Act
    let response = client
        .post(&format!("{}/signin", &app.address))
        .header("Content-Type", "application/json")
        .body(
            json!({
                "username": test_user.username,
                "password": TEST_USER_PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success(),);
    let sign_in_response = response
        .json::<UserResponse>()
        .await
        .expect("Failed to parse json body");
    assert_eq!(&sign_in_response.username, &test_user.username);
    assert_eq!(&sign_in_response.id, &test_user.id);
}

#[tokio::test]
async fn sign_in_errors() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_user = create_test_user(&app.state)
        .await
        .expect("Failed to create test user");
    let test_cases = vec![
        (json!({}), 422, "missing required fields"),
        (
            json!({
                "username": "random",
                "password": TEST_USER_PASSWORD,
            }),
            400,
            "user doesn't exist",
        ),
        (
            json!({
                "username": &test_user.username,
                "password": "badpassword",
            }),
            400,
            "wrong password",
        ),
    ];

    // Act
    for (invalid_body, status_code, error_message) in test_cases {
        let response = client
            .post(&format!("{}/signin", &app.address))
            .header("Content-Type", "application/json")
            .body(invalid_body.to_string())
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            status_code,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with status code {} for the test case {}.",
            response.status().as_str(),
            error_message
        );
    }
}

#[tokio::test]
async fn post_link_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_user = create_test_user(&app.state)
        .await
        .expect("Failed to create test user");

    // Act
    let response = client
        .post(&format!("{}/api/links", &app.address))
        .header("Content-Type", "application/json")
        .header("X-Api-Token", &test_user.pak.to_string())
        .body(
            json!({
                "url": "http://www.example.com/",
                "title": "Example Site",
                "note": "Example note for example site",
            })
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let post_link_resp = response
        .json::<Value>()
        .await
        .expect("Failed to parse json body");
    assert_eq!(&post_link_resp["result"]["url"], "http://www.example.com/");
    assert_eq!(&post_link_resp["result"]["success"], true);
}

#[tokio::test]
async fn post_link_fails() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(&format!("{}/api/links", &app.address))
        .header("Content-Type", "application/json")
        .header("X-Api-Token", "lshelf_XXXXXX_XXXXXXXXXXX") // invalid token
        .body(
            json!({
                "url": "http://www.example.com/",
                "title": "Example Site",
                "note": "Example note for example site",
            })
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

async fn seed_links_for_user(user_id: &str, app_state: &AppState) {
    let links = vec![
        ("https://www.google.com", "Google", "Google search engine"),
        ("https://www.bing.com", "Bing", "Bing search engine"),
    ];
    let mut values: Vec<String> = vec![];

    for (link, title, note) in links {
        values.push(format!("('{link}', '{title}', '{note}', $user_id)"));
    }
    let values = values.join(", ");
    let query = format!("INSERT INTO link (url, title, note, user) VALUES {values};");

    app_state
        .db
        .query(query)
        .bind(("user_id", thing(user_id).unwrap()))
        .await
        .expect("Failed to seed links for user");
}

#[tokio::test]
async fn get_links_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_user = create_test_user(&app.state)
        .await
        .expect("Failed to create test user");

    // Seed some links for the test user in the DB
    seed_links_for_user(&test_user.id, &app.state).await;

    // Act
    let response = client
        .get(&format!("{}/api/links", &app.address))
        .header("Content-Type", "application/json")
        .header("X-Api-Token", &test_user.pak.to_string())
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let links_resp = response
        .json::<Vec<LinkResponse>>()
        .await
        .expect("Failed to parse json body");
    assert_eq!(links_resp.len(), 2);
}
