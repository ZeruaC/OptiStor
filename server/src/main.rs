use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::env;

fn engine_base_url() -> String {
    env::var("OPTISTOR_ENGINE_URL").unwrap_or_else(|_| "http://127.0.0.1:8001".to_string())
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
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/engine/health", get(engine_health));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("failed to bind port 8000");

    println!("optistor-server listening on http://127.0.0.1:8000");

    axum::serve(listener, app).await.expect("server error");
}
