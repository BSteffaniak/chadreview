#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::Arc;

use chadreview_app::routes;
use chadreview_github::GitHubProvider;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    moosicbox_logging::init(None, None).expect("Failed to initialize logging");

    println!("ChadReview - GitHub PR Review Tool");

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let mut github_provider = GitHubProvider::new();
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        github_provider = github_provider.with_token(token);
    }

    let provider: Arc<dyn chadreview_git_provider::GitProvider> = Arc::new(github_provider);

    // Get relay URL if provided (actual connection will happen when first PR is viewed)
    let relay_url = std::env::var("RELAY_URL").ok();

    if relay_url.is_some() {
        println!(
            "RELAY_URL configured. Real-time webhook updates will be enabled when viewing PRs."
        );
    } else {
        println!("RELAY_URL not set. Real-time updates will be disabled.");
        println!("Set RELAY_URL environment variable to enable real-time webhook updates.");
    }

    #[allow(unused_mut)]
    let mut router = routes::create_router(&provider, relay_url);

    // Add local git routes if the feature is enabled
    #[cfg(feature = "local-git")]
    {
        let git_backend = Arc::new(chadreview_git_backend_git2::Git2Backend);
        router = chadreview_app::local_routes::add_local_routes(router, git_backend);
        println!("Local git diff support enabled.");
    }

    println!("Router created with routes:");
    println!("  GET  /pr?owner=<owner>&repo=<repo>&number=<number>");
    println!("  POST /api/pr/comment?owner=<owner>&repo=<repo>&number=<number>");
    println!("  PUT  /api/comment/update?id=<id>");
    println!("  DELETE /api/comment/delete?id=<id>");
    #[cfg(feature = "local-git")]
    println!("  GET  /local[?repo=<path>&base=<ref>&head=<ref>&...]");

    let runtime = switchy::unsync::runtime::Runtime::new();
    let handle = runtime.handle();

    #[allow(unused_mut)]
    let mut builder = hyperchad::app::AppBuilder::new()
        .with_title("ChadReview - GitHub PR Review Tool".to_string())
        .with_description("A high-performance PR review tool built with HyperChad".to_string())
        .with_router(router)
        .with_runtime_handle(handle);

    #[cfg(feature = "assets")]
    for asset in chadreview_app::assets::ASSETS.iter().cloned() {
        log::trace!("chadreview_app: adding static asset route: {asset:?}");
        builder = builder.with_static_asset_route_result(asset).unwrap();
    }

    let app = builder.build_default()?;

    // Set the global renderer for pushing partial view updates (SSE)
    if chadreview_app::RENDERER
        .set(Box::new(app.renderer.clone()))
        .is_err()
    {
        log::warn!("RENDERER was already initialized");
    }

    println!("\nStarting server at http://{host}:{port}");
    println!("Press Ctrl+C to stop\n");

    app.run()
        .map_err(|e| format!("Failed to run server: {e}"))?;

    Ok(())
}
