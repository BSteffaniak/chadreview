# ChadReview AI Provider

AI provider abstraction for ChadReview.

## Overview

This crate provides the `AiProvider` trait that AI integrations implement.

## The AiProvider Trait

```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Get the provider name.
    fn provider_name(&self) -> &'static str;

    /// List available actions/agents for the given repository.
    async fn list_agents(&self, repo_path: &Path) -> Result<Vec<AiActionDefinition>, AiProviderError>;

    /// Execute an AI action with the given context.
    async fn execute(
        &self,
        context: &AiContext,
        action: &AiAction,
        progress_tx: mpsc::Sender<ProgressEntry>,
    ) -> Result<AiResponse, AiProviderError>;

    /// Format execution details as markdown.
    fn format_execution_details(&self, details: &ExecutionDetails) -> String;
}
```

## Progress Updates

The `execute` method receives a channel for sending progress updates.
This enables real-time UI updates via HyperChad SSE:

```rust
// Send progress during execution
progress_tx.send(ProgressEntry {
    tool: "bash".to_string(),
    title: "Running tests".to_string(),
    timestamp: Utc::now(),
}).ok();
```
