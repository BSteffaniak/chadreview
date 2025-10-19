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

// GitHub-specific implementation (MVP)
pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
    rate_limit_tracker: RateLimitTracker,
}

// Future implementations
// pub struct GitLabProvider { ... }
// pub struct BitbucketProvider { ... }
// pub struct GiteaProvider { ... }

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
```

### Implementation Hierarchy

```
chadreview/
├── Cargo.toml                      # Workspace root
├── spec/                           # Specifications
│   ├── PREAMBLE.md
│   ├── architecture.md
│   └── plan.md
├── packages/
│   ├── core/                       # Core domain logic (crate: chadreview_core)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs           # PullRequest, DiffFile, Comment types
│   │   │   ├── provider.rs         # GitProvider trait
│   │   │   ├── github.rs           # GitHubProvider implementation
│   │   │   └── syntax.rs           # Server-side syntax highlighting
│   │   └── tests/
│   ├── app/                        # HyperChad application (crate: chadreview_app)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs             # Application entry point
│   │   │   ├── routes.rs           # Route handlers
│   │   │   ├── components/         # HyperChad UI components
│   │   │   │   ├── pr_header.rs
│   │   │   │   ├── diff_viewer.rs
│   │   │   │   ├── comment_thread.rs
│   │   │   │   └── general_comments.rs
│   │   │   └── state.rs            # Application state management
│   │   └── assets/                 # CSS, vanilla JS
│   └── cli/                        # CLI wrapper (crate: chadreview_cli)
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
```

### Feature Configuration

```toml
# core/Cargo.toml
[features]
default = ["github"]

# Git provider implementations
github = []
gitlab = []     # Post-MVP
bitbucket = []  # Post-MVP

fail-on-warnings = []

# app/Cargo.toml
[features]
default = ["html", "vanilla-js"]

# HyperChad backends
html = ["hyperchad_renderer_html"]
vanilla-js = ["hyperchad_renderer_vanilla_js"]
egui-wgpu = ["hyperchad/egui-wgpu"]
egui-glow = ["hyperchad/egui-glow"]
fltk = ["hyperchad/fltk"]

# Deployment options
actix = ["hyperchad/actix", "hyperchad_renderer_html_actix"]
lambda = ["hyperchad/lambda"]

# Development
dev = []
fail-on-warnings = []
```

## Implementation Details

### GitHub API Client

**Purpose**: Handle all communication with GitHub's REST API, including authentication, rate limiting, and error handling

**Design**:

- Async trait-based design for testability
- PAT-based authentication (OAuth in future)
- Automatic rate limit handling and retry logic
- Response caching with invalidation on real-time updates

```rust
pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
    rate_limit_tracker: RateLimitTracker,
}

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

        let pr_data: GithubPrResponse = response.json().await?;
        Ok(pr_data.into())
    }
}
```

### Syntax Highlighting

**Purpose**: Provide server-side syntax highlighting for diff content to avoid client-side performance overhead

**Design**:

- Use `syntect` crate for syntax highlighting
- Language detection from file extension
- Pre-highlighted HTML sent to client
- Fallback to plain text for unsupported languages

```rust
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
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
}
```

### HyperChad Integration

**Purpose**: Leverage HyperChad's SSE, routing, and multi-backend rendering

**Architecture**:

- HyperChad handles all server-client communication
- Components written once, rendered on all backends
- State updates trigger automatic re-renders via SSE
- No manual WebSocket or polling code needed

Route structure:

```rust
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
    let pr = app_state.github_client.get_pr(&owner, &repo, number).await?;
    let diff = app_state.github_client.get_diff(&owner, &repo, number).await?;
    let comments = app_state.github_client.get_comments(&owner, &repo, number).await?;

    app_state.subscribe_to_pr(&owner, &repo, number).await;

    render_pr_view(pr, diff, comments)
}
```

### Comment Threading

**Purpose**: Display and manage nested comment threads with real-time updates

**Design**:

- Line-level and file-level comments rendered inline in diff
- General comments in separate section below diff
- Nested replies displayed as threaded conversations
- Create/edit/delete actions via vanilla JS fetch to API endpoints

## Testing Framework

### Test Strategy

**Purpose**: Ensure correctness of GitHub API integration, syntax highlighting, and comment management

**Architecture**:

- **Unit tests**: GitHub client response parsing, syntax highlighting logic, comment tree building
- **Integration tests**: Full PR fetching and rendering, comment CRUD operations
- **Mock GitHub API**: Use `wiremock` to simulate GitHub responses for deterministic testing
- **HyperChad component tests**: Verify correct HTML generation and SSE updates

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_get_pr() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "number": 123,
                "title": "Test PR",
                "state": "open",
            })))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new(mock_server.uri(), "test-token");
        let pr = client.get_pr("owner", "repo", 123).await.unwrap();

        assert_eq!(pr.number, 123);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.state, PrState::Open);
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

### Migration Path

**Phase 1**: MVP with single PR view, unified diff, inline comments
**Phase 2**: Add PR list view, CI/CD checks, review submission
**Phase 3**: OAuth authentication, side-by-side diffs, advanced filtering

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
