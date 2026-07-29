mod auth;
mod config;
mod db;
mod engine_client;
mod error;
mod projects;
mod ui;

use std::env;
use std::sync::Arc;

use axum::extract::FromRef;
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

use auth::JwtVerifier;
use engine_client::EngineClient;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    jwt_verifier: Arc<JwtVerifier>,
    engine: Arc<EngineClient>,
}

impl FromRef<AppState> for Arc<JwtVerifier> {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_verifier.clone()
    }
}

fn engine_base_url() -> String {
    env::var("OPTISTOR_ENGINE_URL").unwrap_or_else(|_| "http://127.0.0.1:8001".to_string())
}

fn supabase_url() -> String {
    env::var("OPTISTOR_SUPABASE_URL")
        .unwrap_or_else(|_| "https://fyqulandxyicawmvquxg.supabase.co".to_string())
}

fn database_url() -> String {
    env::var("OPTISTOR_DATABASE_URL").unwrap_or_else(|_| "sqlite://optistor.db".to_string())
}

/// Supabase's publishable/anon key — meant to be public, embedded straight
/// into the login page's client-side JS (see templates/login.html).
fn supabase_publishable_key() -> String {
    env::var("OPTISTOR_SUPABASE_PUBLISHABLE_KEY")
        .unwrap_or_else(|_| "sb_publishable_LeRItnaGFYDsg_-z0-T5vQ_xPdZlx5L".to_string())
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "optistor-server" }))
}

async fn engine_health() -> Json<Value> {
    let url = format!("{}/health", engine_base_url());

    match reqwest::get(&url).await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(body) => Json(json!({ "server": "ok", "engine": body })),
            Err(err) => Json(json!({ "server": "ok", "engine": "error", "detail": err.to_string() })),
        },
        Err(err) => Json(json!({ "server": "ok", "engine": "unreachable", "detail": err.to_string() })),
    }
}

#[tokio::main]
async fn main() {
    let db = db::connect(&database_url()).await.expect("failed to connect to database");
    let jwt_verifier = Arc::new(JwtVerifier::new(&supabase_url()));
    let engine = Arc::new(EngineClient::new(engine_base_url()));
    let state = AppState { db, jwt_verifier, engine };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/engine/health", get(engine_health))
        .merge(projects::router())
        .merge(ui::router())
        .with_state(state)
        .nest_service("/static", ServeDir::new("static"));

    // 0.0.0.0 so the server is reachable from outside its container in
    // Docker/Fly.io — binding to 127.0.0.1 works locally but is invisible
    // to anything outside the container's own network namespace.
    let bind_addr = env::var("OPTISTOR_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener =
        tokio::net::TcpListener::bind(&bind_addr).await.expect("failed to bind address");

    println!("optistor-server listening on http://{bind_addr}");

    axum::serve(listener, app).await.expect("server error");
}
