use std::str::FromStr;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "side", rename_all = "snake_case")]
pub enum LineNumber {
    Old { line: u64 },
    New { line: u64 },
}

#[derive(thiserror::Error, Debug)]
pub struct ParseLineNumberError;

impl std::fmt::Display for ParseLineNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid LineNumber")
    }
}

impl FromStr for LineNumber {
    type Err = ParseLineNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(s) = s.strip_prefix('n') {
            Self::New {
                line: s.parse::<u64>().map_err(|_| ParseLineNumberError)?,
            }
        } else if let Some(s) = s.strip_prefix('o') {
            Self::Old {
                line: s.parse::<u64>().map_err(|_| ParseLineNumberError)?,
            }
        } else {
            return Err(ParseLineNumberError);
        })
    }
}

impl LineNumber {
    #[must_use]
    pub const fn number(&self) -> u64 {
        match self {
            Self::Old { line } | Self::New { line } => *line,
        }
    }
}

impl std::fmt::Display for LineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Old { line } => write!(f, "o{line}"),
            Self::New { line } => write!(f, "n{line}"),
        }
    }
}
