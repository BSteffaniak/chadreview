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
    #[error("Missing body")]
    MissingBody,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
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
    let number_str = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?;
    let number: u64 = number_str.parse()?;

    let pr = provider.get_pr(owner, repo, number).await?;
    let diffs = provider.get_diff(owner, repo, number).await?;

    Ok(render_pr_view(&pr, &diffs))
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
    let number_str = req
        .query
        .get("number")
        .ok_or(RouteError::MissingQueryParam("number"))?;
    let number: u64 = number_str.parse()?;

    let body_bytes = req.body.ok_or(RouteError::MissingBody)?;
    let create_comment: CreateComment = serde_json::from_slice(&body_bytes)?;

    let comment = provider
        .create_comment(owner, repo, number, create_comment)
        .await?;

    Ok(render_comment(&comment))
}

async fn update_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Put) {
        return Err(RouteError::UnsupportedMethod);
    }

    let comment_id_str = req
        .query
        .get("id")
        .ok_or(RouteError::MissingQueryParam("id"))?;
    let comment_id: u64 = comment_id_str.parse()?;

    let body_bytes = req.body.ok_or(RouteError::MissingBody)?;
    let update: UpdateBody = serde_json::from_slice(&body_bytes)?;

    let comment = provider.update_comment(comment_id, update.body).await?;

    Ok(render_comment(&comment))
}

async fn delete_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<(), RouteError> {
    if !matches!(req.method, Method::Delete) {
        return Err(RouteError::UnsupportedMethod);
    }

    let comment_id_str = req
        .query
        .get("id")
        .ok_or(RouteError::MissingQueryParam("id"))?;
    let comment_id: u64 = comment_id_str.parse()?;

    provider.delete_comment(comment_id).await?;

    Ok(())
}

fn render_pr_view(
    pr: &chadreview_pr_models::PullRequest,
    diffs: &[chadreview_pr_models::DiffFile],
) -> Container {
    use hyperchad::template::container;

    container! {
        div class="pr-view" {
            (chadreview_app_ui::pr_header::render_pr_header(pr))
            (chadreview_app_ui::diff_viewer::render(diffs))
        }
    }
    .into()
}

fn render_comment(_comment: &chadreview_pr_models::Comment) -> Container {
    use hyperchad::template::container;

    container! {
        div class="comment" {
            div class="comment-author" { "Comment Author" }
            div class="comment-body" { "Comment Body" }
        }
    }
    .into()
}
