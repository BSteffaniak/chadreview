//! Local git diff routes.
//!
//! Routes for viewing local git diffs without GitHub integration.

use std::path::PathBuf;
use std::sync::Arc;

use chadreview_app_ui::{diff_viewer, local_header};
use chadreview_git_backend::GitBackend;
use chadreview_local_diff::LocalDiffProvider;
use chadreview_local_diff_models::{DiffSpec, DiffSpecError};
use hyperchad::{
    router::{Container, RouteRequest, Router},
    template::container,
};
use switchy::http::models::Method;

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
}

/// Add local routes to an existing router.
///
/// This adds the `/local` route for viewing local git diffs.
#[must_use]
pub fn add_local_routes<B: GitBackend + Send + Sync + 'static>(
    router: Router,
    backend: Arc<B>,
) -> Router {
    router.with_route_result("/local", {
        move |req: RouteRequest| {
            let backend = Arc::clone(&backend);
            async move { local_route(req, backend).await }
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
    let repo_path = if let Some(path) = req.query.get("repo") {
        PathBuf::from(path)
    } else {
        std::env::current_dir().map_err(|e| LocalRouteError::InvalidRepoPath(e.to_string()))?
    };

    // Create provider for this repository
    let provider = LocalDiffProvider::from_path(Arc::clone(&backend), &repo_path)
        .map_err(|e| LocalRouteError::InvalidRepoPath(e.to_string()))?;

    // Parse diff specification from query params
    let spec = DiffSpec::from_query(&req.query)?;

    // Get diff info and files
    let info = provider.get_diff_info(&spec)?;
    let diffs = provider.get_diff(&spec)?;

    Ok(render_local_view(&info, &diffs))
}

/// Render the local diff view.
fn render_local_view(
    info: &chadreview_local_diff_models::LocalDiffInfo,
    diffs: &[chadreview_pr_models::DiffFile],
) -> Container {
    container! {
        div class="local-diff-view" {
            (local_header::render_local_diff_header(info))
            (diff_viewer::render_readonly(diffs))
        }
    }
    .into()
}
