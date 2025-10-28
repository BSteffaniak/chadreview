use chadreview_relay_models::PrKey;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};

pub type MessageSender = mpsc::UnboundedSender<String>;

pub struct AppState {
    pub connections: RwLock<HashMap<String, Vec<MessageSender>>>,
    pub subscriptions: RwLock<HashMap<PrKey, Vec<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_connection(&self, instance_id: String, sender: MessageSender) {
        self.connections
            .write()
            .await
            .entry(instance_id)
            .or_default()
            .push(sender);
    }

    pub async fn remove_connection(&self, instance_id: &str) {
        self.connections.write().await.remove(instance_id);

        let mut subs = self.subscriptions.write().await;
        subs.retain(|_, instances| {
            instances.retain(|id| id != instance_id);
            !instances.is_empty()
        });
    }

    pub async fn subscribe(&self, instance_id: String, pr_key: PrKey) {
        self.subscriptions
            .write()
            .await
            .entry(pr_key)
            .or_default()
            .push(instance_id);
    }

    pub async fn unsubscribe(&self, instance_id: &str, pr_key: &PrKey) {
        if let Some(instances) = self.subscriptions.write().await.get_mut(pr_key) {
            instances.retain(|id| id != instance_id);
        }
    }

    pub async fn get_subscribed_instances(&self, pr_key: &PrKey) -> Vec<String> {
        self.subscriptions
            .read()
            .await
            .get(pr_key)
            .cloned()
            .unwrap_or_default()
    }
}
