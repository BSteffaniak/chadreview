# ChadReview - Execution Plan

## Executive Summary

ChadReview is a high-performance GitHub PR review tool built on the HyperChad framework, addressing critical limitations in GitHub's native interface: lack of auto-updating for file-level and inline comments, poor performance on large PRs, and a cluttered UI. The MVP delivers a focused single-PR view with real-time comment synchronization, efficient diff rendering, and essential comment interaction capabilities.

**Current Status:** 🔴 **Not Started** - Initial planning phase

**Completion Estimate:** ~0% complete - Specification phase

## Status Legend

- 🔴 **Critical** - Blocks core functionality
- 🟡 **Important** - Affects user experience or API design
- 🟢 **Minor** - Nice-to-have or polish items
- ✅ **Complete** - Fully implemented and validated
- 🟡 **In Progress** - Currently being worked on
- ❌ **Blocked** - Waiting on dependencies or design decisions

## Design Decisions (RESOLVED)

### MVP Scope ✅

- **Decision Point**: Start with single PR view only, defer PR list view to post-MVP
- **Rationale**: Focuses development on core value proposition (real-time comments + performance) without UI complexity of list/search features. User navigates via URL: `/pr/:owner/:repo/:number`
- **Alternatives Considered**: Full-featured app with PR list, search, and filters - rejected as too ambitious for MVP

### Diff View Strategy ✅

- **Decision Point**: Unified diff view only for MVP, side-by-side deferred
- **Rationale**: Simpler implementation, single-column layout better for mobile/narrow windows, most developers familiar with unified format from git CLI
- **Alternatives Considered**: Side-by-side as default - rejected due to complexity and horizontal space requirements

### Syntax Highlighting Location ✅

- **Decision Point**: Server-side syntax highlighting
- **Rationale**: Eliminates client-side JS parsing overhead, improves performance on large diffs, reduces bundle size, better for desktop app use case
- **Alternatives Considered**: Client-side highlighting - rejected due to performance concerns on large files

### Comment Display Strategy ✅

- **Decision Point**: Inline for line/file comments, separate section for general PR comments
- **Rationale**: Maintains context for code-related comments while keeping general discussion accessible without cluttering diff view
- **Alternatives Considered**: All comments in sidebar - rejected as it breaks code-comment visual proximity

### Authentication Approach ✅

- **Decision Point**: GitHub Personal Access Token initially, OAuth post-MVP
- **Rationale**: Simpler implementation, no callback URL/redirect flow needed, sufficient for power users, easier testing
- **Alternatives Considered**: OAuth first - deferred to allow faster MVP delivery

### HyperChad Backend Selection ✅

- **Decision Point**: HTML + VanillaJS as default, with Egui/FLTK support
- **Rationale**: Web deployment is primary use case, desktop support is bonus. HTML backend is most mature and performant for this use case.
- **Alternatives Considered**: Egui-only - rejected as web deployment is important for accessibility

### Git Provider Abstraction ✅

- **Decision Point**: Abstract git hosting provider behind a Rust trait (`GitProvider`), with GitHub as the only MVP implementation
- **Rationale**: Future-proofs the design for GitLab, Bitbucket, Gitea, and other platforms without architectural changes. Trait-based design enables testing with mocks and allows users to choose their preferred platform.
- **Alternatives Considered**: Hardcode GitHub-specific logic - rejected as it would require significant refactoring to support other providers later
- **MVP Scope**: Only `GitHubProvider` implementation required for MVP. Trait design must be validated against GitHub's API to ensure it generalizes well.

### Directory Structure Convention ✅

- **Decision Point**: Package directories named without `chadreview_` prefix (e.g., `packages/core/`), but crate names include prefix (e.g., `chadreview_core`)
- **Rationale**: Directory prefix is redundant when already in the `chadreview` repository. Crate names need prefix for global Rust package namespace and to avoid conflicts.
- **Pattern**: `packages/{simple_name}/` with `Cargo.toml` defining `name = "chadreview_{simple_name}"`

### HyperChad Dependency Strategy ✅

- **Decision Point**: Use git URL (`git = "https://github.com/MoosicBox/MoosicBox"`) for HyperChad dependencies instead of local path
- **Rationale**: Ensures we always get the latest HyperChad APIs and features from the upstream repository. Avoids issues with stale local checkouts and makes the build reproducible across different machines without requiring MoosicBox repo to be cloned locally.
- **Alternatives Considered**: Local path dependency - rejected as it requires specific directory structure and doesn't guarantee latest API
- **Note**: Can pin to specific commit with `rev = "abc123"` if stability becomes an issue during development

## Phase 1: Workspace and Package Setup 🔴 **NOT STARTED**

**Goal:** Create ChadReview workspace structure and core packages

**Status:** All tasks pending

### 1.1 Workspace Creation

- [ ] Create workspace root structure 🔴 **CRITICAL**

  - [ ] Create `Cargo.toml` workspace manifest:

    ```toml
    [workspace]
    members = [
        "packages/core",
        "packages/app",
    ]
    resolver = "2"

    [workspace.package]
    version = "0.1.0"
    edition = "2021"
    authors = ["Your Name <your.email@example.com>"]
    license = "MIT OR Apache-2.0"
    repository = "https://github.com/yourusername/chadreview"

    [workspace.dependencies]
    chadreview_core = { path = "packages/core", version = "0.1.0" }

    # External dependencies
    tokio = { version = "1", features = ["full"] }
    reqwest = { version = "0.11", features = ["json"] }
    serde = { version = "1", features = ["derive"] }
    serde_json = "1"
    chrono = { version = "0.4", features = ["serde"] }
    syntect = "5"
    anyhow = "1"
    thiserror = "1"
    async-trait = "0.1"

    # HyperChad framework - use git URL for latest API
    hyperchad = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_app = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_renderer = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_renderer_html = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_renderer_html_actix = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_renderer_vanilla_js = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_router = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_state = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    hyperchad_template = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master" }
    ```

  - [ ] Create `packages/` directory
  - [ ] Initialize git repository with `.gitignore`:

    ```
    /target
    Cargo.lock
    .env
    .idea/
    .vscode/
    *.swp
    *.swo
    *~
    ```

#### 1.1 Verification Checklist

- [ ] Workspace directory structure exists
- [ ] `Cargo.toml` has valid TOML syntax
- [ ] Git repository initialized
- [ ] `.gitignore` covers Rust artifacts
- [ ] Run `cargo metadata` (workspace recognized)

### 1.2 Core Package Creation

- [ ] Create `core` package 🔴 **CRITICAL**

  - [ ] Create `packages/core/` directory
  - [ ] Create `packages/core/src/` directory
  - [ ] Create `packages/core/src/lib.rs` with ONLY clippy configuration:

    ```rust
    #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
    #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
    #![allow(clippy::multiple_crate_versions)]

    ```

  - [ ] Create `packages/core/Cargo.toml`:

    ```toml
    [package]
    name = "chadreview_core"
    version = { workspace = true }
    edition = { workspace = true }
    authors = { workspace = true }
    license = { workspace = true }
    repository = { workspace = true }
    description = "Core domain logic for ChadReview PR review tool"
    readme = "README.md"
    keywords = ["github", "pull-request", "code-review"]
    categories = ["development-tools"]

    [dependencies]

    [features]
    default = ["github"]

    github = []
    gitlab = []
    bitbucket = []

    fail-on-warnings = []

    [dev-dependencies]
    ```

#### 1.2 Verification Checklist

- [ ] Directory structure exists at correct paths
- [ ] `Cargo.toml` has valid TOML syntax and follows workspace conventions
- [ ] `lib.rs` contains ONLY clippy configuration
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_core` (compiles with default features)
- [ ] Run `cargo build -p chadreview_core --no-default-features` (compiles with no features)
- [ ] Run `cargo machete` (zero unused dependencies)

### 1.3 App Package Creation

- [ ] Create `app` package 🔴 **CRITICAL**

  - [ ] Create `packages/app/` directory
  - [ ] Create `packages/app/src/` directory
  - [ ] Create `packages/app/src/main.rs` with minimal bootstrap:

    ```rust
    #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
    #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
    #![allow(clippy::multiple_crate_versions)]

    fn main() {
        println!("ChadReview - GitHub PR Review Tool");
    }
    ```

  - [ ] Create `packages/app/Cargo.toml`:

    ```toml
    [package]
    name = "chadreview_app"
    version = { workspace = true }
    edition = { workspace = true }
    authors = { workspace = true }
    license = { workspace = true }
    repository = { workspace = true }
    description = "HyperChad-based application for ChadReview"
    readme = "README.md"

    [[bin]]
    name = "chadreview"
    path = "src/main.rs"

    [dependencies]
    chadreview_core = { workspace = true }

    [features]
    default = ["html", "vanilla-js"]

    html = []
    vanilla-js = []
    egui-wgpu = []
    egui-glow = []
    fltk = []

    actix = []
    lambda = []

    dev = []
    fail-on-warnings = ["chadreview_core/fail-on-warnings"]

    [dev-dependencies]
    ```

#### 1.3 Verification Checklist

- [ ] Directory structure exists at correct paths
- [ ] `Cargo.toml` has valid TOML syntax
- [ ] `main.rs` compiles and runs
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_app` (compiles)
- [ ] Run `cargo run -p chadreview_app` (prints hello message)
- [ ] Run `cargo machete` (zero unused dependencies)

## Phase 2: Core Domain Models and Provider Abstraction 🔴 **NOT STARTED**

**Goal:** Implement core data types for PR, diffs, and comments, plus GitProvider trait

**Status:** All tasks pending

### 2.1 PR and Metadata Models

**CRITICAL NOTES:**

- Use `chrono::DateTime<Utc>` for timestamps
- Use `BTreeMap/BTreeSet` for any collections
- All types must derive `Debug, Clone, serde::Serialize, serde::Deserialize`

- [ ] Add required dependencies to `packages/core/Cargo.toml` 🔴 **CRITICAL**

  - [ ] Add to `[dependencies]`:
    ```toml
    serde = { workspace = true }
    chrono = { workspace = true }
    async-trait = { workspace = true }
    ```
  - [ ] **VERIFICATION**: Run `cargo tree -p chadreview_core` to confirm dependencies added

- [ ] Create `src/models.rs` with core types 🔴 **CRITICAL**

  - [ ] Add `pub mod models;` to `lib.rs`
  - [ ] Implement complete model definitions:

    ```rust
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PullRequest {
        pub number: u64,
        pub owner: String,
        pub repo: String,
        pub title: String,
        pub description: String,
        pub author: User,
        pub state: PrState,
        pub draft: bool,
        pub base_branch: String,
        pub head_branch: String,
        pub labels: Vec<Label>,
        pub assignees: Vec<User>,
        pub reviewers: Vec<User>,
        pub commits: Vec<Commit>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub provider: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum PrState {
        Open,
        Closed,
        Merged,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct User {
        pub login: String,
        pub avatar_url: String,
        pub html_url: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Label {
        pub name: String,
        pub color: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Commit {
        pub sha: String,
        pub message: String,
        pub author: User,
        pub committed_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiffFile {
        pub filename: String,
        pub status: FileStatus,
        pub additions: usize,
        pub deletions: usize,
        pub hunks: Vec<DiffHunk>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum FileStatus {
        Added,
        Modified,
        Deleted,
        Renamed,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiffHunk {
        pub old_start: usize,
        pub old_lines: usize,
        pub new_start: usize,
        pub new_lines: usize,
        pub lines: Vec<DiffLine>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiffLine {
        pub line_type: LineType,
        pub old_line_number: Option<usize>,
        pub new_line_number: Option<usize>,
        pub content: String,
        pub highlighted_html: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum LineType {
        Addition,
        Deletion,
        Context,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Comment {
        pub id: u64,
        pub author: User,
        pub body: String,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub comment_type: CommentType,
        pub replies: Vec<Comment>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum CommentType {
        General,
        FileLevelComment { path: String },
        LineLevelComment { path: String, line: usize },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateComment {
        pub body: String,
        pub comment_type: CommentType,
        pub in_reply_to: Option<u64>,
    }
    ```

  - [ ] Add unit tests for model serialization:

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_pr_state_serialization() {
            let state = PrState::Open;
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, r#""Open"#);

            let deserialized: PrState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, PrState::Open);
        }

        #[test]
        fn test_comment_type_serialization() {
            let ct = CommentType::LineLevelComment {
                path: "src/main.rs".to_string(),
                line: 42,
            };
            let json = serde_json::to_string(&ct).unwrap();
            let deserialized: CommentType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, ct);
        }
    }
    ```

#### 2.1 Verification Checklist

- [ ] All models compile without errors
- [ ] All types derive required traits
- [ ] Serialization tests pass
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_core` (compiles)
- [ ] Run `cargo test -p chadreview_core` (all tests pass)
- [ ] Run `cargo machete` (all dependencies used)

## Phase 3: Git Provider Trait and GitHub Implementation 🔴 **NOT STARTED**

**Goal:** Define abstract `GitProvider` trait and implement GitHub provider

**Status:** All tasks pending

### 3.1 Git Provider Trait Definition

- [ ] Create `src/provider.rs` with `GitProvider` trait 🔴 **CRITICAL**

  - [ ] Add `pub mod provider;` to `lib.rs`
  - [ ] Define complete `GitProvider` trait:

    ```rust
    use crate::models::*;
    use anyhow::Result;

    #[async_trait::async_trait]
    pub trait GitProvider: Send + Sync {
        async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest>;

        async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>>;

        async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>>;

        async fn create_comment(
            &self,
            owner: &str,
            repo: &str,
            number: u64,
            comment: CreateComment,
        ) -> Result<Comment>;

        async fn update_comment(&self, comment_id: u64, body: String) -> Result<Comment>;

        async fn delete_comment(&self, comment_id: u64) -> Result<()>;

        fn provider_name(&self) -> &str;

        fn supports_drafts(&self) -> bool {
            false
        }

        fn supports_line_comments(&self) -> bool {
            true
        }
    }
    ```

#### 3.1 Verification Checklist

- [ ] Trait compiles without errors
- [ ] All methods have appropriate signatures
- [ ] Documentation comments added to all trait methods
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_core` (compiles)

### 3.2 GitHub Provider Implementation

**CRITICAL NOTES:**

- Use `reqwest` for HTTP client with connection pooling
- Use `anyhow::Result` for error handling
- Implement rate limiting before making requests
- All API calls are async

- [ ] Add required dependencies 🔴 **CRITICAL**

  - [ ] Add to `packages/core/Cargo.toml` dependencies:
    ```toml
    reqwest = { workspace = true }
    anyhow = { workspace = true }
    thiserror = { workspace = true }
    tokio = { workspace = true }
    serde_json = { workspace = true }
    ```
  - [ ] Add to `packages/core/Cargo.toml` dev-dependencies:
    ```toml
    wiremock = "0.5"
    tokio-test = "0.4"
    ```
  - [ ] **VERIFICATION**: Run `cargo tree -p chadreview_core`

- [ ] Create `src/github.rs` with GitHub provider implementation 🔴 **CRITICAL**

  - [ ] Add `#[cfg(feature = "github")] pub mod github;` to `lib.rs`
  - [ ] Implement `GitHubProvider` struct:

    ```rust
    use crate::models::*;
    use crate::provider::GitProvider;
    use anyhow::Result;

    pub struct GitHubProvider {
        http_client: reqwest::Client,
        auth_token: String,
        base_url: String,
    }

    impl GitHubProvider {
        pub fn new(auth_token: String) -> Self {
            Self {
                http_client: reqwest::Client::new(),
                auth_token,
                base_url: "https://api.github.com".to_string(),
            }
        }

        pub fn with_base_url(mut self, base_url: String) -> Self {
            self.base_url = base_url;
            self
        }
    }

    #[async_trait::async_trait]
    impl GitProvider for GitHubProvider {
        async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
            let url = format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);
            let response = self.http_client
                .get(&url)
                .bearer_auth(&self.auth_token)
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await?;

            if !response.status().is_success() {
                anyhow::bail!("GitHub API error: {}", response.status());
            }

            let pr_data: serde_json::Value = response.json().await?;

            Ok(PullRequest {
                number: pr_data["number"].as_u64().unwrap(),
                owner: owner.to_string(),
                repo: repo.to_string(),
                title: pr_data["title"].as_str().unwrap().to_string(),
                description: pr_data["body"].as_str().unwrap_or("").to_string(),
                author: parse_user(&pr_data["user"]),
                state: parse_pr_state(&pr_data),
                draft: pr_data["draft"].as_bool().unwrap_or(false),
                base_branch: pr_data["base"]["ref"].as_str().unwrap().to_string(),
                head_branch: pr_data["head"]["ref"].as_str().unwrap().to_string(),
                labels: parse_labels(&pr_data["labels"]),
                assignees: parse_users(&pr_data["assignees"]),
                reviewers: parse_users(&pr_data["requested_reviewers"]),
                commits: vec![],
                created_at: parse_datetime(pr_data["created_at"].as_str().unwrap()),
                updated_at: parse_datetime(pr_data["updated_at"].as_str().unwrap()),
                provider: "github".to_string(),
            })
        }

        async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>> {
            todo!("Implement in Phase 4")
        }

        async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>> {
            todo!("Implement in Phase 5")
        }

        async fn create_comment(&self, owner: &str, repo: &str, number: u64, comment: CreateComment) -> Result<Comment> {
            todo!("Implement in Phase 6")
        }

        async fn update_comment(&self, comment_id: u64, body: String) -> Result<Comment> {
            todo!("Implement in Phase 6")
        }

        async fn delete_comment(&self, comment_id: u64) -> Result<()> {
            todo!("Implement in Phase 6")
        }

        fn provider_name(&self) -> &str {
            "github"
        }

        fn supports_drafts(&self) -> bool {
            true
        }
    }

    fn parse_user(value: &serde_json::Value) -> User {
        User {
            login: value["login"].as_str().unwrap().to_string(),
            avatar_url: value["avatar_url"].as_str().unwrap().to_string(),
            html_url: value["html_url"].as_str().unwrap().to_string(),
        }
    }

    fn parse_users(value: &serde_json::Value) -> Vec<User> {
        value.as_array()
            .map(|arr| arr.iter().map(parse_user).collect())
            .unwrap_or_default()
    }

    fn parse_pr_state(value: &serde_json::Value) -> PrState {
        match value["state"].as_str().unwrap() {
            "open" => PrState::Open,
            "closed" if value["merged"].as_bool().unwrap_or(false) => PrState::Merged,
            "closed" => PrState::Closed,
            _ => PrState::Open,
        }
    }

    fn parse_labels(value: &serde_json::Value) -> Vec<Label> {
        value.as_array()
            .map(|arr| arr.iter().map(|v| Label {
                name: v["name"].as_str().unwrap().to_string(),
                color: v["color"].as_str().unwrap().to_string(),
            }).collect())
            .unwrap_or_default()
    }

    fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
    ```

  - [ ] Add integration tests with wiremock:

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        #[tokio::test]
        async fn test_get_pr_success() {
            let mock_server = MockServer::start().await;

            let pr_json = serde_json::json!({
                "number": 123,
                "title": "Test PR",
                "body": "Test description",
                "state": "open",
                "draft": false,
                "user": {
                    "login": "testuser",
                    "avatar_url": "https://example.com/avatar.png",
                    "html_url": "https://github.com/testuser"
                },
                "base": { "ref": "main" },
                "head": { "ref": "feature-branch" },
                "labels": [],
                "assignees": [],
                "requested_reviewers": [],
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2025-01-02T00:00:00Z",
                "merged": false
            });

            Mock::given(method("GET"))
                .and(path("/repos/owner/repo/pulls/123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&pr_json))
                .mount(&mock_server)
                .await;

            let client = GitHubProvider::new("test-token".to_string())
                .with_base_url(mock_server.uri());

            let pr = client.get_pr("owner", "repo", 123).await.unwrap();

            assert_eq!(pr.number, 123);
            assert_eq!(pr.title, "Test PR");
            assert_eq!(pr.state, PrState::Open);
            assert_eq!(pr.author.login, "testuser");
            assert_eq!(pr.provider, "github");
        }

        #[tokio::test]
        async fn test_get_pr_merged_state() {
            let mock_server = MockServer::start().await;

            let pr_json = serde_json::json!({
                "number": 456,
                "title": "Merged PR",
                "body": "",
                "state": "closed",
                "merged": true,
                "draft": false,
                "user": {
                    "login": "author",
                    "avatar_url": "https://example.com/avatar.png",
                    "html_url": "https://github.com/author"
                },
                "base": { "ref": "main" },
                "head": { "ref": "feature" },
                "labels": [],
                "assignees": [],
                "requested_reviewers": [],
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2025-01-02T00:00:00Z"
            });

            Mock::given(method("GET"))
                .and(path("/repos/owner/repo/pulls/456"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&pr_json))
                .mount(&mock_server)
                .await;

            let client = GitHubProvider::new("test-token".to_string())
                .with_base_url(mock_server.uri());

            let pr = client.get_pr("owner", "repo", 456).await.unwrap();

            assert_eq!(pr.state, PrState::Merged);
        }
    }
    ```

#### 3.2 Verification Checklist

- [ ] GitHub provider compiles without errors
- [ ] `get_pr` method fully implemented with parsing
- [ ] Integration tests with wiremock pass
- [ ] Error handling for non-200 responses works
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_core` (compiles)
- [ ] Run `cargo test -p chadreview_core` (all tests pass)
- [ ] Run `cargo machete` (all dependencies used)

## Phase 4: Diff Parsing and Syntax Highlighting 🔴 **NOT STARTED**

**Goal:** Parse GitHub diff format and add server-side syntax highlighting

**Status:** All tasks pending

### 4.1 Diff Parser Implementation

- [ ] Add syntect dependency for syntax highlighting 🔴 **CRITICAL**

  - [ ] Add to `packages/core/Cargo.toml`:
    ```toml
    syntect = { workspace = true }
    ```

- [ ] Implement `get_diff` in GitHub provider 🔴 **CRITICAL**

  - [ ] Update `github.rs` with diff fetching and parsing
  - [ ] Parse unified diff format from GitHub API
  - [ ] Convert to `DiffFile` and `DiffHunk` structures
  - [ ] Add tests for diff parsing

- [ ] Create `src/syntax.rs` for syntax highlighting 🔴 **CRITICAL**

  - [ ] Add `pub mod syntax;` to `lib.rs`
  - [ ] Implement `SyntaxHighlighter` struct
  - [ ] Add language detection from file extensions
  - [ ] Generate highlighted HTML for each line
  - [ ] Add tests for highlighting various languages

#### 4.1 Verification Checklist

- [ ] Diff parsing handles all file statuses (added/modified/deleted/renamed)
- [ ] Syntax highlighting works for common languages (Rust, JS, Python, etc.)
- [ ] Fallback to plain text for unknown languages
- [ ] HTML output is properly escaped
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo test -p chadreview_core` (all tests pass)
- [ ] Run `cargo machete` (all dependencies used)

## Phase 5: Comment Fetching and Threading 🔴 **NOT STARTED**

**Goal:** Fetch and organize PR comments into threaded structure

**Status:** All tasks pending

### 5.1 Comment API Implementation

- [ ] Implement `get_comments` in GitHub provider 🔴 **CRITICAL**

  - [ ] Fetch review comments (line-level)
  - [ ] Fetch issue comments (general PR comments)
  - [ ] Fetch review thread comments (replies)
  - [ ] Organize into nested `Comment` structure
  - [ ] Add tests for comment parsing and threading

#### 5.1 Verification Checklist

- [ ] All comment types fetched correctly
- [ ] Nested replies properly structured
- [ ] Comment types correctly identified (general/file-level/line-level)
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo test -p chadreview_core` (all tests pass)
- [ ] Run `cargo machete` (all dependencies used)

## Phase 6: Comment Creation and Mutation 🔴 **NOT STARTED**

**Goal:** Implement comment create, update, and delete operations

**Status:** All tasks pending

### 6.1 Comment CRUD Operations

- [ ] Implement `create_comment` 🔴 **CRITICAL**

  - [ ] Handle line-level comment creation
  - [ ] Handle file-level comment creation
  - [ ] Handle general PR comment creation
  - [ ] Handle replies to existing comments
  - [ ] Add tests

- [ ] Implement `update_comment` 🔴 **CRITICAL**

  - [ ] Update comment body via GitHub API
  - [ ] Add tests

- [ ] Implement `delete_comment` 🔴 **CRITICAL**

  - [ ] Delete comment via GitHub API
  - [ ] Add tests

#### 6.1 Verification Checklist

- [ ] All CRUD operations work correctly
- [ ] Proper error handling for unauthorized operations
- [ ] Tests cover success and failure cases
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_core -- -D warnings` (zero warnings)
- [ ] Run `cargo test -p chadreview_core` (all tests pass)
- [ ] Run `cargo machete` (all dependencies used)

## Phase 7: HyperChad Application Setup 🔴 **NOT STARTED**

**Goal:** Set up HyperChad application structure with routing

**Status:** All tasks pending

### 7.1 HyperChad Integration

- [ ] Add HyperChad dependencies to `packages/app/Cargo.toml` 🔴 **CRITICAL**

  - [ ] Verify HyperChad git dependency resolves correctly
  - [ ] Add HyperChad packages:
    ```toml
    hyperchad = { workspace = true }
    hyperchad_app = { workspace = true }
    hyperchad_renderer = { workspace = true }
    hyperchad_renderer_html = { workspace = true, optional = true }
    hyperchad_renderer_vanilla_js = { workspace = true, optional = true }
    hyperchad_router = { workspace = true }
    hyperchad_state = { workspace = true }
    hyperchad_template = { workspace = true }
    tokio = { workspace = true }
    anyhow = { workspace = true }
    ```

- [ ] Create application structure 🔴 **CRITICAL**

  - [ ] Create `src/state.rs` for application state
  - [ ] Create `src/routes.rs` for route handlers
  - [ ] Create `src/components/` directory for UI components
  - [ ] Update `main.rs` with HyperChad initialization

- [ ] Implement basic routing 🔴 **CRITICAL**

  - [ ] Route: `GET /pr/:owner/:repo/:number` - Main PR view
  - [ ] Route: `POST /api/pr/:owner/:repo/:number/comment` - Create comment
  - [ ] Route: `PUT /api/comment/:id` - Update comment
  - [ ] Route: `DELETE /api/comment/:id` - Delete comment

#### 7.1 Verification Checklist

- [ ] HyperChad application compiles
- [ ] Server starts and listens on configured port
- [ ] Routes registered correctly
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_app` (compiles)
- [ ] Run `cargo run -p chadreview_app` (starts server)

## Phase 8: UI Components - PR Header 🔴 **NOT STARTED**

**Goal:** Render PR metadata (title, description, status, labels, etc.)

**Status:** All tasks pending

### 8.1 PR Header Component

- [ ] Create `src/components/pr_header.rs` 🔴 **CRITICAL**

  - [ ] Render PR title
  - [ ] Render PR description (markdown-to-HTML)
  - [ ] Render PR state badge (open/closed/merged)
  - [ ] Render draft indicator
  - [ ] Render author info with avatar
  - [ ] Render labels
  - [ ] Render assignees and reviewers
  - [ ] Render branch information
  - [ ] Render timestamps (created, updated)

- [ ] Add CSS styling 🟡 **IMPORTANT**

  - [ ] Create `assets/styles.css`
  - [ ] Style PR header for clean, focused layout
  - [ ] Ensure responsive design

#### 8.1 Verification Checklist

- [ ] PR header renders all metadata correctly
- [ ] Styling is clean and uncluttered
- [ ] Component updates via SSE when PR changes
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
- [ ] Run `cargo build -p chadreview_app` (compiles)
- [ ] Manual testing: View real PR, verify all fields display

## Phase 9: UI Components - Diff Viewer 🔴 **NOT STARTED**

**Goal:** Render unified diff with syntax highlighting

**Status:** All tasks pending

### 9.1 Diff Viewer Component

- [ ] Create `src/components/diff_viewer.rs` 🔴 **CRITICAL**

  - [ ] Render file list with status indicators
  - [ ] Render each file's diff hunks
  - [ ] Render line numbers (old and new)
  - [ ] Render syntax-highlighted code
  - [ ] Render addition/deletion/context line indicators
  - [ ] Make files collapsible/expandable
  - [ ] Add file stats (additions/deletions count)

- [ ] Add diff-specific CSS 🟡 **IMPORTANT**

  - [ ] Style additions (green background)
  - [ ] Style deletions (red background)
  - [ ] Style context lines (neutral)
  - [ ] Style line numbers
  - [ ] Ensure code uses monospace font

#### 9.1 Verification Checklist

- [ ] Diff renders correctly for all file statuses
- [ ] Syntax highlighting displays properly
- [ ] Line numbers align correctly
- [ ] Large diffs render without performance issues
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
- [ ] Manual testing: View large PR (50+ files), verify performance

## Phase 10: UI Components - Comment Threads 🔴 **NOT STARTED**

**Goal:** Render inline comment threads with create/reply/edit/delete

**Status:** All tasks pending

### 10.1 Comment Thread Component

- [ ] Create `src/components/comment_thread.rs` 🔴 **CRITICAL**

  - [ ] Render line-level comments under code lines
  - [ ] Render file-level comments at top of file diff
  - [ ] Render nested replies with indentation
  - [ ] Display comment author, timestamp, body
  - [ ] Add "Reply" button for each comment
  - [ ] Add "Edit" button for user's own comments
  - [ ] Add "Delete" button for user's own comments
  - [ ] Add "+ Add comment" button on each code line

- [ ] Implement comment form 🔴 **CRITICAL**

  - [ ] Textarea for comment body
  - [ ] Submit and cancel buttons
  - [ ] Form validation
  - [ ] Loading state during API call
  - [ ] Error handling and display

- [ ] Wire up comment actions 🔴 **CRITICAL**

  - [ ] Create comment: POST to `/api/pr/:owner/:repo/:number/comment`
  - [ ] Update comment: PUT to `/api/comment/:id`
  - [ ] Delete comment: DELETE to `/api/comment/:id`
  - [ ] Handle API responses and errors
  - [ ] Optimistic UI updates with rollback on error

- [ ] Add VanillaJS for interactions 🟡 **IMPORTANT**

  - [ ] Create `assets/comments.js`
  - [ ] Handle "Reply" button clicks (show/hide form)
  - [ ] Handle form submissions (fetch API)
  - [ ] Handle "Edit" button (inline editing)
  - [ ] Handle "Delete" with confirmation

#### 10.1 Verification Checklist

- [ ] Comments render inline correctly
- [ ] Nested replies display with proper indentation
- [ ] Create comment works for all comment types
- [ ] Reply to comment creates nested thread
- [ ] Edit comment updates in place
- [ ] Delete comment removes from UI
- [ ] All actions update via SSE for other viewers
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
- [ ] Manual testing: Create/reply/edit/delete comments on real PR

## Phase 11: UI Components - General Comments 🔴 **NOT STARTED**

**Goal:** Render general PR comments in separate section

**Status:** All tasks pending

### 11.1 General Comments Component

- [ ] Create `src/components/general_comments.rs` 🔴 **CRITICAL**

  - [ ] Render general comments section below diff viewer
  - [ ] Display all general PR comments
  - [ ] Reuse comment thread component for replies
  - [ ] Add form to create new general comment
  - [ ] Wire up create/edit/delete actions

#### 11.1 Verification Checklist

- [ ] General comments display in separate section
- [ ] Comments are distinct from inline code comments
- [ ] All CRUD operations work
- [ ] Real-time updates via SSE
- [ ] Run `cargo fmt` (format code)
- [ ] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)

## Phase 12: End-to-End Testing and Polish 🔴 **NOT STARTED**

**Goal:** Comprehensive testing and UI polish for MVP release

**Status:** All tasks pending

### 12.1 Integration Testing

- [ ] Test full PR view workflow 🔴 **CRITICAL**

  - [ ] Fetch and display real PR from GitHub
  - [ ] Verify all metadata renders correctly
  - [ ] Verify diff renders with syntax highlighting
  - [ ] Verify all comments display inline
  - [ ] Create, edit, delete comments via UI
  - [ ] Verify real-time updates work across multiple clients

- [ ] Test error handling 🔴 **CRITICAL**

  - [ ] Invalid GitHub token
  - [ ] Non-existent PR
  - [ ] Network failures
  - [ ] API rate limiting
  - [ ] Large PR performance

### 12.2 UI Polish

- [ ] Refine CSS for professional appearance 🟡 **IMPORTANT**

  - [ ] Consistent spacing and typography
  - [ ] Clear visual hierarchy
  - [ ] Accessible color contrast
  - [ ] Mobile-responsive layout

- [ ] Add loading states 🟡 **IMPORTANT**

  - [ ] Loading spinner while fetching PR
  - [ ] Skeleton screens for initial render
  - [ ] Loading indicators for comment actions

- [ ] Add error states 🟡 **IMPORTANT**

  - [ ] Error messages for failed API calls
  - [ ] Helpful error text with recovery actions
  - [ ] Toast notifications for transient errors

### 12.3 Documentation

- [ ] Write README.md 🟡 **IMPORTANT**

  - [ ] Installation instructions
  - [ ] Configuration (GitHub token setup)
  - [ ] Usage guide with screenshots
  - [ ] Feature list
  - [ ] Known limitations

- [ ] Add inline code documentation 🟡 **IMPORTANT**

  - [ ] Document all public APIs
  - [ ] Add usage examples
  - [ ] Document environment variables

#### 12.1-12.3 Verification Checklist

- [ ] All integration tests pass
- [ ] Manual testing on multiple real PRs successful
- [ ] Error handling gracefully degrades
- [ ] UI is polished and professional
- [ ] Documentation is complete and accurate
- [ ] Run `cargo fmt` (workspace-wide)
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings` (zero warnings)
- [ ] Run `cargo build --release` (optimized build succeeds)
- [ ] Run `cargo test --all` (all tests pass)
- [ ] Run `cargo machete` (no unused dependencies)

## Success Criteria

The following criteria must be met for the MVP to be considered successful:

- [ ] Can view any public/private GitHub PR (with valid token)
- [ ] PR metadata displays completely (title, description, author, labels, assignees, reviewers, branches, commits, dates, state)
- [ ] Unified diff viewer with server-side syntax highlighting works flawlessly
- [ ] All comments (line-level, file-level, general) display inline and in dedicated section
- [ ] Real-time comment updates work via HyperChad SSE (no manual refresh needed)
- [ ] Can create new comments on specific code lines
- [ ] Can create general PR comments
- [ ] Can reply to existing comment threads
- [ ] Can edit own comments
- [ ] Can delete own comments
- [ ] Large PRs (100+ files, 10k+ lines) render efficiently
- [ ] Clean, uncluttered UI focused on code and discussions
- [ ] Works on both web (HTML+VanillaJS) and desktop (Egui) backends
- [ ] Zero clippy warnings with fail-on-warnings enabled
- [ ] Test coverage > 80% for core domain logic
- [ ] Comprehensive documentation for setup and usage

## Technical Decisions

### Language and Framework

- **Rust** with standard toolchain (edition 2021)
- **BTreeMap/BTreeSet** for all collections (never HashMap/HashSet)
- **Workspace dependencies** using `{ workspace = true }`
- **Underscore naming** for all packages
- **HyperChad** framework from git: `https://github.com/MoosicBox/MoosicBox`

### Architecture Patterns

- **Trait-based git provider**: Enables mocking for tests, future alternative implementations (GitLab, Bitbucket, etc.)
- **Server-side syntax highlighting**: Better performance, simpler client, smaller bundle size
- **SSE-based real-time updates**: Handled automatically by HyperChad, no manual WebSocket code
- **Unified diff only (MVP)**: Simpler implementation, defer side-by-side to post-MVP

### Key Design Principles

1. **Performance through simplicity**: Leverage native HTML rendering, avoid heavy JS frameworks, rely on browser efficiency
2. **Real-time by default**: All comment updates push via SSE, no manual refresh UX pattern
3. **HyperChad abstraction**: Let HyperChad handle server/client communication, rendering backend selection, and state sync
4. **Focus on core value**: MVP delivers auto-updating comments and large PR performance, defer advanced features

## Risk Mitigation

### High-Risk Areas

1. **GitHub API Rate Limiting**

   - Risk: Exceeding 5,000 req/hour limit, especially with polling for updates
   - Mitigation: Implement rate limit tracking, cache PR data aggressively, use conditional requests (ETags), batch comment fetches

2. **Large PR Performance**

   - Risk: PRs with thousands of files/lines may overwhelm rendering
   - Mitigation: Rely on browser HTML efficiency, defer virtualization unless proven necessary, profile on real-world large PRs early

3. **HyperChad Learning Curve**

   - Risk: HyperChad may have undocumented quirks or limitations
   - Mitigation: Reference examples in the MoosicBox repository (packages/app/native, packages/marketing_site), consult HyperChad documentation and source code, reach out to maintainers early if issues arise

4. **Comment Threading Complexity**

   - Risk: GitHub's comment API has multiple endpoints (review comments, issue comments, review threads) that must be unified
   - Mitigation: Thorough testing with various comment scenarios, use wiremock to simulate all comment types

5. **XSS Vulnerabilities**

   - Risk: Rendering user-generated comment bodies could expose XSS attacks
   - Mitigation: Always HTML-escape comment bodies, use markdown parser with safe defaults, security audit before release
