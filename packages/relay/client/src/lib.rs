#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use anyhow::Result;
use chadreview_relay_models::{
    ClientMessage, PrKey, RelayMessage, ServerMessage, SubscribeMessage, UnsubscribeMessage,
    WebhookEvent,
};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub type EventCallback = Arc<dyn Fn(WebhookEvent) + Send + Sync>;

pub struct RelayClient {
    instance_id: String,
    relay_url: String,
    subscriptions: Arc<RwLock<HashMap<PrKey, EventCallback>>>,
    sender: Arc<RwLock<Option<futures::channel::mpsc::UnboundedSender<Message>>>>,
    ready: Arc<tokio::sync::Notify>,
    pending_confirmations: Arc<RwLock<HashMap<PrKey, tokio::sync::oneshot::Sender<()>>>>,
}

impl RelayClient {
    /// Connect to the relay server
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established
    pub async fn connect_async(relay_url: &str, instance_id: String) -> Result<Arc<Self>> {
        let client = Arc::new(Self {
            instance_id,
            relay_url: relay_url.to_string(),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            sender: Arc::new(RwLock::new(None)),
            ready: Arc::new(tokio::sync::Notify::new()),
            pending_confirmations: Arc::new(RwLock::new(HashMap::new())),
        });

        let notified = client.ready.notified();
        client.clone().spawn_connection_loop();
        notified.await;

        Ok(client)
    }

    /// Connect to the relay server
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established
    pub fn connect(relay_url: &str, instance_id: String) -> Result<Arc<Self>> {
        let client = Arc::new(Self {
            instance_id,
            relay_url: relay_url.to_string(),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            sender: Arc::new(RwLock::new(None)),
            ready: Arc::new(tokio::sync::Notify::new()),
            pending_confirmations: Arc::new(RwLock::new(HashMap::new())),
        });

        client.clone().spawn_connection_loop();

        Ok(client)
    }

    /// Subscribe to PR webhook events
    ///
    /// # Errors
    /// Returns an error if the subscription message cannot be sent
    pub async fn subscribe(&self, pr_key: PrKey, callback: EventCallback) -> Result<()> {
        self.subscriptions
            .write()
            .await
            .insert(pr_key.clone(), callback);

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_confirmations
            .write()
            .await
            .insert(pr_key.clone(), tx);

        self.send_message(ClientMessage::Subscribe(SubscribeMessage { pr_key }))
            .await?;

        rx.await?;

        Ok(())
    }

    /// Unsubscribe from PR webhook events
    ///
    /// # Errors
    /// Returns an error if the unsubscribe message cannot be sent
    pub async fn unsubscribe(&self, pr_key: &PrKey) -> Result<()> {
        self.subscriptions.write().await.remove(pr_key);

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_confirmations
            .write()
            .await
            .insert(pr_key.clone(), tx);

        self.send_message(ClientMessage::Unsubscribe(UnsubscribeMessage {
            pr_key: pr_key.clone(),
        }))
        .await?;

        rx.await?;

        Ok(())
    }

    async fn send_message(&self, msg: ClientMessage) -> Result<()> {
        let json = serde_json::to_string(&msg)?;
        if let Some(sender) = self.sender.read().await.as_ref() {
            sender.unbounded_send(Message::Text(json.into()))?;
        }
        Ok(())
    }

    fn spawn_connection_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.clone().connection_loop().await {
                    log::error!("Connection error: {e}");
                }

                log::info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    async fn connection_loop(self: Arc<Self>) -> Result<()> {
        let url = format!("{}/ws/{}", self.relay_url, self.instance_id);
        log::info!("Connecting to relay server at {url}");

        let (ws_stream, _) = connect_async(&url).await?;
        log::info!("Connected to relay server");

        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = futures::channel::mpsc::unbounded::<Message>();
        *self.sender.write().await = Some(tx);
        self.ready.notify_waiters();

        let write_task = {
            tokio::spawn(async move {
                while let Some(msg) = rx.next().await {
                    if write.send(msg).await.is_err() {
                        break;
                    }
                }
            })
        };

        let read_task = {
            let subscriptions = self.subscriptions.clone();
            let sender = self.sender.clone();
            let pending_confirmations = self.pending_confirmations.clone();

            tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                match server_msg {
                                    ServerMessage::Webhook(relay_msg) => {
                                        Self::handle_webhook_event(&subscriptions, *relay_msg)
                                            .await;
                                    }
                                    ServerMessage::Pong => {
                                        log::trace!("Received pong");
                                    }
                                    ServerMessage::Subscribed { pr_key } => {
                                        log::info!("Subscribed to {pr_key:?}");
                                        let mut confirmations = pending_confirmations.write().await;
                                        if let Some(tx) = confirmations.remove(&pr_key) {
                                            let _ = tx.send(());
                                        }
                                    }
                                    ServerMessage::Unsubscribed { pr_key } => {
                                        log::info!("Unsubscribed from {pr_key:?}");
                                        let mut confirmations = pending_confirmations.write().await;
                                        if let Some(tx) = confirmations.remove(&pr_key) {
                                            let _ = tx.send(());
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            log::info!("Server closed connection");
                            break;
                        }
                        Err(e) => {
                            log::error!("WebSocket error: {e}");
                            break;
                        }
                        _ => {}
                    }
                }

                *sender.write().await = None;
            })
        };

        tokio::select! {
            _ = write_task => {},
            _ = read_task => {},
        }

        Ok(())
    }

    async fn handle_webhook_event(
        subscriptions: &Arc<RwLock<HashMap<PrKey, EventCallback>>>,
        relay_msg: RelayMessage,
    ) {
        let subs = subscriptions.read().await;
        if let Some(callback) = subs.get(&relay_msg.pr_key) {
            callback(relay_msg.event);
        }
    }

    #[must_use]
    pub fn get_or_create_instance_id() -> String {
        let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_path = config_dir.join("chadreview/instance_id");

        std::fs::read_to_string(&config_path).map_or_else(
            |_| {
                let id = uuid::Uuid::new_v4().to_string();
                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&config_path, &id);
                id
            },
            |id| id.trim().to_string(),
        )
    }
}
