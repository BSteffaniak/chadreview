#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

mod state;
mod webhook;
mod websocket;

use actix_web::{App, HttpServer, middleware, web};
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let state = web::Data::new(AppState::new());

    log::info!("Starting relay server on {host}:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .route("/webhook/{instance_id}", web::post().to(webhook::handler))
            .route("/ws/{instance_id}", web::get().to(websocket::handler))
            .route("/health", web::get().to(|| async { "OK" }))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
