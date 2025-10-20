use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub avatar_url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: User,
    pub committed_at: DateTime<Utc>,
}
