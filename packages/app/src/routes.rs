#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::Arc;

use chadreview_git_provider::GitProvider;
use chadreview_pr_models::CreateComment;
use hyperchad::router::{Container, RouteRequest, Router};
use switchy::http::models::Method;

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("Missing query param: '{0}'")]
    MissingQueryParam(&'static str),
    #[error("Unsupported method")]
    UnsupportedMethod,
    #[error("Invalid PR number")]
    InvalidPrNumber(#[from] std::num::ParseIntError),
    #[error("Provider error: {0}")]
    Provider(#[from] anyhow::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid body: {0}")]
    InvalidBody(#[from] hyperchad::router::ParseError),
}

#[derive(serde::Deserialize)]
struct UpdateBody {
    body: String,
}

pub fn create_router(provider: &Arc<dyn GitProvider>) -> Router {
    Router::new()
        .with_route_result("/pr", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { pr_route(req, provider).await }
            }
        })
        .with_route_result("/api/pr/comment", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { create_comment_route(req, provider).await }
            }
        })
        .with_route_result("/api/comment/update", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { update_comment_route(req, provider).await }
            }
        })
        .with_no_content_result("/api/comment/delete", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { delete_comment_route(req, provider).await }
            }
        })
}

async fn pr_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Get) {
        return Err(RouteError::UnsupportedMethod);
    }

    let owner = req
        .query
        .get("owner")
        .ok_or(RouteError::MissingQueryParam("owner"))?;
    let repo = req
        .query
        .get("repo")
        .ok_or(RouteError::MissingQueryParam("repo"))?;
    let number = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?
        .parse::<u64>()?;

    let pr = provider.get_pr(owner, repo, number).await?;
    let diffs = provider.get_diff(owner, repo, number).await?;
    let comments = provider.get_comments(owner, repo, number).await?;

    Ok(render_pr_view(&pr, &diffs, &comments, owner, repo, number))
}

async fn create_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Post) {
        return Err(RouteError::UnsupportedMethod);
    }

    let owner = req
        .query
        .get("owner")
        .ok_or(RouteError::MissingQueryParam("owner"))?;
    let repo = req
        .query
        .get("repo")
        .ok_or(RouteError::MissingQueryParam("repo"))?;
    let number = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?
        .parse::<u64>()?;

    let create_comment: CreateComment = req.parse_form().map_err(RouteError::InvalidBody)?;

    let comment = provider
        .create_comment(owner, repo, number, create_comment)
        .await?;

    Ok(render_comment(&comment, owner, repo, number))
}

async fn update_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Put) {
        return Err(RouteError::UnsupportedMethod);
    }

    let comment_id = req
        .query
        .get("id")
        .ok_or(RouteError::MissingQueryParam("id"))?
        .parse::<u64>()?;
    let owner = req
        .query
        .get("owner")
        .ok_or(RouteError::MissingQueryParam("owner"))?;
    let repo = req
        .query
        .get("repo")
        .ok_or(RouteError::MissingQueryParam("repo"))?;
    let number = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?
        .parse::<u64>()?;

    let update: UpdateBody = req.parse_form().map_err(RouteError::InvalidBody)?;

    let comment = provider
        .update_comment(owner, repo, number, comment_id, update.body)
        .await?;

    Ok(render_comment(&comment, owner, repo, number))
}

async fn delete_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<(), RouteError> {
    if !matches!(req.method, Method::Delete) {
        return Err(RouteError::UnsupportedMethod);
    }

    let comment_id = req
        .query
        .get("id")
        .ok_or(RouteError::MissingQueryParam("id"))?
        .parse::<u64>()?;
    let owner = req
        .query
        .get("owner")
        .ok_or(RouteError::MissingQueryParam("owner"))?;
    let repo = req
        .query
        .get("repo")
        .ok_or(RouteError::MissingQueryParam("repo"))?;
    let number = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?
        .parse::<u64>()?;

    provider
        .delete_comment(owner, repo, number, comment_id)
        .await?;

    Ok(())
}

fn render_pr_view(
    pr: &chadreview_pr_models::PullRequest,
    diffs: &[chadreview_pr_models::DiffFile],
    comments: &[chadreview_pr_models::Comment],
    owner: &str,
    repo: &str,
    number: u64,
) -> Container {
    use hyperchad::template::container;

    container! {
        div class="pr-view" {
            (chadreview_app_ui::pr_header::render_pr_header(pr))
            (chadreview_app_ui::diff_viewer::render(diffs, comments, owner, repo, number))
        }
    }
    .into()
}

fn render_comment(
    comment: &chadreview_pr_models::Comment,
    owner: &str,
    repo: &str,
    number: u64,
) -> Container {
    chadreview_app_ui::comment_thread::render_comment_thread(comment, 0, owner, repo, number).into()
}
