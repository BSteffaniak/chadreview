use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use chadreview_diff_models::LineNumber;

use crate::user::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub author: User,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_type: CommentType,
    pub replies: Vec<Self>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "comment_type", rename_all = "snake_case")]
pub enum CommentType {
    General,
    FileLevelComment {
        path: String,
    },
    LineLevelComment {
        path: String,
        commit_sha: String,
        #[serde(flatten)]
        line: LineNumber,
    },
    Reply {
        root_comment_id: u64,
        in_reply_to: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateComment {
    pub body: String,
    #[serde(flatten)]
    pub comment_type: CommentType,
}
