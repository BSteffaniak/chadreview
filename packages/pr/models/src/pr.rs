use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::user::{Label, User};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub description: String,
    pub author: User,
    pub state: PrState,
    pub draft: bool,
    pub base_branch: String,
    pub head_branch: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub reviewers: Vec<User>,
    pub head_sha: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrState {
    Open,
    Closed,
    Merged,
}
