//! `OpenCode` CLI executor.

use std::fmt::Write;
use std::time::Instant;

use chrono::Utc;
use serde_json::Value;
use switchy::unsync::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use switchy::unsync::process::{Command, Stdio};
use switchy::unsync::task;
use switchy::unsync::time::{Duration, timeout};

use chadreview_ai_provider::{AiProviderError, mpsc};
use chadreview_ai_provider_models::{
    AiContext, AiResponse,
    models::{ExecutionDetails, TokenUsage, ToolExecution},
};
use chadreview_local_comment_models::{AiAction, ProgressEntry};

/// Executor for the `OpenCode` CLI.
pub struct OpenCodeExecutor<'a> {
    /// Path to the opencode binary.
    binary_path: &'a str,
}

impl<'a> OpenCodeExecutor<'a> {
    /// Create a new executor.
    #[must_use]
    pub const fn new(binary_path: &'a str) -> Self {
        Self { binary_path }
    }

    /// Build the prompt from context.
    ///
    /// When `is_continuation` is true (replying to an existing thread with a session),
    /// only the user's comment body is returned since `OpenCode` already has all the
    /// context from the previous conversation.
    #[must_use]
    pub fn build_prompt(context: &AiContext, is_continuation: bool) -> String {
        // For continuations, just send the user's message - OpenCode has the context
        if is_continuation {
            return context.comment_body.clone();
        }

        // For new conversations, include full context
        let mut prompt = String::new();

        // Context header
        write!(
            prompt,
            "You are helping review local code changes in a repository.\n\n\
             REPOSITORY: {}\n\
             DIFF: {}\n",
            context.repo_path.display(),
            context.diff_description,
        )
        .unwrap();

        // Specific code context (if line-level comment)
        if let (Some(path), Some(line)) = (&context.file_path, &context.line) {
            write!(
                prompt,
                "\nSPECIFIC CODE CONTEXT:\n\
                 - File: {path}\n\
                 - Line: {line}\n"
            )
            .unwrap();

            if let Some(hunk) = &context.diff_hunk {
                write!(prompt, "- Code snippet:\n```\n{hunk}\n```\n").unwrap();
            }
        }

        // Thread history (if this is part of a conversation)
        if !context.thread_history.is_empty() {
            prompt.push_str("\nTHREAD HISTORY (previous discussion):\n");
            for msg in &context.thread_history {
                let author = if msg.is_ai_response {
                    "AI"
                } else {
                    &msg.author
                };
                write!(
                    prompt,
                    "[@{} at {}]:\n{}\n\n",
                    author,
                    msg.timestamp.format("%Y-%m-%d %H:%M"),
                    msg.body
                )
                .unwrap();
            }
        }

        // User's request
        write!(prompt, "\nUSER'S REQUEST:\n{}\n\n", context.comment_body).unwrap();

        // Instructions
        prompt.push_str(
            "GUIDELINES:\n\
             1. Focus on the SPECIFIC code context provided, not the entire diff\n\
             2. If this is a QUESTION, explain clearly and concisely\n\
             3. If this is a COMMAND, implement the requested changes\n\
             4. Reference specific file paths and line numbers when relevant\n\
             5. Be thorough but concise\n",
        );

        prompt
    }

    /// Execute `OpenCode` CLI and capture output.
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
    #[allow(clippy::too_many_lines)]
    pub async fn execute(
        &self,
        context: &AiContext,
        action: &AiAction,
        session_id: Option<&str>,
        progress_tx: mpsc::Sender<ProgressEntry>,
    ) -> Result<AiResponse, AiProviderError> {
        let is_continuation = session_id.is_some();
        let prompt = Self::build_prompt(context, is_continuation);
        let start_time = Instant::now();

        // Get timeout from environment (default: no timeout)
        let timeout_secs: Option<u64> = std::env::var("OPENCODE_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok());

        log::info!(
            "Executing OpenCode: {} run --agent {} --format json{}(timeout: {})",
            self.binary_path,
            action.agent,
            session_id.map_or(String::new(), |s| format!(" --session {s} ")),
            timeout_secs.map_or_else(|| "none".to_string(), |s| format!("{s}s"))
        );

        // Build command
        let mut cmd = Command::new(self.binary_path);
        cmd.arg("run")
            .arg(&prompt)
            .arg("--agent")
            .arg(&action.agent)
            .arg("--format")
            .arg("json")
            .current_dir(&context.repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add session ID if continuing a conversation
        if let Some(sid) = session_id {
            cmd.arg("--session").arg(sid);
        }

        // Add model override if specified
        if let Some(model) = &action.model {
            cmd.arg("--model").arg(model);
        }

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            log::error!("Failed to spawn OpenCode process: {e}");
            AiProviderError::SpawnFailed(e.to_string())
        })?;

        log::debug!("OpenCode process spawned successfully");

        // Take stdout and stderr handles
        let stdout = child.stdout.take().ok_or_else(|| {
            AiProviderError::ExecutionFailed("Failed to capture stdout".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            AiProviderError::ExecutionFailed("Failed to capture stderr".to_string())
        })?;

        // Spawn task to collect stderr in background
        let stderr_handle = task::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr);
            let mut stderr_output = String::new();
            if let Err(e) = stderr_reader.read_to_string(&mut stderr_output).await {
                log::warn!("Failed to read stderr: {e}");
            }
            stderr_output
        });

        // State for collecting response
        let mut response_text = String::new();
        let mut tools_used = Vec::new();
        let mut tokens = TokenUsage::default();
        let mut cost = None;
        let model_used = action
            .model
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let mut captured_session_id: Option<String> = None;

        // Process stdout line-by-line
        let stdout_reader = BufReader::new(stdout);
        let mut lines = stdout_reader.lines();

        let process_lines = async {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }

                log::trace!("OpenCode output: {line}");

                let event: Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Failed to parse JSON line: {e} - line: {line}");
                        continue;
                    }
                };

                // Capture session ID from any event that has it
                if captured_session_id.is_none()
                    && let Some(sid) = event["sessionID"].as_str()
                {
                    captured_session_id = Some(sid.to_string());
                    log::debug!("Captured OpenCode session ID: {sid}");
                }

                match event["type"].as_str() {
                    Some("step_start") => {
                        // Acknowledge step_start - no action needed
                        log::debug!("OpenCode step_start");
                    }
                    Some("tool_use") => {
                        if let Some(part) = event.get("part") {
                            let tool = part["tool"].as_str().unwrap_or("unknown");
                            let title = part["state"]["title"].as_str().unwrap_or("").to_string();

                            log::debug!("OpenCode tool_use: {tool} - {title}");

                            // Send progress update
                            let entry = ProgressEntry {
                                tool: tool.to_string(),
                                title: title.clone(),
                                timestamp: Utc::now(),
                            };
                            let _ = progress_tx.send(entry);

                            // Record tool execution
                            tools_used.push(ToolExecution {
                                tool: tool.to_string(),
                                title,
                                input: part["state"]["input"].clone(),
                                output_preview: part["state"]["output"]
                                    .as_str()
                                    .map(|s| truncate_output(s, 500)),
                            });
                        }
                    }
                    Some("text") => {
                        if let Some(part) = event.get("part") {
                            // Only capture final text (has time.end)
                            if part["time"]["end"].is_number() {
                                response_text = part["text"].as_str().unwrap_or("").to_string();
                                log::debug!(
                                    "OpenCode text response: {} chars",
                                    response_text.len()
                                );
                            }
                        }
                    }
                    Some("step_finish") => {
                        if let Some(part) = event.get("part") {
                            tokens = TokenUsage {
                                input: part["tokens"]["input"].as_u64().unwrap_or(0),
                                output: part["tokens"]["output"].as_u64().unwrap_or(0),
                            };
                            cost = part["cost"].as_f64();
                            log::debug!(
                                "OpenCode step_finish: {} input tokens, {} output tokens",
                                tokens.input,
                                tokens.output
                            );
                        }
                    }
                    Some("error") => {
                        // Try to get the detailed error message from the API response first,
                        // then fall back to the error message, then the error name
                        let error_msg = event["error"]["data"]["message"]
                            .as_str()
                            .or_else(|| event["error"]["message"].as_str())
                            .or_else(|| event["error"]["name"].as_str())
                            .unwrap_or("Unknown error");
                        log::error!("OpenCode error event: {error_msg}");
                        return Err(AiProviderError::ExecutionFailed(error_msg.to_string()));
                    }
                    Some(other) => {
                        log::trace!("Unhandled OpenCode event type: {other}");
                    }
                    None => {
                        log::trace!("OpenCode event without type: {event:?}");
                    }
                }
            }
            Ok(())
        };

        // Apply timeout only if configured
        let inner_result = if let Some(secs) = timeout_secs {
            match timeout(Duration::from_secs(secs), process_lines).await {
                Ok(inner) => inner,
                Err(_elapsed) => {
                    log::error!("OpenCode execution timed out after {secs}s");
                    // Try to kill the process
                    if let Err(e) = child.kill().await {
                        log::warn!("Failed to kill OpenCode process: {e}");
                    }
                    return Err(AiProviderError::Timeout(secs));
                }
            }
        } else {
            process_lines.await
        };

        // Check for errors from the read loop
        inner_result?;

        // Wait for process to finish
        let status = child.wait().await?;
        let duration = start_time.elapsed();

        // Collect stderr
        let stderr_output = stderr_handle.await.unwrap_or_default();
        if !stderr_output.is_empty() {
            log::warn!("OpenCode stderr: {stderr_output}");
        }

        // Check exit status
        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            log::error!(
                "OpenCode process failed with exit code {exit_code}: stderr={stderr_output}"
            );
            return Err(AiProviderError::ProcessFailed {
                exit_code,
                stderr: stderr_output,
            });
        }

        log::info!(
            "OpenCode completed successfully: {} chars response, {} tools used, {:?} elapsed",
            response_text.len(),
            tools_used.len(),
            duration
        );

        Ok(AiResponse {
            content: response_text,
            model_used,
            execution_details: Some(ExecutionDetails {
                model_used: action
                    .model
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                tools_used,
                tokens,
                cost,
                duration_seconds: duration.as_secs(),
            }),
            session_id: captured_session_id,
        })
    }
}

/// Truncate output to a maximum length.
fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...(truncated)", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_prompt_basic() {
        let context = AiContext::new(
            PathBuf::from("/path/to/repo"),
            "main..feature".to_string(),
            "Please explain this code".to_string(),
        );

        let prompt = OpenCodeExecutor::build_prompt(&context, false);

        assert!(prompt.contains("/path/to/repo"));
        assert!(prompt.contains("main..feature"));
        assert!(prompt.contains("Please explain this code"));
    }

    #[test]
    fn test_build_prompt_with_file_context() {
        let context = AiContext::new(
            PathBuf::from("/path/to/repo"),
            "main..feature".to_string(),
            "What does this function do?".to_string(),
        )
        .with_file_path("src/main.rs".to_string())
        .with_line("n42".to_string())
        .with_diff_hunk("fn main() {\n    println!(\"Hello\");\n}".to_string());

        let prompt = OpenCodeExecutor::build_prompt(&context, false);

        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("n42"));
        assert!(prompt.contains("fn main()"));
    }

    #[test]
    fn test_truncate_output() {
        let short = "hello";
        assert_eq!(truncate_output(short, 10), "hello");

        let long = "hello world this is a long string";
        let truncated = truncate_output(long, 10);
        assert!(truncated.starts_with("hello worl"));
        assert!(truncated.ends_with("...(truncated)"));
    }
}
