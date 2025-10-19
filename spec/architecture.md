# ChadReview Architecture

## System Overview

ChadReview is a modern GitHub PR review tool that addresses the fundamental limitations of GitHub's native review interface. Built on the HyperChad framework, it provides real-time comment synchronization across all comment types (not just top-level review comments), exceptional performance when viewing large diffs, and a clean, uncluttered interface focused on code and discussions.

The application leverages HyperChad's unique architecture to deliver both desktop and web experiences from a single Rust codebase. HyperChad handles the complexity of server-client communication via SSE, rendering backend selection (HTML/Egui/FLTK), and state synchronization, allowing ChadReview to focus purely on the GitHub PR review domain logic.

The MVP focuses on viewing a single PR with full metadata, unified diff viewer with server-side syntax highlighting, inline comment threading, and complete comment interaction capabilities (create, reply, edit, delete). Authentication uses GitHub Personal Access Tokens initially, with OAuth planned for future releases.

```
Current GitHub Review Workflow:
User → GitHub Web UI → GitHub API
         ↓
    Limited real-time updates
    Poor performance on large PRs
    Cluttered interface

Proposed ChadReview Architecture:
User → ChadReview (Desktop/Web) → ChadReview Backend → GitHub API
         ↓                              ↓
    HyperChad SSE (real-time)      Server-side rendering
    Efficient HTML rendering       Syntax highlighting
    Clean, focused UI              Comment management
```

## Design Goals

### Primary Objectives

- **Real-time Synchronization**: ALL comments (line-level, file-level, general) auto-update instantly via HyperChad SSE, eliminating the need for manual page refreshes
- **Performance**: Handle PRs of any realistic size efficiently through vanilla HTML rendering and server-side syntax highlighting, with no special virtualization needed
- **User Experience**: Provide a clean, uncluttered interface that focuses on code and discussions, removing noise and distractions present in GitHub's UI
- **Simplicity**: Leverage HyperChad's built-in infrastructure to minimize custom code and dependencies, making the codebase maintainable and performant
- **Reliability**: Robust error handling for GitHub API failures, network issues, and authentication problems, with graceful degradation where possible

### Secondary Objectives

- **Multi-platform Support**: Desktop and web deployment from single codebase via HyperChad backend selection
- **Extensibility**: Clean architecture that allows future additions like CI/CD status, review workflows, and PR list views
- **Provider Abstraction**: Trait-based design supporting multiple git hosting platforms (GitLab, Bitbucket, Gitea) beyond GitHub
- **OAuth Authentication**: Seamless GitHub OAuth flow (post-MVP)
- **Advanced Diff Views**: Side-by-side diffs, diff algorithms (post-MVP)

## Component Architecture

### Core Abstractions

The application is built on domain-specific abstractions that enable provider-agnostic PR review functionality.

**Package: `chadreview_git_provider`**

```rust
// Git hosting provider abstraction - supports GitHub, GitLab, Bitbucket, etc.
#[async_trait::async_trait]
pub trait GitProvider: Send + Sync {
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest>;
    async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>>;
    async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>>;
    async fn create_comment(&self, owner: &str, repo: &str, number: u64, comment: CreateComment) -> Result<Comment>;
    async fn update_comment(&self, comment_id: u64, body: String) -> Result<Comment>;
    async fn delete_comment(&self, comment_id: u64) -> Result<()>;

    // Provider metadata
    fn provider_name(&self) -> &str;
    fn supports_drafts(&self) -> bool;
    fn supports_line_comments(&self) -> bool;
}
```

**Package: `chadreview_github`**

```rust
// GitHub-specific implementation (MVP)
pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
    rate_limit_tracker: RateLimitTracker,
}

impl GitProvider for GitHubProvider {
    // Implementation details in GitHub API Client section
}
```

**Future Provider Implementations** (Post-MVP):

- `chadreview_gitlab` - GitLab provider
- `chadreview_bitbucket` - Bitbucket provider
- `chadreview_gitea` - Gitea provider

**Package: `chadreview_pr_models`**

Provider-agnostic domain models for pull requests, diffs, and comments.

```rust
// src/pr.rs
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
    pub provider: String,  // "github", "gitlab", etc.
}

pub enum PrState {
    Open,
    Closed,
    Merged,
}

// src/diff.rs
pub struct DiffFile {
    pub filename: String,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
}

pub struct DiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<DiffLine>,
}

pub struct DiffLine {
    pub line_type: LineType,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
    pub content: String,
    pub highlighted_html: String,
}

pub enum LineType {
    Addition,
    Deletion,
    Context,
}

pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

// src/comment.rs
pub struct Comment {
    pub id: u64,
    pub author: User,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_type: CommentType,
    pub replies: Vec<Comment>,
}

pub enum CommentType {
    General,
    FileLevelComment { path: String },
    LineLevelComment { path: String, line: usize },
}

// src/user.rs
pub struct User {
    pub id: String,
    pub username: String,
    pub avatar_url: String,
    pub html_url: String,
}

pub struct Label {
    pub name: String,
    pub color: String,
}

pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: User,
    pub committed_at: DateTime<Utc>,
}
```

### Implementation Hierarchy

Following the MoosicBox HyperChad pattern, the codebase is organized into domain-specific packages with nested model crates for clean separation of concerns and optimal compilation boundaries.

```
chadreview/
├── Cargo.toml                      # Workspace root
├── spec/                           # Specifications
│   ├── PREAMBLE.md
│   ├── architecture.md
│   └── plan.md
├── packages/
│   ├── git_provider/               # Provider abstraction package
│   │   ├── models/                 # Models crate (crate: chadreview_git_provider_models)
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       └── lib.rs          # Shared provider models (if any)
│   │   │
│   │   ├── Cargo.toml              # Main crate (crate: chadreview_git_provider)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── provider.rs         # GitProvider trait definition
│   │
│   ├── github/                     # GitHub implementation package
│   │   ├── models/                 # Models crate (crate: chadreview_github_models)
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       └── lib.rs          # GitHub API response models
│   │   │
│   │   ├── Cargo.toml              # Main crate (crate: chadreview_github)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs           # GitHub API client
│   │       └── provider.rs         # GitProvider trait impl for GitHub
│   │
│   ├── pr/                         # PR domain package
│   │   ├── models/                 # Models crate (crate: chadreview_pr_models)
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── pr.rs           # PullRequest, PrState
│   │   │       ├── diff.rs         # DiffFile, DiffHunk, DiffLine
│   │   │       ├── comment.rs      # Comment, CommentType
│   │   │       └── user.rs         # User, Label, etc.
│   │   │
│   │   ├── Cargo.toml              # Main crate (crate: chadreview_pr)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── diff.rs             # Diff parsing/handling logic
│   │
│   ├── syntax/                     # Syntax highlighting (crate: chadreview_syntax)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs              # Server-side syntax highlighting
│   │
│   ├── state/                      # State management (crate: chadreview_state)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   └── app/                        # Main application package
│       ├── models/                 # App models crate (crate: chadreview_app_models)
│       │   ├── Cargo.toml
│       │   └── src/
│       │       └── lib.rs          # App-specific models (if any)
│       │
│       ├── ui/                     # UI components (crate: chadreview_app_ui)
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs
│       │       ├── pr_header.rs
│       │       ├── diff_viewer.rs
│       │       ├── comment_thread.rs
│       │       └── general_comments.rs
│       │
│       ├── Cargo.toml              # Main app (crate: chadreview_app)
│       ├── src/
│       │   ├── main.rs             # Application entry point
│       │   ├── lib.rs
│       │   ├── routes.rs           # Route handlers
│       │   ├── actions.rs
│       │   └── events.rs
│       │
│       └── assets/                 # CSS, vanilla JS
```

### Package Organization Principles

**Domain-Specific Naming**: Each package is named for its domain responsibility (no generic "core" packages)

**Models Separation**: Models are extracted into nested `/models` subdirectories with their own Cargo.toml, providing:

- Clean compilation boundaries
- Reduced rebuild times when only models change
- Ability to share models without pulling in implementation dependencies

**UI as Separate Crate**: UI components live in `app/ui/` as a distinct crate, allowing:

- UI development independent of application logic
- Reuse across different applications
- Testing UI components in isolation

**Provider Abstraction**: The `git_provider` package defines traits that multiple provider implementations (GitHub, GitLab, Bitbucket) will implement

**Workspace Dependencies Convention**: All workspace dependencies MUST use `default-features = false`:

- Forces explicit opt-in to required features in each crate
- Prevents feature creep and unnecessary dependencies
- Ensures minimal dependency trees
- Follows MoosicBox conventions
- Example:
  ```toml
  [workspace.dependencies]
  chadreview_pr_models = { path = "packages/pr/models", version = "0.1.0", default-features = false }
  tokio = { version = "1", default-features = false }
  ```
- Individual crates then enable only needed features:
  ```toml
  [dependencies]
  tokio = { workspace = true, features = ["full"] }
  ```

### Crate Dependency Graph

```
chadreview_pr_models
    ↑
    ├─ chadreview_pr
    ├─ chadreview_git_provider
    ├─ chadreview_git_provider_models
    ├─ chadreview_github_models
    └─ chadreview_app_models

chadreview_git_provider + chadreview_pr_models
    ↑
    └─ chadreview_github

chadreview_github_models + chadreview_git_provider
    ↑
    └─ chadreview_github

chadreview_pr_models + chadreview_app_models
    ↑
    └─ chadreview_app_ui

chadreview_pr_models
    ↑
    └─ chadreview_state

All packages
    ↑
    └─ chadreview_app
```

### Feature Configuration

```toml
# github/Cargo.toml
[features]
default = []
fail-on-warnings = []

# app/Cargo.toml
[features]
default = ["html", "vanilla-js", "github"]

# Git provider implementations
github = ["chadreview_github"]
gitlab = []     # Post-MVP
bitbucket = []  # Post-MVP

# HyperChad rendering backends
html = ["hyperchad/renderer-html"]
vanilla-js = ["html", "hyperchad/renderer-vanilla-js"]
egui-wgpu = ["hyperchad/renderer-egui-wgpu"]
egui-glow = ["hyperchad/renderer-egui-glow"]
fltk = ["hyperchad/renderer-fltk"]

# Deployment options
actix = ["hyperchad/renderer-html-actix"]
lambda = ["hyperchad/renderer-html-lambda"]

# Development
dev = ["assets", "static-routes"]
assets = ["hyperchad/renderer-assets"]
static-routes = ["hyperchad/router-static-routes"]
fail-on-warnings = []
```

## Implementation Details

### GitHub API Client

**Package**: `chadreview_github` (depends on: `chadreview_github_models`, `chadreview_git_provider`, `chadreview_pr_models`)

**Purpose**: Handle all communication with GitHub's REST API, including authentication, rate limiting, and error handling

**Design**:

- Async trait-based design for testability
- PAT-based authentication (OAuth in future)
- Automatic rate limit handling and retry logic
- Response caching with invalidation on real-time updates
- Transforms GitHub API responses into provider-agnostic PR models

**Implementation** (`github/src/client.rs`):

```rust
pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
    rate_limit_tracker: RateLimitTracker,
}

impl GitHubProvider {
    pub fn new(auth_token: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            auth_token,
            base_url: "https://api.github.com".to_string(),
            rate_limit_tracker: RateLimitTracker::new(),
        }
    }
}
```

**Provider Implementation** (`github/src/provider.rs`):

```rust
use chadreview_git_provider::GitProvider;
use chadreview_pr_models::{PullRequest, DiffFile, Comment};
use chadreview_github_models::GithubPrResponse;

#[async_trait::async_trait]
impl GitProvider for GitHubProvider {
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
        self.rate_limit_tracker.check_and_wait().await?;

        let url = format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);
        let response = self.http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .send()
            .await?;

        self.rate_limit_tracker.update_from_headers(&response);

        // Parse GitHub-specific response model
        let pr_data: GithubPrResponse = response.json().await?;

        // Transform to provider-agnostic model
        Ok(pr_data.into())
    }

    fn provider_name(&self) -> &str {
        "github"
    }

    fn supports_drafts(&self) -> bool {
        true
    }

    fn supports_line_comments(&self) -> bool {
        true
    }
}
```

**GitHub Models** (`github/models/src/lib.rs`):

```rust
use serde::{Deserialize, Serialize};
use chadreview_pr_models::PullRequest;

// GitHub API response structure
#[derive(Debug, Deserialize, Serialize)]
pub struct GithubPrResponse {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub draft: bool,
    // ... GitHub-specific fields
}

// Transform GitHub response to provider-agnostic model
impl From<GithubPrResponse> for PullRequest {
    fn from(github_pr: GithubPrResponse) -> Self {
        // Transformation logic
        PullRequest {
            number: github_pr.number,
            title: github_pr.title,
            // ...
        }
    }
}
```

### Syntax Highlighting

**Package**: `chadreview_syntax` (no internal dependencies)

**Purpose**: Provide server-side syntax highlighting for diff content to avoid client-side performance overhead

**Design**:

- Use `syntect` crate for syntax highlighting
- Language detection from file extension
- Pre-highlighted HTML sent to client
- Fallback to plain text for unsupported languages

**Implementation** (`syntax/src/lib.rs`):

```rust
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_line(&self, line: &str, language: &str) -> String {
        let syntax = self.syntax_set
            .find_syntax_by_extension(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        syntect::html::highlighted_html_for_string(
            line,
            &self.syntax_set,
            syntax,
            theme,
        ).unwrap_or_else(|_| html_escape::encode_text(line).to_string())
    }

    pub fn highlight_diff(&self, diff_file: &mut DiffFile) {
        let language = Self::detect_language(&diff_file.filename);

        for hunk in &mut diff_file.hunks {
            for line in &mut hunk.lines {
                line.highlighted_html = self.highlight_line(&line.content, &language);
            }
        }
    }

    fn detect_language(filename: &str) -> String {
        filename
            .split('.')
            .last()
            .unwrap_or("txt")
            .to_string()
    }
}
```

### HyperChad Integration

**Package**: `chadreview_app` (depends on: all packages)

**Purpose**: Leverage HyperChad's SSE, routing, and multi-backend rendering

**Architecture**:

- HyperChad handles all server-client communication
- Components written once, rendered on all backends
- State updates trigger automatic re-renders via SSE
- No manual WebSocket or polling code needed
- Routes import and compose UI components from `chadreview_app_ui`

**Application Entry** (`app/src/main.rs`):

```rust
use hyperchad::app::AppBuilder;
use chadreview_app::{routes, init_app_state};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_state = init_app_state()?;

    AppBuilder::new()
        .with_routes(routes())
        .with_state(app_state)
        .build()?
        .run()
}
```

**Route Handlers** (`app/src/routes.rs`):

```rust
use hyperchad::router::{Router, get, post, put, delete};
use chadreview_app_ui::{pr_header, diff_viewer, comment_thread};
use chadreview_git_provider::GitProvider;
use chadreview_state::AppState;

pub fn routes() -> Router {
    Router::new()
        .route("/pr/:owner/:repo/:number", get(pr_view))
        .route("/api/pr/:owner/:repo/:number/comment", post(create_comment))
        .route("/api/comment/:id", put(update_comment))
        .route("/api/comment/:id", delete(delete_comment))
}

async fn pr_view(
    Path((owner, repo, number)): Path<(String, String, u64)>,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    let provider = &app_state.git_provider;

    let pr = provider.get_pr(&owner, &repo, number).await?;
    let diff = provider.get_diff(&owner, &repo, number).await?;
    let comments = provider.get_comments(&owner, &repo, number).await?;

    app_state.subscribe_to_pr(&owner, &repo, number).await;

    // Compose UI from app_ui components
    render_pr_view(pr, diff, comments)
}
```

**UI Components** (`app/ui/src/pr_header.rs`):

```rust
use hyperchad::template::{container, Containers};
use chadreview_pr_models::PullRequest;

pub fn pr_header(pr: &PullRequest) -> impl Containers {
    container()
        .child(/* header HTML/components */)
}
```

### Comment Threading

**Package**: `chadreview_app_ui` (depends on: `chadreview_pr_models`, `chadreview_app_models`)

**Purpose**: Display and manage nested comment threads with real-time updates

**Design**:

- Line-level and file-level comments rendered inline in diff
- General comments in separate section below diff
- Nested replies displayed as threaded conversations
- Create/edit/delete actions via vanilla JS fetch to API endpoints
- Real-time updates via HyperChad SSE

**Implementation** (`app/ui/src/comment_thread.rs`):

```rust
use hyperchad::template::{container, Containers};
use chadreview_pr_models::{Comment, CommentType};

pub fn comment_thread(comments: &[Comment]) -> impl Containers {
    container()
        .children(comments.iter().map(|comment| {
            comment_item(comment, 0)
        }))
}

fn comment_item(comment: &Comment, depth: usize) -> impl Containers {
    container()
        .child(/* comment body, author, timestamp */)
        .children(comment.replies.iter().map(|reply| {
            comment_item(reply, depth + 1)
        }))
}
```

## Testing Framework

### Test Strategy

**Purpose**: Ensure correctness of GitHub API integration, syntax highlighting, and comment management

**Architecture**:

Tests are organized by package with different testing strategies based on the package's responsibilities:

**Package: `chadreview_pr_models`**

- Unit tests for model serialization/deserialization
- No mocking needed - pure data structures

**Package: `chadreview_github`**

- Unit tests for GitHub API response parsing and transformation
- Integration tests using `wiremock` to mock GitHub API
- Test rate limiting and error handling

**Package: `chadreview_syntax`**

- Unit tests for syntax highlighting with various languages
- Performance tests for large files

**Package: `chadreview_app_ui`**

- Component rendering tests
- Snapshot tests for HTML output

**Package: `chadreview_app`**

- Integration tests for full request/response cycles
- Route handler tests
- End-to-end tests with test database

### Example: GitHub Provider Tests

**Location**: `packages/github/src/provider.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use chadreview_pr_models::PrState;

    #[tokio::test]
    async fn test_get_pr() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "number": 123,
                "title": "Test PR",
                "state": "open",
                "draft": false,
                "body": "PR description",
            })))
            .mount(&mock_server)
            .await;

        let mut client = GitHubProvider::new("test-token".to_string());
        client.base_url = mock_server.uri();

        let pr = client.get_pr("owner", "repo", 123).await.unwrap();

        assert_eq!(pr.number, 123);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.state, PrState::Open);
        assert_eq!(pr.provider, "github");
    }

    #[tokio::test]
    async fn test_rate_limit_handling() {
        // Test rate limit tracking and retry logic
    }
}
```

### Example: UI Component Tests

**Location**: `packages/app/ui/src/comment_thread.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chadreview_pr_models::{Comment, CommentType, User};

    #[test]
    fn test_render_comment_thread() {
        let comments = vec![
            Comment {
                id: 1,
                author: User { /* ... */ },
                body: "Parent comment".to_string(),
                comment_type: CommentType::General,
                replies: vec![
                    Comment {
                        id: 2,
                        body: "Reply comment".to_string(),
                        /* ... */
                    }
                ],
                /* ... */
            }
        ];

        let rendered = comment_thread(&comments);
        // Assert rendered output structure
    }
}
```

## Security Considerations

- **Token Storage**: GitHub PAT stored securely (environment variable or keyring integration)
- **Token Transmission**: Always use HTTPS for GitHub API requests
- **XSS Prevention**: All user-generated content (comments, PR descriptions) must be HTML-escaped
- **Rate Limiting**: Respect GitHub API rate limits to avoid account suspension
- **Scope Validation**: Verify PAT has required scopes (`repo` for private repos, `public_repo` for public)

## Resource Management

- **HTTP Connection Pooling**: Reuse connections to GitHub API via `reqwest` client
- **Memory**: LRU cache for PR data to avoid repeated API calls for recently viewed PRs
- **GitHub API Rate Limits**: 5,000 requests/hour for authenticated users, track and respect limits
- **SSE Connections**: One SSE connection per client viewing a PR, cleaned up on disconnect

## Integration Strategy

ChadReview is a standalone application that integrates with:

1. **GitHub API**: All PR data fetched via REST API v3
2. **HyperChad Framework**: Core rendering and real-time update infrastructure from the MoosicBox project (via git dependency)
3. **MoosicBox Conventions**: Follow workspace dependency management and coding standards

### Package Design Principles

Following the MoosicBox HyperChad application pattern:

1. **Domain-Specific Packages**: No generic "core" or "common" packages - each package has a clear domain purpose
2. **Nested Models Crates**: Models live in `/models` subdirectories with separate Cargo.toml files
3. **UI as Separate Crate**: UI components in `app/ui/` for independent development and testing
4. **Provider Abstraction**: Traits in dedicated packages separate from implementations
5. **Clean Dependencies**: Models packages have minimal dependencies; implementations depend on models

### Migration Path

**Phase 1**: MVP with single PR view, unified diff, inline comments

- Packages: `pr`, `git_provider`, `github`, `syntax`, `state`, `app`, `app/models`, `app/ui`
- Features: GitHub provider, HTML/Vanilla-JS rendering, inline comments

**Phase 2**: Add PR list view, CI/CD checks, review submission

- No new packages needed
- Enhanced UI components and routes

**Phase 3**: OAuth authentication, side-by-side diffs, advanced filtering

- Potential new packages: `auth` for OAuth flow
- Additional provider implementations: `gitlab`, `bitbucket`

## Configuration and Environment

```bash
# Required
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx           # GitHub Personal Access Token

# Optional
CHADREVIEW_PORT=3000                     # Server port (default: 3000)
CHADREVIEW_HOST=0.0.0.0                  # Server host (default: 0.0.0.0)
CHADREVIEW_CACHE_SIZE=100                # Number of PRs to cache (default: 100)
CHADREVIEW_GITHUB_API_URL=https://api.github.com  # Custom GitHub API URL
RUST_LOG=info                            # Logging level
```

## Success Criteria

**Functional Requirements**:

- [ ] View any public/private PR (with appropriate token permissions)
- [ ] Display complete PR metadata (title, description, author, labels, etc.)
- [ ] Render unified diff with server-side syntax highlighting
- [ ] Show all comments inline (line-level, file-level) and general comments separately
- [ ] Real-time comment updates via HyperChad SSE
- [ ] Create new comments on specific lines or files
- [ ] Reply to existing comment threads
- [ ] Edit and delete own comments
- [ ] Handle large PRs (100+ files, 10k+ lines) efficiently

**Technical Requirements**:

- [ ] Zero clippy warnings with fail-on-warnings
- [ ] All tests pass (unit and integration)
- [ ] Documentation complete with usage examples
- [ ] Works on desktop (Egui) and web (HTML+VanillaJS) backends

**Quality Requirements**:

- [ ] Test coverage > 80% for GitHub client and comment management
- [ ] Sub-second page load for typical PRs (<50 files)
- [ ] No XSS vulnerabilities in comment rendering
- [ ] Graceful error handling for API failures

**User Experience Requirements**:

- [ ] Clean, uncluttered interface focused on code and discussions
- [ ] No manual refresh needed - all comments auto-update
- [ ] Significantly better performance than GitHub UI on large PRs
