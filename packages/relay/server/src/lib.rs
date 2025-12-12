#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions, clippy::cargo_common_metadata)]

pub mod state;
pub mod webhook;
pub mod websocket;
mod ws;

use actix_web::{App, HttpServer, middleware, web};
use state::AppState;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub webhook_secret: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            webhook_secret: None,
        }
    }
}

impl ServerConfig {
    #[must_use]
    pub const fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            webhook_secret: None,
        }
    }

    #[must_use]
    pub const fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    #[must_use]
    pub fn with_host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    #[must_use]
    pub fn with_webhook_secret(mut self, secret: Option<String>) -> Self {
        self.webhook_secret = secret;
        self
    }
}

/// # Errors
///
/// Returns an error if the server fails to bind or run
#[allow(clippy::future_not_send)]
pub async fn run_server(config: ServerConfig) -> std::io::Result<()> {
    let RunServerResponse { join_handle, .. } = run_server_with_handle(&config)?;

    join_handle.await?
}

pub struct RunServerResponse {
    pub handle: actix_web::dev::ServerHandle,
    pub addrs: Vec<std::net::SocketAddr>,
    pub join_handle: JoinHandle<Result<(), std::io::Error>>,
}

/// # Errors
///
/// Returns an error if the server fails to bind
pub fn run_server_with_handle(config: &ServerConfig) -> std::io::Result<RunServerResponse> {
    log::info!(
        "Starting relay server on {}:{} (test mode)",
        config.host,
        config.port
    );

    let state = web::Data::new(AppState::new(config.webhook_secret.clone()));

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .route("/webhook", web::post().to(webhook::handler))
            .route("/ws/{instance_id}", web::get().to(websocket::handler))
            .route("/health", web::get().to(|| async { "OK" }))
    })
    .bind((config.host.as_str(), config.port))?;

    let addrs = server.addrs();
    let server = server.run();
    let handle = server.handle();

    let join_handle = tokio::spawn(server);

    Ok(RunServerResponse {
        handle,
        addrs,
        join_handle,
    })
}
