use chadreview_relay_server::{ServerConfig, run_server_with_handle};

pub struct TestRelayServer {
    port: u16,
    http_url: String,
    ws_url: String,
    handle: actix_web::dev::ServerHandle,
}

impl TestRelayServer {
    /// # Errors
    ///
    /// Returns an error if the server fails to start or no ports are available
    pub async fn start() -> anyhow::Result<Self> {
        Self::start_with_secret(None).await
    }

    /// # Errors
    ///
    /// Returns an error if the server fails to start or no ports are available
    pub async fn start_with_secret(webhook_secret: Option<String>) -> anyhow::Result<Self> {
        let config =
            ServerConfig::new("127.0.0.1".to_string(), 0).with_webhook_secret(webhook_secret);

        let response = run_server_with_handle(&config)?;
        let port = response
            .addrs
            .first()
            .expect("Expected at least one address")
            .port();
        let http_url = format!("http://127.0.0.1:{port}");
        let ws_url = format!("ws://127.0.0.1:{port}");

        wait_for_server_ready(&http_url).await?;

        Ok(Self {
            port,
            http_url,
            ws_url,
            handle: response.handle,
        })
    }

    #[must_use]
    pub fn http_url(&self) -> &str {
        &self.http_url
    }

    #[must_use]
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestRelayServer {
    fn drop(&mut self) {
        let handle = self.handle.clone();
        tokio::spawn(async move {
            handle.stop(true).await;
        });
    }
}

async fn wait_for_server_ready(url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let health_url = format!("{url}/health");

    for _ in 0..30 {
        if let Ok(response) = client.get(&health_url).send().await
            && response.status().is_success()
        {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    anyhow::bail!("Server failed to start within timeout")
}
