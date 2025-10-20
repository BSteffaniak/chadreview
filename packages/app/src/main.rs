#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use std::sync::Arc;

use chadreview_app::routes;
use chadreview_github::GitHubProvider;

fn main() {
    println!("ChadReview - GitHub PR Review Tool");

    let auth_token =
        std::env::var("GITHUB_TOKEN").unwrap_or_else(|_| "dummy-token-for-compilation".to_string());

    let provider: Arc<dyn chadreview_git_provider::GitProvider> =
        Arc::new(GitHubProvider::new(auth_token));
    let _router = routes::create_router(&provider);

    println!("Router created successfully with 4 routes:");
    println!("  GET  /pr?owner=<owner>&repo=<repo>&number=<number>");
    println!("  POST /api/pr/comment?owner=<owner>&repo=<repo>&number=<number>");
    println!("  PUT  /api/comment/update?id=<id>");
    println!("  DELETE /api/comment/delete?id=<id>");
}
