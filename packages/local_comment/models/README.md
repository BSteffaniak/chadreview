# ChadReview Local Comment Models

Data models for local diff comments with AI integration support.

## Overview

This crate provides the data structures for managing comments on local git diffs:

- `LocalComment` - A comment with optional AI action and execution status
- `LocalCommentType` - Where the comment is attached (general, file, line, reply)
- `AiAction` - AI action specification (provider, agent, model)
- `AiExecutionStatus` - Status of AI execution (pending, running, completed, failed)
- `ExecutionDetails` - "How I worked on this" transparency information
- `ProgressEntry` - Real-time progress updates during AI execution

## Usage

```rust
use chadreview_local_comment_models::{
    LocalComment, LocalCommentType, LocalUser, AiAction, LineNumber,
};

// Create a line-level comment
let comment = LocalComment::new(
    LocalUser::default(),
    "Please refactor this function".to_string(),
    LocalCommentType::LineLevelComment {
        path: "src/main.rs".to_string(),
        line: LineNumber::New { line: 42 },
    },
);

// With AI action
let comment = comment.with_ai_action(AiAction {
    provider: "opencode".to_string(),
    agent: "build".to_string(),
    model: None,
    custom_instructions: None,
});
```
