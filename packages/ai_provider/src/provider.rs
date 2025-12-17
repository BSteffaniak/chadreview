//! AI provider trait definition.

use std::fmt::Write;
use std::path::Path;

use async_trait::async_trait;
use switchy::unsync::sync::mpsc;

use chadreview_ai_provider_models::{
    AiActionDefinition, AiContext, AiResponse, models::ExecutionDetails,
};
use chadreview_local_comment_models::{AiAction, ProgressEntry};

/// Errors that can occur when using an AI provider.
#[derive(Debug, thiserror::Error)]
pub enum AiProviderError {
    /// Provider not found.
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// Agent not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to spawn the AI process.
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    /// Process exited with non-zero status.
    #[error("Process failed with exit code {exit_code}: {stderr}")]
    ProcessFailed { exit_code: i32, stderr: String },

    /// Execution timed out.
    #[error("Execution timed out after {0} seconds")]
    Timeout(u64),

    /// Failed to parse output.
    #[error("Failed to parse output: {0}")]
    ParseError(String),
}

/// Trait for AI provider implementations.
///
/// Providers implement this trait to integrate with different AI systems
/// (e.g., `OpenCode`, Claude API, etc.).
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Get the provider name.
    fn provider_name(&self) -> &'static str;

    /// List available actions/agents for the given repository.
    ///
    /// This may include both global agents (from user config) and
    /// repo-specific agents (from .opencode/agents/ or similar).
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be read.
    async fn list_agents(
        &self,
        repo_path: &Path,
    ) -> Result<Vec<AiActionDefinition>, AiProviderError>;

    /// Execute an AI action with the given context.
    ///
    /// Progress updates should be sent through the provided channel.
    /// The caller can use these to update the UI in real-time.
    ///
    /// # Arguments
    ///
    /// * `context` - The context for execution (repo, file, comment, etc.)
    /// * `action` - The action to execute (agent, model, etc.)
    /// * `session_id` - Optional session ID to continue a previous conversation
    /// * `progress_tx` - Channel to send progress updates
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    async fn execute(
        &self,
        context: &AiContext,
        action: &AiAction,
        session_id: Option<&str>,
        progress_tx: mpsc::Sender<ProgressEntry>,
    ) -> Result<AiResponse, AiProviderError>;

    /// Format execution details as markdown ("How I worked on this").
    ///
    /// This is used to display transparency information in the UI.
    fn format_execution_details(&self, details: &ExecutionDetails) -> String {
        let mut output = String::new();

        output.push_str("## How I worked on this\n\n");

        write!(output, "**Model:** {}\n\n", details.model_used).unwrap();

        if !details.tools_used.is_empty() {
            output.push_str("<details>\n<summary>Tools used</summary>\n\n");
            for tool in &details.tools_used {
                writeln!(output, "- **{}**: {}", tool.tool, tool.title).unwrap();
            }
            output.push_str("\n</details>\n\n");
        }

        writeln!(
            output,
            "**Tokens:** {} input, {} output",
            details.tokens.input, details.tokens.output
        )
        .unwrap();

        if let Some(cost) = details.cost {
            writeln!(output, "**Cost:** ${cost:.4}").unwrap();
        }

        writeln!(output, "**Duration:** {}s", details.duration_seconds).unwrap();

        output
    }
}
