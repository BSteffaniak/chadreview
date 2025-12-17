#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! `OpenCode` AI provider for `ChadReview`.
//!
//! This crate provides an implementation of the `AiProvider` trait
//! that integrates with the `OpenCode` CLI.

mod config;
mod executor;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use chadreview_ai_provider::{
    AiProvider, AiProviderError,
    models::{AiActionDefinition, AiContext, AiResponse},
    mpsc,
};
use chadreview_local_comment_models::{AiAction, ProgressEntry};

pub use config::{OpenCodeConfig, OpenCodeConfigError};
pub use executor::OpenCodeExecutor;

/// `OpenCode` AI provider implementation.
pub struct OpenCodeProvider {
    /// Path to the opencode binary.
    binary_path: String,
    /// Cached global config.
    config: Option<Arc<OpenCodeConfig>>,
}

impl OpenCodeProvider {
    /// Create a new `OpenCode` provider.
    ///
    /// Uses `OPENCODE_BINARY` environment variable if set,
    /// otherwise defaults to "opencode".
    #[must_use]
    pub fn new() -> Self {
        let binary_path =
            std::env::var("OPENCODE_BINARY").unwrap_or_else(|_| "opencode".to_string());

        Self {
            binary_path,
            config: None,
        }
    }

    /// Create with a specific binary path.
    #[must_use]
    pub const fn with_binary_path(binary_path: String) -> Self {
        Self {
            binary_path,
            config: None,
        }
    }

    /// Get or load the global config.
    fn get_config(&self) -> Result<Arc<OpenCodeConfig>, AiProviderError> {
        if let Some(config) = &self.config {
            return Ok(Arc::clone(config));
        }

        let config = OpenCodeConfig::load_global().map_err(|e| {
            AiProviderError::ConfigError(format!("Failed to load OpenCode config: {e}"))
        })?;

        Ok(Arc::new(config))
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AiProvider for OpenCodeProvider {
    fn provider_name(&self) -> &'static str {
        "opencode"
    }

    async fn list_agents(
        &self,
        _repo_path: &Path,
    ) -> Result<Vec<AiActionDefinition>, AiProviderError> {
        let config = self.get_config()?;
        Ok(config.to_action_definitions())
    }

    async fn execute(
        &self,
        context: &AiContext,
        action: &AiAction,
        session_id: Option<&str>,
        progress_tx: mpsc::Sender<ProgressEntry>,
    ) -> Result<AiResponse, AiProviderError> {
        let executor = OpenCodeExecutor::new(&self.binary_path);
        executor
            .execute(context, action, session_id, progress_tx)
            .await
    }
}
