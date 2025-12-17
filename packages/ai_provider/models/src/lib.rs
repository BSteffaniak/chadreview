#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! AI provider models for `ChadReview`.
//!
//! This crate provides data structures for AI provider integration,
//! including context, responses, and action definitions.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export local_comment_models for types used in AI responses
pub use chadreview_local_comment_models as models;

/// Context passed to an AI provider for execution.
#[derive(Debug, Clone)]
pub struct AiContext {
    /// Repository root path.
    pub repo_path: PathBuf,
    /// Description of what's being diffed (e.g., "main..feature").
    pub diff_description: String,
    /// File path if this is a file/line comment.
    pub file_path: Option<String>,
    /// Line number if this is a line comment (e.g., "n42" or "o10").
    pub line: Option<String>,
    /// Code snippet around the commented line.
    pub diff_hunk: Option<String>,
    /// The user's comment body (their instruction/question).
    pub comment_body: String,
    /// Previous messages in the thread for context.
    pub thread_history: Vec<ThreadMessage>,
}

/// A message in a comment thread (for conversation context).
#[derive(Debug, Clone)]
pub struct ThreadMessage {
    /// Author name.
    pub author: String,
    /// Message body.
    pub body: String,
    /// Whether this is an AI response.
    pub is_ai_response: bool,
    /// When the message was sent.
    pub timestamp: DateTime<Utc>,
}

/// Definition of an available AI action/agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionDefinition {
    /// Unique identifier (e.g., "opencode:plan").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this agent does.
    pub description: String,
    /// Provider name (e.g., "opencode").
    pub provider: String,
    /// Default model for this agent.
    pub default_model: Option<String>,
    /// What this agent can do.
    pub capabilities: AgentCapabilities,
    /// Source of this agent definition.
    pub source: AgentSource,
}

/// Capabilities of an AI agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Can read files.
    pub can_read: bool,
    /// Can write/edit files.
    pub can_write: bool,
    /// Can execute shell commands.
    pub can_execute: bool,
}

/// Source of an agent definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentSource {
    /// From global config (e.g., ~/.config/opencode/opencode.json).
    GlobalConfig { path: PathBuf },
    /// From repo-specific config (e.g., .opencode/agents/foo.md).
    RepoConfig { path: PathBuf },
    /// Built-in default.
    BuiltIn,
}

/// Response from AI execution.
#[derive(Debug, Clone)]
pub struct AiResponse {
    /// The main response content (markdown).
    pub content: String,
    /// Model that was actually used.
    pub model_used: String,
    /// Execution details for transparency.
    pub execution_details: Option<models::ExecutionDetails>,
    /// Session ID for continuing conversations (provider-specific).
    /// For `OpenCode`, this is the session ID that can be passed to `--session`.
    pub session_id: Option<String>,
}

impl AiContext {
    /// Create a new AI context for a comment.
    #[must_use]
    pub const fn new(repo_path: PathBuf, diff_description: String, comment_body: String) -> Self {
        Self {
            repo_path,
            diff_description,
            file_path: None,
            line: None,
            diff_hunk: None,
            comment_body,
            thread_history: vec![],
        }
    }

    /// Set the file path.
    #[must_use]
    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Set the line number.
    #[must_use]
    pub fn with_line(mut self, line: String) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the diff hunk.
    #[must_use]
    pub fn with_diff_hunk(mut self, hunk: String) -> Self {
        self.diff_hunk = Some(hunk);
        self
    }

    /// Set the thread history.
    #[must_use]
    pub fn with_thread_history(mut self, history: Vec<ThreadMessage>) -> Self {
        self.thread_history = history;
        self
    }
}
