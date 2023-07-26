use crate::netem::{NetEm, Output};
use axum::http::Uri;
use axum::routing::{get, post};
use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use log::LevelFilter;
use std::net::SocketAddr;

mod netem;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = 3000;
    let log_level = LevelFilter::Info;

    env_logger::builder().filter_level(log_level).try_init()?;

    let app = Router::new()
        .route("/api", post(api))
        .fallback(get(fallback));
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("No route for {}", uri))
}

async fn api(Json(netem): Json<NetEm>) -> Json<Output> {
    Json(netem.execute().await)
}
