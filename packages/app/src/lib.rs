#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::OnceLock;

use hyperchad::renderer::Renderer;

pub mod actions;
pub mod events;
#[cfg(feature = "local-git")]
pub mod local_routes;
pub mod routes;

/// Global renderer instance for rendering UI components and pushing partial view updates.
pub static RENDERER: OnceLock<Box<dyn Renderer>> = OnceLock::new();

/// Module for pushing partial view updates via SSE.
#[cfg(feature = "local-git")]
pub mod sse {
    use std::collections::HashSet;

    use chadreview_app_ui::local_comments::{
        ai_status_str_id, local_thread_replies_id, render_ai_status_container,
        render_thread_replies,
    };
    use chadreview_local_comment_models::{AiExecutionStatus, LocalComment};
    use hyperchad::renderer::View;
    use switchy::uuid::Uuid;

    use crate::RENDERER;

    /// Push an AI status update to the client via SSE.
    ///
    /// This sends a partial view update that targets the AI status container
    /// for the specified comment, replacing its contents with the new status.
    ///
    /// Any errors are logged but don't cause the function to fail.
    pub async fn push_ai_status_update(comment_id: Uuid, status: &AiExecutionStatus) {
        let Some(renderer) = RENDERER.get() else {
            log::warn!("RENDERER not initialized, cannot push SSE update");
            return;
        };

        let container = render_ai_status_container(comment_id, status);
        let view = View::builder().with_fragment(container).build();

        renderer
            .render(view)
            .await
            .inspect(|()| {
                log::debug!(
                    "Pushed AI status update for comment {}: {:?}",
                    comment_id,
                    ai_status_str_id(comment_id)
                );
            })
            .inspect_err(|e| {
                log::error!("Failed to push AI status update for comment {comment_id}: {e:?}");
            })
            .ok();
    }

    /// Push updated thread replies to the client via SSE.
    ///
    /// This sends a partial view update that targets the replies container
    /// for the specified thread, replacing its contents with all current replies.
    /// Used when a new reply is added (e.g., AI response).
    ///
    /// Any errors are logged but don't cause the function to fail.
    pub async fn push_thread_replies(
        thread_id: Uuid,
        replies: &[LocalComment],
        repo_path: &str,
        viewed_reply_ids: &HashSet<Uuid>,
    ) {
        let Some(renderer) = RENDERER.get() else {
            log::warn!("RENDERER not initialized, cannot push SSE update");
            return;
        };

        let container = render_thread_replies(thread_id, replies, repo_path, viewed_reply_ids);
        let view = View::builder().with_fragment(container).build();

        renderer
            .render(view)
            .await
            .inspect(|()| {
                log::debug!(
                    "Pushed thread replies update for thread {}: {:?}",
                    thread_id,
                    local_thread_replies_id(thread_id)
                );
            })
            .inspect_err(|e| {
                log::error!("Failed to push thread replies update for thread {thread_id}: {e:?}");
            })
            .ok();
    }
}

#[cfg(feature = "assets")]
pub mod assets {
    use std::{path::PathBuf, sync::LazyLock};

    use hyperchad::renderer;

    static CARGO_MANIFEST_DIR: LazyLock<Option<std::path::PathBuf>> =
        LazyLock::new(|| std::option_env!("CARGO_MANIFEST_DIR").map(Into::into));

    static ASSETS_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        CARGO_MANIFEST_DIR.as_ref().map_or_else(
            || <PathBuf as std::str::FromStr>::from_str("public").unwrap(),
            |dir| dir.join("public"),
        )
    });

    pub static ASSETS: LazyLock<Vec<renderer::assets::StaticAssetRoute>> = LazyLock::new(|| {
        vec![
            #[cfg(feature = "vanilla-js")]
            renderer::assets::StaticAssetRoute {
                route: format!(
                    "/js/{}",
                    hyperchad::renderer_vanilla_js::SCRIPT_NAME_HASHED.as_str()
                ),
                target: renderer::assets::AssetPathTarget::FileContents(
                    hyperchad::renderer_vanilla_js::SCRIPT.as_bytes().into(),
                ),
            },
            renderer::assets::StaticAssetRoute {
                route: "/public".to_string(),
                target: ASSETS_DIR.clone().try_into().unwrap(),
            },
        ]
    });
}
