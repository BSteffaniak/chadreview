//! Local git diff routes.
//!
//! Routes for viewing local git diffs without GitHub integration,
//! including local commenting with AI integration.

use std::path::PathBuf;
use std::sync::Arc;

use chadreview_app_ui::{diff_viewer::local as diff_viewer, local_comments, local_header};
use chadreview_git_backend::GitBackend;
use chadreview_local_comment::LocalCommentStore;
use chadreview_local_comment_models::{
    AiAction, AiExecutionStatus, LocalComment, LocalCommentType, LocalUser, ProgressEntry,
    ThreadState,
};
use chadreview_local_diff::LocalDiffProvider;
use chadreview_local_diff_models::{DiffSpec, DiffSpecError};
use chrono::Utc;
use hyperchad::{
    router::{Container, RouteRequest, Router},
    template::container,
};
use switchy::http::models::Method;
use switchy::uuid::Uuid;

use crate::sse::{push_ai_status_update, push_thread_replies};

// Conditional imports for AI integration
#[cfg(feature = "ai-integration-opencode")]
use chadreview_ai_provider::AiProvider;
#[cfg(feature = "ai-integration-opencode")]
use chadreview_ai_provider_models::AiContext;
#[cfg(feature = "ai-integration-opencode")]
use chadreview_opencode_provider::OpenCodeProvider;

/// Error type for local route operations.
#[derive(Debug, thiserror::Error)]
pub enum LocalRouteError {
    /// The request method is not supported for this route.
    #[error("Unsupported method")]
    UnsupportedMethod,
    /// Error parsing diff specification from query parameters.
    #[error("Invalid diff spec: {0}")]
    DiffSpec(#[from] DiffSpecError),
    /// Error from the git backend.
    #[error("Git error: {0}")]
    Git(#[from] anyhow::Error),
    /// Invalid repository path.
    #[error("Invalid repository path: {0}")]
    InvalidRepoPath(String),
    /// Comment store error.
    #[error("Comment store error: {0}")]
    CommentStore(#[from] chadreview_local_comment::LocalCommentStoreError),
    /// Missing required parameter.
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    /// Invalid UUID.
    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),
    /// Comment not found.
    #[error("Comment not found: {0}")]
    CommentNotFound(Uuid),
    /// JSON parse error.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    /// Form parse error.
    #[error("Invalid form body: {0}")]
    InvalidBody(#[from] hyperchad::router::ParseError),
    /// Provider error.
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// Form data for creating a new local comment.
#[derive(serde::Deserialize)]
struct CreateLocalCommentForm {
    /// The comment body text.
    body: String,
    /// Comment type: "general", "file_level", or "line_level".
    #[serde(rename = "type", default)]
    comment_type: String,
    /// File path (for file_level and line_level comments).
    path: Option<String>,
    /// Line number (for line_level comments).
    line: Option<u64>,
    /// Line side: "old" or "new" (for line_level comments).
    side: Option<String>,
    /// Optional AI agent in format "provider:agent" (e.g., "opencode:code").
    ai_agent: Option<String>,
}

/// Form data for replying to a comment thread.
#[derive(serde::Deserialize)]
struct ReplyCommentForm {
    /// The reply body text.
    body: String,
    /// The thread ID to reply to.
    thread_id: Uuid,
    /// Optional AI agent in format "provider:agent".
    ai_agent: Option<String>,
}

/// Add local routes to an existing router.
#[must_use]
pub fn add_local_routes<B: GitBackend + Send + Sync + 'static>(
    router: Router,
    backend: Arc<B>,
) -> Router {
    let backend_local = Arc::clone(&backend);
    let backend_create = Arc::clone(&backend);
    let backend_reply = Arc::clone(&backend);
    let backend_delete = Arc::clone(&backend);
    let backend_resolve = Arc::clone(&backend);
    let backend_reply_view = Arc::clone(&backend);
    let backend_file_view = Arc::clone(&backend);
    let backend_file_diff = Arc::clone(&backend);

    router
        .with_route_result("/local", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_local);
                async move { local_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/comment", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_create);
                async move { create_comment_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/comment/reply", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_reply);
                async move { reply_comment_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/comment/delete", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_delete);
                async move { delete_comment_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/comment/state", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_resolve);
                async move { set_state_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/reply/view", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_reply_view);
                async move { reply_view_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/file/view", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_file_view);
                async move { file_view_route(req, backend).await }
            }
        })
        .with_route_result("/api/local/file/diff", {
            move |req: RouteRequest| {
                let backend = Arc::clone(&backend_file_diff);
                async move { file_diff_route(req, backend).await }
            }
        })
}

/// Handle the `/local` route for viewing local git diffs.
///
/// Query parameters:
/// - `repo` - Optional path to the repository (defaults to CWD)
/// - `base` / `head` - Range diff (e.g., `main..feature`)
/// - `three_dot` - Use merge-base semantics (e.g., `main...feature`)
/// - `commit` - Single commit SHA
/// - `commits` - Comma-separated commit SHAs
/// - `mode` - Multi-commit mode: `separate` (default) or `squashed`
/// - `staged` - Only show staged changes
/// - `against` - What to diff working tree against (default: HEAD)
/// - `untracked` - Include untracked files (default: true)
async fn local_route<B: GitBackend + 'static>(
    req: RouteRequest,
    backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Get) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    // Determine repository path
    let repo_path = get_repo_path(&req)?;

    // Create provider for this repository
    let provider = LocalDiffProvider::from_path(Arc::clone(&backend), &repo_path)
        .map_err(|e| LocalRouteError::InvalidRepoPath(e.to_string()))?;

    // Parse diff specification from query params
    let spec = DiffSpec::from_query(&req.query)?;

    // Get diff info and files
    let info = provider.get_diff_info(&spec)?;
    let diffs = provider.get_diff(&spec)?;

    // Load comments for this repository
    let store = LocalCommentStore::new(&repo_path)?;
    let thread_indices = store.list_threads()?;

    // Load full threads for display
    let mut comments = Vec::new();
    for idx in &thread_indices {
        if let Ok(thread) = store.load_thread(idx.id) {
            comments.push(thread);
        }
    }

    // Load viewed files and replies
    let viewed_paths = store.get_viewed_file_paths()?;
    let viewed_reply_ids = store.get_viewed_reply_ids()?;

    Ok(render_local_view(
        &info,
        &diffs,
        &comments,
        &repo_path,
        &viewed_paths,
        &viewed_reply_ids,
    ))
}

/// Handle POST `/api/local/comment` - Create a new comment.
///
/// Query parameters:
/// - `repo` - Repository path
///
/// Form body:
/// - `body` - Comment text (required)
/// - `type` - Comment type: "general", "file_level", or "line_level"
/// - `path` - File path (for file_level and line_level)
/// - `line` - Line number (for line_level)
/// - `side` - Line side: "old" or "new" (for line_level)
/// - `ai_agent` - Optional AI agent in format "provider:agent" (e.g., "opencode:code")
async fn create_comment_route<B: GitBackend + 'static>(
    req: RouteRequest,
    _backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Post) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    let repo_path = get_repo_path(&req)?;

    // Parse the form body
    let form: CreateLocalCommentForm = req.parse_form()?;

    // Convert form data to comment type
    let comment_type = parse_comment_type_from_form(&form)?;
    let ai_action = parse_ai_action_from_form(&form);

    // Get user identity from git config (simplified)
    let author = LocalUser::default(); // TODO: Get from git config

    // Create the comment
    let mut comment = LocalComment::new(author, form.body, comment_type);
    if let Some(action) = ai_action {
        comment = comment.with_ai_action(action);
    }

    // Save to store
    let store = LocalCommentStore::new(&repo_path)?;
    store.save_thread(&comment)?;

    // If AI action is specified, trigger execution
    if comment.ai_action.is_some() {
        // Clone what we need for the spawned task
        let comment_id = comment.id;
        let repo = repo_path.clone();

        // Spawn async AI execution (thread_id == comment_id for root comments)
        switchy::unsync::task::spawn(async move {
            execute_ai_action(repo, comment_id, comment_id).await;
        });
    }

    // Return the rendered comment as a full thread with reply form
    // New comments have no replies yet, so empty set
    let viewed_reply_ids = std::collections::HashSet::new();
    let repo_path_str = repo_path.to_string_lossy().to_string();
    Ok(local_comments::render_local_comment_with_reply(
        &comment,
        &repo_path_str,
        &viewed_reply_ids,
    ))
}

/// Handle POST `/api/local/comment/reply` - Reply to a comment.
///
/// Query parameters:
/// - `repo` - Repository path
///
/// Form body:
/// - `body` - Reply text (required)
/// - `thread_id` - UUID of the thread to reply to
/// - `ai_agent` - Optional AI agent in format "provider:agent"
async fn reply_comment_route<B: GitBackend + 'static>(
    req: RouteRequest,
    _backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Post) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    let repo_path = get_repo_path(&req)?;

    // Parse the form body
    let form: ReplyCommentForm = req.parse_form()?;
    let ai_action = parse_ai_action_from_string(form.ai_agent.as_deref());

    let author = LocalUser::default();

    // Create the reply comment
    let reply_type = LocalCommentType::Reply {
        root_comment_id: form.thread_id,
        in_reply_to: form.thread_id,
    };
    let mut reply = LocalComment::new(author, form.body, reply_type);
    if let Some(action) = ai_action.clone() {
        reply = reply.with_ai_action(action);
    }

    let store = LocalCommentStore::new(&repo_path)?;
    store.add_reply(form.thread_id, reply.clone())?;

    // If AI action is specified, trigger execution
    if ai_action.is_some() {
        let thread_id = form.thread_id;
        let reply_id = reply.id;
        let repo = repo_path.clone();

        switchy::unsync::task::spawn(async move {
            execute_ai_action(repo, thread_id, reply_id).await;
        });
    }

    // Return just the rendered reply item (not a full thread wrapper)
    // New replies are not viewed yet
    let repo_path_str = repo_path.to_string_lossy().to_string();
    Ok(local_comments::render_local_comment_item(
        &reply,
        form.thread_id,
        &repo_path_str,
        false, // is_viewed
    ))
}

/// Handle DELETE `/api/local/comment/delete` - Delete a comment or thread.
///
/// Query parameters:
/// - `repo` - Repository path
/// - `thread_id` - The root thread ID
/// - `comment_id` - Optional: The specific comment to delete. If not provided or same as
///                  thread_id, deletes the entire thread. Otherwise deletes just that reply.
async fn delete_comment_route<B: GitBackend + 'static>(
    req: RouteRequest,
    _backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Delete) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    let repo_path = get_repo_path(&req)?;
    let thread_id = parse_uuid_param(&req, "thread_id")?;

    // Get optional comment_id, defaulting to thread_id if not provided
    let comment_id = req
        .query
        .get("comment_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(thread_id);

    let store = LocalCommentStore::new(&repo_path)?;

    if thread_id == comment_id {
        // Delete entire thread
        store.delete_thread(thread_id)?;
        log::info!("Deleted thread {thread_id}");
    } else {
        // Delete just the reply
        store.delete_reply(thread_id, comment_id)?;
        log::info!("Deleted reply {comment_id} from thread {thread_id}");
    }

    // Return empty container - HyperChad will use hx-swap="delete" to remove the element
    Ok(container! { div {} }.into())
}

/// Handle POST `/api/local/comment/state` - Set thread state explicitly.
///
/// Query parameters:
/// - `repo` - Repository path
/// - `thread_id` - The root thread ID
/// - `state` - New state: "open", "resolved", or "saved_for_later"
///
/// Sets the thread state and returns the re-rendered thread.
async fn set_state_route<B: GitBackend + 'static>(
    req: RouteRequest,
    _backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Post) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    let repo_path = get_repo_path(&req)?;
    let thread_id = parse_uuid_param(&req, "thread_id")?;

    // Parse state from query param
    let new_state = req
        .query
        .get("state")
        .map(|s| match s.as_str() {
            "resolved" => ThreadState::Resolved,
            "saved_for_later" => ThreadState::SavedForLater,
            _ => ThreadState::Open,
        })
        .unwrap_or(ThreadState::Open);

    let store = LocalCommentStore::new(&repo_path)?;

    // Load the thread
    let mut thread = store.load_thread(thread_id)?;

    // Set new state
    thread.state = new_state;
    thread.updated_at = Utc::now();

    // Save the updated thread
    store.save_thread(&thread)?;

    log::info!("Thread {thread_id} state changed to: {:?}", thread.state);

    // Re-render the thread with the new state
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let viewed_reply_ids = store.get_viewed_reply_ids()?;
    Ok(local_comments::render_local_comment_with_reply(
        &thread,
        &repo_path_str,
        &viewed_reply_ids,
    ))
}

/// Handle POST/DELETE `/api/local/reply/view` - Mark reply as viewed/unviewed.
///
/// Query parameters:
/// - `repo` - Repository path
/// - `thread_id` - The thread containing the reply
/// - `reply_id` - The specific reply to mark viewed/unviewed
///
/// POST: Mark as viewed, returns re-rendered reply (collapsed)
/// DELETE: Mark as unviewed, returns re-rendered reply (expanded)
async fn reply_view_route<B: GitBackend + 'static>(
    req: RouteRequest,
    _backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    let repo_path = get_repo_path(&req)?;
    let thread_id = parse_uuid_param(&req, "thread_id")?;
    let reply_id = parse_uuid_param(&req, "reply_id")?;

    let store = LocalCommentStore::new(&repo_path)?;

    // Load the thread to find the reply
    let thread = store.load_thread(thread_id)?;

    // Find the reply in the thread
    let reply = find_reply_in_thread(&thread, reply_id)
        .ok_or(LocalRouteError::CommentNotFound(reply_id))?;

    match req.method {
        Method::Post => {
            // Mark as viewed
            store.mark_reply_viewed(reply_id)?;
            log::info!("Marked reply {reply_id} as viewed");
        }
        Method::Delete => {
            // Mark as unviewed
            store.mark_reply_unviewed(reply_id)?;
            log::info!("Marked reply {reply_id} as unviewed");
        }
        _ => return Err(LocalRouteError::UnsupportedMethod),
    }

    // Re-render the reply with the new state
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let is_viewed = store.is_reply_viewed(reply_id);
    Ok(local_comments::render_local_comment_item(
        &reply,
        thread_id,
        &repo_path_str,
        is_viewed,
    ))
}

/// Find a reply within a thread by ID.
fn find_reply_in_thread(thread: &LocalComment, reply_id: Uuid) -> Option<LocalComment> {
    for reply in &thread.replies {
        if reply.id == reply_id {
            return Some(reply.clone());
        }
        // Check nested replies (if any)
        if let Some(found) = find_reply_in_thread(reply, reply_id) {
            return Some(found);
        }
    }
    None
}

/// Handle POST/DELETE `/api/local/file/view` - Mark file as viewed/unviewed.
///
/// Query parameters:
/// - `repo` - Repository path
/// - `path` - File path to mark
///
/// POST: Mark as viewed, returns collapsed file header
/// DELETE: Mark as unviewed, returns expanded file with content
async fn file_view_route<B: GitBackend + 'static>(
    req: RouteRequest,
    backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    let repo_path = get_repo_path(&req)?;
    let file_path = req
        .query
        .get("path")
        .ok_or_else(|| LocalRouteError::MissingParameter("path".to_string()))?
        .clone();

    let store = LocalCommentStore::new(&repo_path)?;

    match req.method {
        Method::Post => {
            // Mark as viewed -> return collapsed header
            store.mark_file_viewed(&file_path)?;
            log::info!("Marked file as viewed: {file_path}");

            // We need the diff file info to render the header
            // Load the diff to find this file
            let provider = LocalDiffProvider::from_path(Arc::clone(&backend), &repo_path)
                .map_err(|e| LocalRouteError::ProviderError(e.to_string()))?;
            let spec = parse_diff_spec(&req)?;
            let diffs = provider.get_diff(&spec)?;

            let diff_file = diffs
                .iter()
                .find(|f| f.filename == file_path)
                .ok_or_else(|| {
                    LocalRouteError::MissingParameter(format!("File not found: {file_path}"))
                })?;

            let repo_path_str = repo_path.to_string_lossy().to_string();
            Ok(diff_viewer::render_file_collapsed(diff_file, &repo_path_str).into())
        }
        Method::Delete => {
            // Mark as unviewed -> return expanded file with content
            store.mark_file_unviewed(&file_path)?;
            log::info!("Marked file as unviewed: {file_path}");

            // Load diff and comments to render the full file
            let provider = LocalDiffProvider::from_path(Arc::clone(&backend), &repo_path)
                .map_err(|e| LocalRouteError::ProviderError(e.to_string()))?;
            let spec = parse_diff_spec(&req)?;
            let diffs = provider.get_diff(&spec)?;

            let diff_file = diffs
                .iter()
                .find(|f| f.filename == file_path)
                .ok_or_else(|| {
                    LocalRouteError::MissingParameter(format!("File not found: {file_path}"))
                })?;

            // Load comments
            let thread_indices = store.list_threads()?;
            let mut comments = Vec::new();
            for idx in &thread_indices {
                if let Ok(thread) = store.load_thread(idx.id) {
                    comments.push(thread);
                }
            }

            // Load viewed reply IDs
            let viewed_reply_ids = store.get_viewed_reply_ids()?;

            let repo_path_str = repo_path.to_string_lossy().to_string();
            Ok(diff_viewer::render_file_expanded(
                diff_file,
                &comments,
                &repo_path_str,
                false,
                &viewed_reply_ids,
            )
            .into())
        }
        _ => Err(LocalRouteError::UnsupportedMethod),
    }
}

/// Handle GET `/api/local/file/diff` - Get rendered diff for a file (lazy loading).
///
/// Query parameters:
/// - `repo` - Repository path
/// - `path` - File path to render
///
/// Returns the expanded file container (header + content).
async fn file_diff_route<B: GitBackend + 'static>(
    req: RouteRequest,
    backend: Arc<B>,
) -> Result<Container, LocalRouteError> {
    if !matches!(req.method, Method::Get) {
        return Err(LocalRouteError::UnsupportedMethod);
    }

    let repo_path = get_repo_path(&req)?;
    let file_path = req
        .query
        .get("path")
        .ok_or_else(|| LocalRouteError::MissingParameter("path".to_string()))?
        .clone();

    let store = LocalCommentStore::new(&repo_path)?;

    // Load diff
    let provider = LocalDiffProvider::from_path(Arc::clone(&backend), &repo_path)
        .map_err(|e| LocalRouteError::ProviderError(e.to_string()))?;
    let spec = parse_diff_spec(&req)?;
    let diffs = provider.get_diff(&spec)?;

    let diff_file = diffs
        .iter()
        .find(|f| f.filename == file_path)
        .ok_or_else(|| LocalRouteError::MissingParameter(format!("File not found: {file_path}")))?;

    // Load comments
    let thread_indices = store.list_threads()?;
    let mut comments = Vec::new();
    for idx in &thread_indices {
        if let Ok(thread) = store.load_thread(idx.id) {
            comments.push(thread);
        }
    }

    // Check if file is viewed (for the checkbox state)
    let is_viewed = store.is_file_viewed(&file_path);

    // Load viewed reply IDs
    let viewed_reply_ids = store.get_viewed_reply_ids()?;

    let repo_path_str = repo_path.to_string_lossy().to_string();
    Ok(diff_viewer::render_file_expanded(
        diff_file,
        &comments,
        &repo_path_str,
        is_viewed,
        &viewed_reply_ids,
    )
    .into())
}

// Helper functions

fn get_repo_path(req: &RouteRequest) -> Result<PathBuf, LocalRouteError> {
    if let Some(path) = req.query.get("repo") {
        Ok(PathBuf::from(path))
    } else {
        std::env::current_dir().map_err(|e| LocalRouteError::InvalidRepoPath(e.to_string()))
    }
}

fn parse_uuid_param(req: &RouteRequest, name: &str) -> Result<Uuid, LocalRouteError> {
    let value = req
        .query
        .get(name)
        .ok_or_else(|| LocalRouteError::MissingParameter(name.to_string()))?;
    Uuid::parse_str(value).map_err(|_| LocalRouteError::InvalidUuid(value.clone()))
}

/// Parse diff specification from query parameters.
fn parse_diff_spec(req: &RouteRequest) -> Result<DiffSpec, LocalRouteError> {
    DiffSpec::from_query(&req.query).map_err(LocalRouteError::from)
}

/// Parse comment type from form data.
fn parse_comment_type_from_form(
    form: &CreateLocalCommentForm,
) -> Result<LocalCommentType, LocalRouteError> {
    let type_str = if form.comment_type.is_empty() {
        "general"
    } else {
        &form.comment_type
    };

    match type_str {
        "general" => Ok(LocalCommentType::General),
        "file_level" => {
            let path = form
                .path
                .clone()
                .ok_or_else(|| LocalRouteError::MissingParameter("path".to_string()))?;
            Ok(LocalCommentType::FileLevelComment { path })
        }
        "line_level" => {
            let path = form
                .path
                .clone()
                .ok_or_else(|| LocalRouteError::MissingParameter("path".to_string()))?;
            let line = form
                .line
                .ok_or_else(|| LocalRouteError::MissingParameter("line".to_string()))?;
            let side = form.side.as_deref().unwrap_or("new");
            let line_number = if side == "old" {
                chadreview_local_comment_models::LineNumber::Old { line }
            } else {
                chadreview_local_comment_models::LineNumber::New { line }
            };
            Ok(LocalCommentType::LineLevelComment {
                path,
                line: line_number,
            })
        }
        _ => Ok(LocalCommentType::General),
    }
}

/// Parse AI action from form data.
fn parse_ai_action_from_form(form: &CreateLocalCommentForm) -> Option<AiAction> {
    parse_ai_action_from_string(form.ai_agent.as_deref())
}

/// Parse AI action from an optional agent string.
fn parse_ai_action_from_string(agent_str: Option<&str>) -> Option<AiAction> {
    let agent_str = agent_str?;

    if agent_str.is_empty() {
        return None;
    }

    // Parse "provider:agent" format
    let parts: Vec<&str> = agent_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    Some(AiAction {
        provider: parts[0].to_string(),
        agent: parts[1].to_string(),
        model: None,
        custom_instructions: None,
    })
}

/// Execute AI action for a comment using OpenCode provider.
///
/// # Arguments
/// * `repo_path` - Path to the repository
/// * `thread_id` - The root thread ID (for loading/saving)
/// * `comment_id` - The comment with the AI action (same as thread_id for root comments)
#[cfg(feature = "ai-integration-opencode")]
async fn execute_ai_action(repo_path: PathBuf, thread_id: Uuid, comment_id: Uuid) {
    log::info!("Starting AI execution for comment {comment_id} (OpenCode)");

    // Load the comment and its AI action
    let store = match LocalCommentStore::new(&repo_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open comment store: {e}");
            return;
        }
    };

    // Load the root thread to get session ID for conversation continuity
    let thread = match store.load_thread(thread_id) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Thread {thread_id} not found: {e}");
            return;
        }
    };
    let session_id = thread.opencode_session_id.clone();

    // Load the specific comment (may be same as thread for root comments)
    let comment = if thread_id == comment_id {
        thread.clone()
    } else {
        match store.get_comment(comment_id) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Comment {comment_id} not found: {e}");
                return;
            }
        }
    };

    let ai_action = match &comment.ai_action {
        Some(a) => a.clone(),
        None => {
            log::error!("Comment {comment_id} has no AI action");
            return;
        }
    };

    if session_id.is_some() {
        log::info!("Continuing OpenCode session: {:?}", session_id);
    }

    // Update status to Running
    let started_at = Utc::now();
    let running_status = AiExecutionStatus::Running {
        started_at,
        progress: vec![],
    };

    if let Err(e) = store.update_reply_ai_status(thread_id, comment_id, running_status.clone()) {
        log::error!("Failed to update AI status: {e}");
        return;
    }
    push_ai_status_update(comment_id, &running_status).await;

    // Build AI context from comment
    let context = build_ai_context(&repo_path, &comment);

    // Create progress channel
    let (progress_tx, mut progress_rx) = switchy::unsync::sync::mpsc::unbounded::<ProgressEntry>();

    // Spawn task to forward progress updates to SSE
    let store_for_progress = match LocalCommentStore::new(&repo_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open comment store for progress: {e}");
            return;
        }
    };
    let progress_thread_id = thread_id;
    let progress_comment_id = comment_id;
    switchy::unsync::task::spawn(async move {
        let mut progress_entries = Vec::new();
        while let Ok(entry) = progress_rx.recv_async().await {
            progress_entries.push(entry);
            let status = AiExecutionStatus::Running {
                started_at,
                progress: progress_entries.clone(),
            };
            store_for_progress
                .update_reply_ai_status(progress_thread_id, progress_comment_id, status.clone())
                .inspect_err(|e| {
                    log::warn!(
                        "Failed to update AI status for comment {}: {}",
                        progress_comment_id,
                        e
                    )
                })
                .ok();
            push_ai_status_update(progress_comment_id, &status).await;
        }
    });

    // Execute via OpenCode provider
    let provider = OpenCodeProvider::new();
    let result = provider
        .execute(&context, &ai_action, session_id.as_deref(), progress_tx)
        .await;

    let finished_at = Utc::now();

    match result {
        Ok(response) => {
            // Always update session ID if we got one back
            if let Some(new_session_id) = &response.session_id {
                if let Err(e) = store.update_session_id(thread_id, new_session_id.clone()) {
                    log::warn!("Failed to save session ID: {e}");
                } else {
                    log::debug!("Updated thread {thread_id} with session ID: {new_session_id}");
                }
            }

            // Create reply with AI response
            let ai_author = LocalUser {
                name: format!("AI ({})", ai_action.agent),
                email: "ai@local".to_string(),
            };

            let response_comment = LocalComment::new(
                ai_author,
                response.content.clone(),
                LocalCommentType::Reply {
                    root_comment_id: thread_id,
                    in_reply_to: comment_id,
                },
            );

            if let Err(e) = store.add_reply(thread_id, response_comment.clone()) {
                log::error!("Failed to add AI response: {e}");
                let failed_status = AiExecutionStatus::Failed {
                    finished_at,
                    error: format!("Failed to add response: {e}"),
                };
                store
                    .update_reply_ai_status(thread_id, comment_id, failed_status.clone())
                    .inspect_err(|e| log::warn!("Failed to update AI status to failed: {e}"))
                    .ok();
                push_ai_status_update(comment_id, &failed_status).await;
                return;
            }

            // Reload the thread to get all replies including the new one
            let repo_path_str = repo_path.to_string_lossy().to_string();
            if let Ok(updated_thread) = store.get_comment(thread_id) {
                let viewed_reply_ids = store.get_viewed_reply_ids().unwrap_or_default();
                push_thread_replies(
                    thread_id,
                    &updated_thread.replies,
                    &repo_path_str,
                    &viewed_reply_ids,
                )
                .await;
            }

            let completed_status = AiExecutionStatus::Completed {
                finished_at,
                response_comment_id: response_comment.id,
                execution_details: response.execution_details,
            };

            store
                .update_reply_ai_status(thread_id, comment_id, completed_status.clone())
                .inspect_err(|e| log::error!("Failed to update AI status to completed: {e}"))
                .ok();
            push_ai_status_update(comment_id, &completed_status).await;
            log::info!("AI execution completed for comment {comment_id}");
        }
        Err(e) => {
            log::error!("AI execution failed for comment {comment_id}: {e}");
            let failed_status = AiExecutionStatus::Failed {
                finished_at,
                error: format!("{e}"),
            };
            store
                .update_reply_ai_status(thread_id, comment_id, failed_status.clone())
                .inspect_err(|e| log::warn!("Failed to update AI status to failed: {e}"))
                .ok();
            push_ai_status_update(comment_id, &failed_status).await;
        }
    }
}

/// Build AI context from a comment.
#[cfg(feature = "ai-integration-opencode")]
fn build_ai_context(repo_path: &PathBuf, comment: &LocalComment) -> AiContext {
    let mut context = AiContext::new(
        repo_path.clone(),
        "local diff".to_string(), // TODO: Get actual diff description
        comment.body.clone(),
    );

    // Add file/line context if available
    match &comment.comment_type {
        LocalCommentType::FileLevelComment { path } => {
            context = context.with_file_path(path.clone());
        }
        LocalCommentType::LineLevelComment { path, line } => {
            context = context.with_file_path(path.clone());
            context = context.with_line(line.to_string());
        }
        _ => {}
    }

    context
}

/// Execute AI action for a comment (simulation fallback when OpenCode not available).
///
/// # Arguments
/// * `repo_path` - Path to the repository
/// * `thread_id` - The root thread ID (for loading/saving)
/// * `comment_id` - The comment with the AI action (same as thread_id for root comments)
#[cfg(not(feature = "ai-integration-opencode"))]
async fn execute_ai_action(repo_path: PathBuf, thread_id: Uuid, comment_id: Uuid) {
    log::info!("Starting AI execution for comment {comment_id} (simulated)");

    // Load the comment and its AI action
    let store = match LocalCommentStore::new(&repo_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open comment store: {e}");
            return;
        }
    };

    let comment = match store.get_comment(comment_id) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Comment {comment_id} not found: {e}");
            return;
        }
    };

    let ai_action = match &comment.ai_action {
        Some(a) => a.clone(),
        None => {
            log::error!("Comment {comment_id} has no AI action");
            return;
        }
    };

    // Update status to Running
    let started_at = Utc::now();
    let running_status = AiExecutionStatus::Running {
        started_at,
        progress: vec![],
    };

    if let Err(e) = store.update_reply_ai_status(thread_id, comment_id, running_status.clone()) {
        log::error!("Failed to update AI status: {e}");
        return;
    }
    push_ai_status_update(comment_id, &running_status).await;

    log::warn!(
        "AI integration not enabled. Simulating execution for {}:{} on comment {}",
        ai_action.provider,
        ai_action.agent,
        comment_id
    );

    // Simulate progress updates
    for i in 1..=3 {
        switchy::unsync::time::sleep(std::time::Duration::from_secs(1)).await;

        let progress = AiExecutionStatus::Running {
            started_at,
            progress: (1..=i)
                .map(|n| ProgressEntry {
                    tool: format!("tool_{n}"),
                    title: format!("Simulated step {n}"),
                    timestamp: Utc::now(),
                })
                .collect(),
        };

        store
            .update_reply_ai_status(thread_id, comment_id, progress.clone())
            .inspect_err(|e| log::warn!("Failed to update AI progress status: {e}"))
            .ok();
        push_ai_status_update(comment_id, &progress).await;
    }

    let finished_at = Utc::now();
    let duration = finished_at.signed_duration_since(started_at).num_seconds() as u64;

    // Create simulated response
    let ai_response_body = format!(
        "**AI Response** (simulated - OpenCode integration not enabled)\n\n\
        To enable real AI execution, build with:\n\
        ```\n\
        cargo run --features ai-integration-opencode\n\
        ```\n\n\
        Or set `OPENCODE_BINARY` environment variable to specify the opencode binary path.\n\n\
        Agent requested: `{}:{}`\n\
        Duration: {}s",
        ai_action.provider, ai_action.agent, duration
    );

    let ai_author = LocalUser {
        name: format!("AI ({})", ai_action.agent),
        email: "ai@local".to_string(),
    };

    let response_comment = LocalComment::new(
        ai_author,
        ai_response_body,
        LocalCommentType::Reply {
            root_comment_id: thread_id,
            in_reply_to: comment_id,
        },
    );

    if let Err(e) = store.add_reply(thread_id, response_comment.clone()) {
        log::error!("Failed to add AI response: {e}");
        let failed_status = AiExecutionStatus::Failed {
            finished_at,
            error: format!("Failed to add response: {e}"),
        };
        store
            .update_reply_ai_status(thread_id, comment_id, failed_status.clone())
            .inspect_err(|e| log::warn!("Failed to update AI status to failed: {e}"))
            .ok();
        push_ai_status_update(comment_id, &failed_status).await;
        return;
    }

    // Reload the thread to get all replies including the new one
    let repo_path_str = repo_path.to_string_lossy().to_string();
    if let Ok(updated_thread) = store.get_comment(thread_id) {
        let viewed_reply_ids = store.get_viewed_reply_ids().unwrap_or_default();
        push_thread_replies(
            thread_id,
            &updated_thread.replies,
            &repo_path_str,
            &viewed_reply_ids,
        )
        .await;
    }

    let completed_status = AiExecutionStatus::Completed {
        finished_at,
        response_comment_id: response_comment.id,
        execution_details: Some(chadreview_local_comment_models::ExecutionDetails {
            model_used: "simulated".to_string(),
            tools_used: vec![],
            tokens: chadreview_local_comment_models::TokenUsage {
                input: 0,
                output: 0,
            },
            cost: None,
            duration_seconds: duration,
        }),
    };

    store
        .update_reply_ai_status(thread_id, comment_id, completed_status.clone())
        .inspect_err(|e| log::error!("Failed to update AI status to completed: {e}"))
        .ok();
    push_ai_status_update(comment_id, &completed_status).await;
    log::info!("AI execution completed for comment {comment_id} (simulated)");
}

/// Render the local diff view with comments.
fn render_local_view(
    info: &chadreview_local_diff_models::LocalDiffInfo,
    diffs: &[chadreview_pr_models::DiffFile],
    comments: &[LocalComment],
    repo_path: &std::path::Path,
    viewed_paths: &std::collections::HashSet<String>,
    viewed_reply_ids: &std::collections::HashSet<Uuid>,
) -> Container {
    let repo_path_str = repo_path.to_string_lossy();

    let general_comments: Vec<_> = comments
        .iter()
        .filter(|c| matches!(c.comment_type, LocalCommentType::General))
        .collect();

    container! {
        div padding=20 gap=20 {
            (local_header::render_local_diff_header(info))

            // General comments section - always render container so hx-target works
            div id="general-comments-section" gap=12 {
                @if !general_comments.is_empty() {
                    div direction=row align-items=center justify-content=space-between margin-bottom=8 {
                        h2 font-size=18 font-weight=600 color="#24292f" {
                            "General Comments"
                        }
                        (local_comments::render_general_comment_controls())
                    }
                    @for comment in &general_comments {
                        (local_comments::render_local_comment_with_reply(comment, &repo_path_str, viewed_reply_ids))
                    }
                }
            }

            // Comment form for general comments
            (render_comment_form(&repo_path_str))

            // Diff view with inline comments (file-level and line-level)
            (diff_viewer::render_local(diffs, comments, &repo_path_str, viewed_paths, viewed_reply_ids))
        }
    }
    .into()
}

/// Render the comment creation form for general comments.
fn render_comment_form(repo_path: &str) -> Container {
    let api_url = format!("/api/local/comment?repo={}", urlencoding::encode(repo_path));

    container! {
        div
            padding=16
            background="#f6f8fa"
            border="1px solid #d0d7de"
            border-radius=6
            gap=12
        {
            h3 font-size=16 font-weight=600 color="#24292f" margin-bottom=8 {
                "Add General Comment"
            }
            form
                hx-post=(api_url)
                hx-swap="beforeend"
                hx-target="#general-comments-section"
                gap=12
            {
                input type=hidden name="type" value="general";

                textarea
                    name="body"
                    placeholder="Write your comment..."
                    height=100
                    padding=12
                    border="1px solid #d0d7de"
                    border-radius=6
                    font-size=14;

                div margin-top=8 {
                    (local_comments::render_ai_action_selector("ai_agent", local_comments::DEFAULT_AGENTS))
                }

                button
                    type=submit
                    background="#1a7f37"
                    color="#ffffff"
                    padding-x=16
                    padding-y=8
                    border-radius=6
                    font-weight=600
                    font-size=14
                    cursor=pointer
                    margin-top=8
                {
                    "Submit Comment"
                }
            }
        }
    }
    .into()
}
