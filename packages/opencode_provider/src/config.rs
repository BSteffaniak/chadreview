//! `OpenCode` configuration parsing.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use chadreview_ai_provider_models::{AgentCapabilities, AgentSource, AiActionDefinition};

/// Errors that can occur when loading `OpenCode` config.
#[derive(Debug, thiserror::Error)]
pub enum OpenCodeConfigError {
    /// Config file not found.
    #[error("OpenCode config not found")]
    NotFound,

    /// Failed to read config file.
    #[error("Failed to read config: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse config file.
    #[error("Failed to parse config: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Parsed opencode.json structure.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeConfig {
    /// Agent configurations.
    #[serde(default)]
    pub agent: HashMap<String, AgentConfig>,

    /// Path this config was loaded from.
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

/// Configuration for a single agent.
#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    /// Agent mode (e.g., "primary").
    pub mode: Option<String>,
    /// Default model for this agent.
    pub model: Option<String>,
    /// Tool permissions.
    #[serde(default)]
    pub tools: ToolsConfig,
    /// System prompt.
    pub system_prompt: Option<String>,
}

/// Tool permission configuration.
#[derive(Debug, Default, Deserialize)]
pub struct ToolsConfig {
    /// Can write files.
    #[serde(default)]
    pub write: bool,
    /// Can edit files.
    #[serde(default)]
    pub edit: bool,
    /// Can run bash commands.
    #[serde(default)]
    pub bash: bool,
}

impl OpenCodeConfig {
    /// Find and load the global `OpenCode` config.
    ///
    /// Searches in order:
    /// 1. `$XDG_CONFIG_HOME/opencode/opencode.json`
    /// 2. `~/.config/opencode/opencode.json`
    /// 3. `~/.config/nixos/configs/opencode/opencode.json`
    ///
    /// # Errors
    ///
    /// Returns an error if no config is found or parsing fails.
    pub fn load_global() -> Result<Self, OpenCodeConfigError> {
        let candidates = Self::config_candidates();

        for path in candidates.into_iter().flatten() {
            if path.exists() {
                log::debug!("Loading OpenCode config from: {}", path.display());
                return Self::load_from_path(&path);
            }
        }

        Err(OpenCodeConfigError::NotFound)
    }

    /// Get candidate paths for the config file.
    fn config_candidates() -> Vec<Option<PathBuf>> {
        vec![
            // XDG config home
            dirs::config_dir().map(|p| p.join("opencode/opencode.json")),
            // Standard ~/.config
            dirs::home_dir().map(|p| p.join(".config/opencode/opencode.json")),
            // NixOS-style config
            dirs::home_dir().map(|p| p.join(".config/nixos/configs/opencode/opencode.json")),
        ]
    }

    /// Load config from a specific path.
    fn load_from_path(path: &PathBuf) -> Result<Self, OpenCodeConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Self = serde_json::from_str(&content)?;
        config.config_path = Some(path.clone());
        Ok(config)
    }

    /// Convert agents to action definitions.
    #[must_use]
    pub fn to_action_definitions(&self) -> Vec<AiActionDefinition> {
        self.agent
            .iter()
            .map(|(name, config)| {
                let description = config.mode.as_ref().map_or_else(
                    || format!("{name} agent"),
                    |mode| format!("{name} ({mode})"),
                );

                AiActionDefinition {
                    id: format!("opencode:{name}"),
                    name: name.clone(),
                    description,
                    provider: "opencode".to_string(),
                    default_model: config.model.clone(),
                    capabilities: AgentCapabilities {
                        can_read: true, // All agents can read
                        can_write: config.tools.write || config.tools.edit,
                        can_execute: config.tools.bash,
                    },
                    source: AgentSource::GlobalConfig {
                        path: self.config_path.clone().unwrap_or_default(),
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "agent": {
                "plan": {
                    "mode": "primary",
                    "model": "anthropic/claude-sonnet-4-20250514",
                    "tools": {
                        "write": false,
                        "edit": false,
                        "bash": false
                    }
                },
                "build": {
                    "mode": "primary",
                    "model": "anthropic/claude-sonnet-4-20250514",
                    "tools": {
                        "write": true,
                        "edit": true,
                        "bash": true
                    }
                }
            }
        }"#;

        let config: OpenCodeConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.agent.len(), 2);
        assert!(config.agent.contains_key("plan"));
        assert!(config.agent.contains_key("build"));

        let build = &config.agent["build"];
        assert!(build.tools.write);
        assert!(build.tools.edit);
        assert!(build.tools.bash);
    }

    #[test]
    fn test_to_action_definitions() {
        let json = r#"{
            "agent": {
                "plan": {
                    "mode": "planning",
                    "model": "anthropic/claude-sonnet-4-20250514"
                }
            }
        }"#;

        let config: OpenCodeConfig = serde_json::from_str(json).unwrap();
        let actions = config.to_action_definitions();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "opencode:plan");
        assert_eq!(actions[0].name, "plan");
        assert_eq!(actions[0].provider, "opencode");
    }
}
