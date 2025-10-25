#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::Arc;

use chadreview_app_ui::{
    comment_thread::{
        comment_thread_id, render_comment_item, render_comment_thread, render_reply_form,
    },
    diff_viewer::render_line_comments,
    general_comments,
};
use chadreview_git_provider::GitProvider;
use chadreview_pr_models::{
    CommentType, CreateComment,
    comment::{LineNumber, ParseLineNumberError},
};
use hyperchad::{
    renderer::Content,
    router::{Container, RouteRequest, Router},
};
use hyperchad_template::{Selector, container};
use switchy::http::models::Method;

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("Missing query param: '{0}'")]
    MissingQueryParam(&'static str),
    #[error("Unsupported method")]
    UnsupportedMethod,
    #[error("Invalid LineNumber")]
    InvalidLineNumber(#[from] ParseLineNumberError),
    #[error("Invalid bool")]
    InvalidBool(#[from] std::str::ParseBoolError),
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
        .with_route_result("/api/comment/delete", {
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
) -> Result<Content, RouteError> {
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
    let comment_type = create_comment.comment_type.clone();

    let comment = provider
        .create_comment(owner, repo, number, create_comment)
        .await?;

    let mut content = Content::builder();

    match comment_type {
        CommentType::General | CommentType::FileLevelComment { .. } | CommentType::Reply { .. } => {
            if let CommentType::Reply {
                root_comment_id,
                in_reply_to,
            } = &comment_type
            {
                content.primary(render_comment_thread(
                    *root_comment_id,
                    &comment,
                    1,
                    owner,
                    repo,
                    number,
                ));
                content.fragment(render_reply_form(
                    *root_comment_id,
                    *in_reply_to,
                    owner,
                    repo,
                    number,
                ));
            } else {
                content.primary(render_comment_thread(
                    comment.id, &comment, 0, owner, repo, number,
                ));
            }
        }
        CommentType::LineLevelComment {
            path,
            commit_sha,
            line,
        } => {
            content.primary(render_line_comments(
                &commit_sha,
                std::iter::once(&comment),
                &path,
                line,
                owner,
                repo,
                number,
            ));
        }
    }

    Ok(content.build())
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
    let root = req
        .query
        .get("root")
        .map(|s| s.parse::<bool>())
        .transpose()?
        .unwrap_or(false);

    let update: UpdateBody = req.parse_form().map_err(RouteError::InvalidBody)?;

    let comment = provider
        .update_comment(owner, repo, number, comment_id, update.body)
        .await?;

    Ok(render_comment_item(&comment, root, owner, repo, number).into())
}

async fn delete_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Content, RouteError> {
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
    let root = req
        .query
        .get("root")
        .map(|s| s.parse::<bool>())
        .transpose()?
        .unwrap_or(false);

    let mut response = Content::builder();

    if root {
        let path = req
            .query
            .get("path")
            .ok_or(RouteError::MissingQueryParam("path"))?;
        let line = req
            .query
            .get("line")
            .ok_or(RouteError::MissingQueryParam("line"))?
            .parse::<LineNumber>()?;
        let commit_sha = req
            .query
            .get("commit_sha")
            .ok_or(RouteError::MissingQueryParam("commit_sha"))?;

        let mut comment = provider
            .get_comment(owner, repo, number, comment_id, true)
            .await?;

        if !comment.replies.is_empty() {
            let mut root = comment.replies.remove(0);
            root.replies = comment.replies.drain(..).collect();

            response.primary(render_line_comments(
                commit_sha,
                std::iter::once(&root),
                path,
                line,
                owner,
                repo,
                number,
            ));
        }
    }

    response.delete_selector(Selector::Id(comment_thread_id(comment_id)));

    provider
        .delete_comment(owner, repo, number, comment_id)
        .await?;

    Ok(response.build())
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
            (general_comments::render_general_comments_section(comments, owner, repo, number))
            (chadreview_app_ui::diff_viewer::render(&pr.head_sha, diffs, comments, owner, repo, number))
        }
    }
    .into()
}
