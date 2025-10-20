use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::user::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub author: User,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_type: CommentType,
    pub replies: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommentType {
    General,
    FileLevelComment { path: String },
    LineLevelComment { path: String, line: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateComment {
    pub body: String,
    pub comment_type: CommentType,
    pub in_reply_to: Option<u64>,
}
