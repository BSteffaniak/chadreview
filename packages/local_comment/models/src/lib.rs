#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Local comment models for `ChadReview`.
//!
//! This crate provides data structures for comments on local git diffs,
//! including AI action integration support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use switchy::uuid::Uuid;

pub use chadreview_diff_models::LineNumber;

/// State of a comment thread.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThreadState {
    /// Thread is open and active.
    #[default]
    Open,
    /// Thread has been resolved.
    Resolved,
    /// Thread is saved for later review.
    SavedForLater,
}

impl ThreadState {
    /// Returns true if the thread should be collapsed by default.
    #[must_use]
    pub const fn is_collapsed(&self) -> bool {
        matches!(self, Self::Resolved | Self::SavedForLater)
    }
}

/// A comment on a local diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalComment {
    /// Unique identifier for this comment.
    pub id: Uuid,
    /// Author of the comment.
    pub author: LocalUser,
    /// The comment body (markdown).
    pub body: String,
    /// When the comment was created.
    pub created_at: DateTime<Utc>,
    /// When the comment was last updated.
    pub updated_at: DateTime<Utc>,
    /// Type/location of the comment.
    pub comment_type: LocalCommentType,
    /// Nested replies to this comment.
    pub replies: Vec<Self>,
    /// State of this comment thread (open, resolved, or saved for later).
    pub state: ThreadState,
    /// AI action to execute for this comment (if any).
    pub ai_action: Option<AiAction>,
    /// Current status of AI execution (if an AI action was specified).
    pub ai_status: Option<AiExecutionStatus>,
    /// `OpenCode` session ID for continuing conversations.
    /// Only set on root threads (not replies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_session_id: Option<String>,
}

/// Type of comment indicating where it is attached.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocalCommentType {
    /// General comment not attached to a specific file or line.
    General,
    /// Comment attached to a file but not a specific line.
    FileLevelComment {
        /// Path to the file.
        path: String,
    },
    /// Comment attached to a specific line in a file.
    LineLevelComment {
        /// Path to the file.
        path: String,
        /// Line number (old or new side).
        #[serde(flatten)]
        line: LineNumber,
    },
    /// Reply to another comment.
    Reply {
        /// ID of the root comment in the thread.
        root_comment_id: Uuid,
        /// ID of the comment being replied to.
        in_reply_to: Uuid,
    },
}

/// AI action specification - provider-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiAction {
    /// Provider identifier (e.g., "opencode").
    pub provider: String,
    /// Agent/action name (e.g., "plan", "build").
    pub agent: String,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional custom instructions to append to the prompt.
    pub custom_instructions: Option<String>,
}

/// Status of AI execution for a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AiExecutionStatus {
    /// AI execution is queued but not started.
    Pending,
    /// AI execution is in progress.
    Running {
        /// When execution started.
        started_at: DateTime<Utc>,
        /// Progress entries (tool calls, etc.) so far.
        progress: Vec<ProgressEntry>,
    },
    /// AI execution completed successfully.
    Completed {
        /// When execution finished.
        finished_at: DateTime<Utc>,
        /// ID of the reply comment containing the AI response.
        response_comment_id: Uuid,
        /// Execution details for "How I worked on this" section.
        execution_details: Option<ExecutionDetails>,
    },
    /// AI execution failed.
    Failed {
        /// When execution finished.
        finished_at: DateTime<Utc>,
        /// Error message.
        error: String,
    },
}

/// A progress entry during AI execution (e.g., tool call).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// Tool/action name (e.g., "bash", "read", "edit").
    pub tool: String,
    /// Human-readable title/description.
    pub title: String,
    /// When this entry occurred.
    pub timestamp: DateTime<Utc>,
}

/// Execution details for transparency ("How I worked on this").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetails {
    /// Model that was used.
    pub model_used: String,
    /// Tools used during execution.
    pub tools_used: Vec<ToolExecution>,
    /// Token usage statistics.
    pub tokens: TokenUsage,
    /// Cost in dollars (if available).
    pub cost: Option<f64>,
    /// Total execution duration in seconds.
    pub duration_seconds: u64,
}

/// Record of a tool execution during AI processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    /// Tool name.
    pub tool: String,
    /// Human-readable title.
    pub title: String,
    /// Input parameters (as JSON).
    pub input: serde_json::Value,
    /// Output preview (truncated if large).
    pub output_preview: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
}

/// Local user identity (from git config).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    /// User's name.
    pub name: String,
    /// User's email.
    pub email: String,
}

impl Default for LocalUser {
    fn default() -> Self {
        Self {
            name: "Anonymous".to_string(),
            email: "anonymous@local".to_string(),
        }
    }
}

/// Index entry for efficient thread listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentThreadIndex {
    /// Thread ID (same as root comment ID).
    pub id: Uuid,
    /// Type of the root comment.
    pub comment_type: LocalCommentType,
    /// When the thread was created.
    pub created_at: DateTime<Utc>,
    /// When the thread was last updated.
    pub updated_at: DateTime<Utc>,
    /// Number of replies in the thread.
    pub reply_count: usize,
    /// State of the thread (open, resolved, or saved for later).
    pub state: ThreadState,
    /// Whether the root comment has an AI action.
    pub has_ai_action: bool,
    /// Summary of AI execution status.
    pub ai_status_summary: Option<AiExecutionStatusSummary>,
}

/// Simplified AI status for index/listing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiExecutionStatusSummary {
    Pending,
    Running,
    Completed,
    Failed,
}

impl From<&AiExecutionStatus> for AiExecutionStatusSummary {
    fn from(status: &AiExecutionStatus) -> Self {
        match status {
            AiExecutionStatus::Pending => Self::Pending,
            AiExecutionStatus::Running { .. } => Self::Running,
            AiExecutionStatus::Completed { .. } => Self::Completed,
            AiExecutionStatus::Failed { .. } => Self::Failed,
        }
    }
}

/// Data for creating a new local comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocalComment {
    /// Comment body (markdown).
    pub body: String,
    /// Type/location of the comment.
    #[serde(flatten)]
    pub comment_type: LocalCommentType,
    /// Optional AI action to execute.
    pub ai_action: Option<AiAction>,
}

impl LocalComment {
    /// Create a new local comment.
    #[must_use]
    pub fn new(author: LocalUser, body: String, comment_type: LocalCommentType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            author,
            body,
            created_at: now,
            updated_at: now,
            comment_type,
            replies: vec![],
            state: ThreadState::default(),
            ai_action: None,
            ai_status: None,
            opencode_session_id: None,
        }
    }

    /// Create a new comment with an AI action.
    #[must_use]
    pub fn with_ai_action(mut self, action: AiAction) -> Self {
        self.ai_action = Some(action);
        self.ai_status = Some(AiExecutionStatus::Pending);
        self
    }

    /// Count total replies recursively.
    #[must_use]
    pub fn count_replies(&self) -> usize {
        let mut count = self.replies.len();
        for reply in &self.replies {
            count += reply.count_replies();
        }
        count
    }

    /// Create an index entry for this comment.
    #[must_use]
    pub fn to_index_entry(&self) -> CommentThreadIndex {
        CommentThreadIndex {
            id: self.id,
            comment_type: self.comment_type.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            reply_count: self.count_replies(),
            state: self.state,
            has_ai_action: self.ai_action.is_some(),
            ai_status_summary: self.ai_status.as_ref().map(AiExecutionStatusSummary::from),
        }
    }
}

// =============================================================================
// Viewed Files
// =============================================================================

/// Index of files that have been marked as viewed.
///
/// This is stored separately from comments and tracks which files
/// the user has reviewed in a diff.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewedFilesIndex {
    /// Map of file path to when it was marked as viewed.
    #[serde(default)]
    pub files: std::collections::HashMap<String, DateTime<Utc>>,
}

// =============================================================================
// Viewed Replies
// =============================================================================

/// Index of replies that have been marked as viewed.
///
/// This is stored separately from comments and tracks which replies
/// the user has reviewed in a thread.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewedRepliesIndex {
    /// Map of reply comment ID to when it was marked as viewed.
    #[serde(default)]
    pub replies: std::collections::HashMap<Uuid, DateTime<Utc>>,
}
