#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use chadreview_relay_server::{ServerConfig, run_server};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let webhook_secret = std::env::var("GITHUB_WEBHOOK_SECRET").ok();

    let config = ServerConfig::new(host, port).with_webhook_secret(webhook_secret);
    run_server(config).await
}
