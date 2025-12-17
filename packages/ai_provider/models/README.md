# ChadReview AI Provider Models

Data models for AI provider integration.

## Overview

This crate provides the data structures for AI provider abstraction:

- `AiContext` - Context passed to AI for execution (repo, file, line, comment)
- `AiResponse` - Response from AI execution
- `AiActionDefinition` - Definition of an available AI action/agent
- `AgentCapabilities` - What an agent can do (read, write, execute)
- `AgentSource` - Where the agent definition came from

## Usage

```rust
use chadreview_ai_provider_models::{AiContext, AiActionDefinition, AgentCapabilities};
use std::path::PathBuf;

// Create context for AI execution
let context = AiContext::new(
    PathBuf::from("/path/to/repo"),
    "main..feature".to_string(),
    "Please refactor this function".to_string(),
)
.with_file_path("src/main.rs".to_string())
.with_line("n42".to_string());
```
