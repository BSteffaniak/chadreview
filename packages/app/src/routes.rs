#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::Arc;

use chadreview_app_ui::{
    comment_thread::{
        comment_class, comment_thread_id, render_comment_item, render_comment_thread,
        render_reply_form,
    },
    diff_viewer::render_line_comments,
    general_comments,
};
use chadreview_git_provider::GitProvider;
use chadreview_pr_models::{
    CommentType, CreateComment,
    comment::{LineNumber, ParseLineNumberError},
};
use chadreview_relay_client::RelayClient;
use chadreview_relay_models::PrKey;
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

pub fn create_router(provider: &Arc<dyn GitProvider>, relay_url: Option<String>) -> Router {
    Router::new()
        .with_route_result("/pr", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                let relay_url = relay_url.clone();
                async move { pr_route(req, provider, relay_url).await }
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
        .with_route_result("/api/comment/resolve", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { resolve_comment_route(req, provider).await }
            }
        })
        .with_route_result("/api/comment/expand", {
            let provider = provider.clone();
            move |req: RouteRequest| {
                let provider = provider.clone();
                async move { expand_comment_route(req, provider).await }
            }
        })
}

async fn pr_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
    relay_url: Option<String>,
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

    // Subscribe to PR webhook events for real-time updates
    if let Some(url) = relay_url {
        let instance_id = RelayClient::get_or_create_instance_id();

        // Connect to relay server (lazily, on first PR view)
        match RelayClient::connect_async(&url, instance_id).await {
            Ok(client) => {
                let pr_key = PrKey {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    number,
                };

                let owner_clone = owner.clone();
                let repo_clone = repo.clone();

                // Create callback that will be invoked when webhook events arrive
                let callback = Arc::new(move |event| {
                    log::info!(
                        "Received webhook event for PR {owner_clone}/{repo_clone} #{number}: {event:?}"
                    );

                    // TODO: In a future enhancement, this callback should:
                    // 1. Refetch the updated PR data from the GitHub provider
                    // 2. Trigger HyperChad SSE updates to push changes to all connected clients
                    // 3. Update the UI with the new data (new comments, PR state changes, etc.)
                    //
                    // For now, the callback just logs the event. The real-time update mechanism
                    // would need to integrate with HyperChad's state management and SSE system.
                });

                // Subscribe to this PR's events
                if let Err(e) = client.subscribe(pr_key, callback).await {
                    log::warn!("Failed to subscribe to PR webhook events: {e}");
                } else {
                    log::info!("Subscribed to webhook events for PR {owner}/{repo} #{number}");
                }
            }
            Err(e) => {
                log::warn!("Failed to connect to relay server: {e}");
            }
        }
    }

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

async fn resolve_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Post) {
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
    let resolved = req
        .query
        .get("resolved")
        .ok_or(RouteError::MissingQueryParam("resolved"))?
        .parse::<bool>()?;

    provider
        .resolve_comment(owner, repo, number, comment_id, resolved)
        .await?;

    let comment = provider
        .get_comment(owner, repo, number, comment_id, true)
        .await?;

    Ok(render_comment_thread(comment_id, &comment, 0, owner, repo, number).into())
}

async fn expand_comment_route(
    req: RouteRequest,
    provider: Arc<dyn GitProvider>,
) -> Result<Container, RouteError> {
    if !matches!(req.method, Method::Get) {
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

    let comment = provider
        .get_comment(owner, repo, number, comment_id, true)
        .await?;

    // Render with collapse ignored (temporarily expanded)
    Ok(render_comment_thread_expanded(
        comment_id, &comment, 0, owner, repo, number,
    ))
}

fn render_comment_thread_expanded(
    root_comment_id: u64,
    comment: &chadreview_pr_models::Comment,
    depth: usize,
    owner: &str,
    repo: &str,
    number: u64,
) -> Container {
    let margin_left = i32::try_from(depth * 20).unwrap_or(0);
    let is_root = depth == 0;

    container! {
        div
            id=(comment_thread_id(comment.id))
            class=(comment_class(comment.id))
            margin-left=(margin_left)
            border-left="2, #d0d7de"
            padding-left=12
            gap=12
        {
            (render_comment_item(comment, is_root, owner, repo, number))
            (render_reply_form(root_comment_id, comment.id, owner, repo, number))
            @for reply in &comment.replies {
                (render_comment_thread_expanded(root_comment_id, reply, depth + 1, owner, repo, number))
            }
        }
    }
    .into()
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
