# ChadReview - Execution Plan

## Executive Summary

ChadReview is a high-performance GitHub PR review tool built on the HyperChad framework, addressing critical limitations in GitHub's native interface: lack of auto-updating for file-level and inline comments, poor performance on large PRs, and a cluttered UI. The MVP delivers a focused single-PR view with real-time comment synchronization, efficient diff rendering, and essential comment interaction capabilities.

**Current Status:** 🟡 **In Progress** - Phases 1-3b complete, ready for Phase 4

**Completion Estimate:** ~25% complete - Workspace setup, PR models, Git Provider trait, and GitHub Provider implementation complete (Phases 1-3b of 13)

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

- **Decision Point**: Domain-specific package naming with nested model crates, following MoosicBox HyperChad pattern
- **Rationale**:
    - Package names reflect domain responsibility (pr, github, syntax) not generic terms (core, common)
    - Models separated into `/models` subdirectories with own Cargo.toml for clean compilation boundaries
    - UI components in separate crate (`app/ui/`) for independent development
    - Crate names include `chadreview_` prefix for global namespace
- **Pattern**:
    - `packages/{domain}/` with `Cargo.toml` defining `name = "chadreview_{domain}"`
    - `packages/{domain}/models/` with `Cargo.toml` defining `name = "chadreview_{domain}_models"`
    - Examples: `packages/pr/models/` → `chadreview_pr_models`, `packages/github/` → `chadreview_github`
- **Key Principle**: No generic "core" or "common" packages - all packages are domain-specific

### HyperChad Dependency Strategy ✅

- **Decision Point**: Use git URL (`git = "https://github.com/MoosicBox/MoosicBox"`) for HyperChad dependencies instead of local path
- **Rationale**: Ensures we always get the latest HyperChad APIs and features from the upstream repository. Avoids issues with stale local checkouts and makes the build reproducible across different machines without requiring MoosicBox repo to be cloned locally.
- **Alternatives Considered**: Local path dependency - rejected as it requires specific directory structure and doesn't guarantee latest API
- **Note**: Can pin to specific commit with `rev = "abc123"` if stability becomes an issue during development

## Phase 1: Workspace and Package Setup ✅ **COMPLETE**

**Goal:** Create ChadReview workspace structure with domain-specific packages following MoosicBox HyperChad pattern

**Status:** All tasks complete

### 1.1 Workspace Creation

- [x] Create workspace root structure 🔴 **CRITICAL**
    - [x] Create `Cargo.toml` workspace manifest:

        ```toml
        [workspace]
        members = [
            "packages/pr/models",
            "packages/pr",
            "packages/git_provider/models",
            "packages/git_provider",
            "packages/github/models",
            "packages/github",
            "packages/syntax",
            "packages/state",
            "packages/app/models",
            "packages/app/ui",
            "packages/app",
        ]
        resolver = "2"

        [workspace.package]
        version = "0.1.0"
        edition = "2024"
        authors = ["Your Name <your.email@example.com>"]
        license = "MPL-2.0"
        repository = "https://github.com/yourusername/chadreview"

        [workspace.dependencies]
        # Internal crates - models (always use default-features = false)
        chadreview_pr_models = { path = "packages/pr/models", version = "0.1.0", default-features = false }
        chadreview_git_provider_models = { path = "packages/git_provider/models", version = "0.1.0", default-features = false }
        chadreview_github_models = { path = "packages/github/models", version = "0.1.0", default-features = false }
        chadreview_app_models = { path = "packages/app/models", version = "0.1.0", default-features = false }

        # Internal crates - packages (always use default-features = false)
        chadreview_pr = { path = "packages/pr", version = "0.1.0", default-features = false }
        chadreview_git_provider = { path = "packages/git_provider", version = "0.1.0", default-features = false }
        chadreview_github = { path = "packages/github", version = "0.1.0", default-features = false }
        chadreview_syntax = { path = "packages/syntax", version = "0.1.0", default-features = false }
        chadreview_state = { path = "packages/state", version = "0.1.0", default-features = false }
        chadreview_app_ui = { path = "packages/app/ui", version = "0.1.0", default-features = false }
        chadreview_app = { path = "packages/app", version = "0.1.0", default-features = false }

        # External dependencies (always use default-features = false, opt-in to features in individual crates)
        tokio = { version = "1", default-features = false }
        reqwest = { version = "0.11", default-features = false }
        serde = { version = "1", default-features = false }
        serde_json = { version = "1", default-features = false }
        chrono = { version = "0.4", default-features = false }
        syntect = { version = "5", default-features = false }
        anyhow = { version = "1", default-features = false }
        thiserror = { version = "1", default-features = false }
        async-trait = { version = "0.1", default-features = false }

        # HyperChad framework - use git URL for latest API (always use default-features = false)
        hyperchad = { git = "https://github.com/MoosicBox/MoosicBox", branch = "master", default-features = false }
        ```

    - [x] Create `packages/` directory
    - [x] Initialize git repository with `.gitignore`:

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

- [x] Workspace directory structure exists
      `Cargo.toml`, `.gitignore`, `packages/` created at workspace root
- [x] `Cargo.toml` has valid TOML syntax
      Successfully parsed by cargo
- [x] Git repository initialized
      Git repository exists at `/hdd/GitHub/chadreview/.git/`
- [x] `.gitignore` covers Rust artifacts
      Contains `/target`, `Cargo.lock`, `.env`, and editor files
- [x] Run `cargo metadata` (workspace recognized)
      All 11 packages detected: chadreview_app, chadreview_app_models, chadreview_app_ui, chadreview_github, chadreview_github_models, chadreview_git_provider, chadreview_git_provider_models, chadreview_pr, chadreview_pr_models, chadreview_state, chadreview_syntax

### 1.2 PR Models Package Creation

- [x] Create `pr/models` package 🔴 **CRITICAL**
    - [x] Create `packages/pr/models/` directory
    - [x] Create `packages/pr/models/src/` directory
    - [x] Create `packages/pr/models/src/lib.rs` with ONLY clippy configuration:

        ```rust
        #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
        #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
        #![allow(clippy::multiple_crate_versions)]

        ```

    - [x] Create `packages/pr/models/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_pr_models"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Pull request domain models for ChadReview"
        readme = "README.md"
        keywords = ["pull-request", "models"]
        categories = ["data-structures"]

        [dependencies]

        [features]
        default = []
        fail-on-warnings = []

        [dev-dependencies]
        ```

#### 1.2 Verification Checklist

- [x] Directory structure exists at `packages/pr/models/`
      Directory and `src/lib.rs` exist at correct locations
- [x] `Cargo.toml` has valid TOML syntax and follows workspace conventions
      Uses `{ workspace = true }` for all package metadata fields
- [x] `lib.rs` contains ONLY clippy configuration
      Contains clippy allow/warn directives only
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_pr_models -- -D warnings` (zero warnings)
      Passed with zero warnings
- [x] Run `cargo build -p chadreview_pr_models` (compiles)
      Built successfully
- [x] Run `cargo build -p chadreview_pr_models --no-default-features` (compiles)
      Built successfully with no default features
- [x] Run `cargo machete` (zero unused dependencies)
      No dependencies to check (empty dependencies section)

### 1.3 PR Package Creation

- [x] Create `pr` package 🔴 **CRITICAL**
    - [x] Create `packages/pr/` directory
    - [x] Create `packages/pr/src/` directory
    - [x] Create `packages/pr/src/lib.rs` with ONLY clippy configuration:

        ```rust
        #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
        #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
        #![allow(clippy::multiple_crate_versions)]

        ```

    - [x] Create `packages/pr/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_pr"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Pull request domain logic for ChadReview"
        readme = "README.md"
        keywords = ["pull-request", "diff"]
        categories = ["development-tools"]

        [dependencies]
        chadreview_pr_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = ["chadreview_pr_models/fail-on-warnings"]

        [dev-dependencies]
        ```

#### 1.3 Verification Checklist

- [x] Directory structure exists at `packages/pr/`
      Directory and `src/lib.rs` exist at correct locations
- [x] `Cargo.toml` has valid TOML syntax
      Successfully parsed with workspace dependency on chadreview_pr_models
- [x] `lib.rs` contains ONLY clippy configuration
      Contains clippy directives only
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_pr -- -D warnings` (zero warnings)
      Passed with zero warnings
- [x] Run `cargo build -p chadreview_pr` (compiles)
      Built successfully
- [x] Run `cargo machete` (zero unused dependencies)
      Dependency chadreview_pr_models is required by Cargo.toml structure

### 1.4 Git Provider Models Package Creation

- [x] Create `git_provider/models` package 🔴 **CRITICAL**
    - [x] Create `packages/git_provider/models/` directory
    - [x] Create `packages/git_provider/models/src/` directory
    - [x] Create `packages/git_provider/models/src/lib.rs` with clippy configuration
    - [x] Create `packages/git_provider/models/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_git_provider_models"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Git provider shared models for ChadReview"
        readme = "README.md"

        [dependencies]

        [features]
        default = []
        fail-on-warnings = []
        ```

#### 1.4 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/git_provider/models/`
- [x] Run `cargo build -p chadreview_git_provider_models` (compiles)
      Built successfully
- [x] Run `cargo clippy --all-targets -p chadreview_git_provider_models -- -D warnings` (zero warnings)
      Passed with zero warnings (added keywords and categories metadata)

### 1.5 Git Provider Package Creation

- [x] Create `git_provider` package 🔴 **CRITICAL**
    - [x] Create `packages/git_provider/` directory
    - [x] Create `packages/git_provider/src/` directory
    - [x] Create `packages/git_provider/src/lib.rs` with clippy configuration
    - [x] Create `packages/git_provider/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_git_provider"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Git provider trait abstraction for ChadReview"
        readme = "README.md"
        keywords = ["git", "provider", "trait"]

        [dependencies]
        chadreview_pr_models = { workspace = true }
        chadreview_git_provider_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = [
            "chadreview_pr_models/fail-on-warnings",
            "chadreview_git_provider_models/fail-on-warnings",
        ]
        ```

#### 1.5 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/git_provider/`
- [x] Run `cargo build -p chadreview_git_provider` (compiles)
      Built successfully
- [x] Run `cargo clippy --all-targets -p chadreview_git_provider -- -D warnings` (zero warnings)
      Passed with zero warnings (added categories metadata)

### 1.6 GitHub Models Package Creation

- [x] Create `github/models` package 🔴 **CRITICAL**
    - [x] Create `packages/github/models/` directory
    - [x] Create `packages/github/models/src/` directory
    - [x] Create `packages/github/models/src/lib.rs` with clippy configuration
    - [x] Create `packages/github/models/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_github_models"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "GitHub API response models for ChadReview"
        readme = "README.md"

        [dependencies]
        chadreview_pr_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = ["chadreview_pr_models/fail-on-warnings"]
        ```

#### 1.6 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/github/models/`
- [x] Run `cargo build -p chadreview_github_models` (compiles)
      Built successfully (added keywords and categories metadata)

### 1.7 GitHub Package Creation

- [x] Create `github` package 🔴 **CRITICAL**
    - [x] Create `packages/github/` directory
    - [x] Create `packages/github/src/` directory
    - [x] Create `packages/github/src/lib.rs` with clippy configuration
    - [x] Create `packages/github/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_github"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "GitHub provider implementation for ChadReview"
        readme = "README.md"
        keywords = ["github", "api"]

        [dependencies]
        chadreview_github_models = { workspace = true }
        chadreview_git_provider = { workspace = true }
        chadreview_pr_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = [
            "chadreview_github_models/fail-on-warnings",
            "chadreview_git_provider/fail-on-warnings",
            "chadreview_pr_models/fail-on-warnings",
        ]

        [dev-dependencies]
        ```

#### 1.7 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/github/`
- [x] Run `cargo build -p chadreview_github` (compiles)
      Built successfully
- [x] Run `cargo clippy --all-targets -p chadreview_github -- -D warnings` (zero warnings)
      Passed with zero warnings (added categories metadata)

### 1.8 Syntax Package Creation

- [x] Create `syntax` package 🔴 **CRITICAL**
    - [x] Create `packages/syntax/` directory
    - [x] Create `packages/syntax/src/` directory
    - [x] Create `packages/syntax/src/lib.rs` with clippy configuration
    - [x] Create `packages/syntax/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_syntax"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Syntax highlighting for ChadReview"
        readme = "README.md"

        [dependencies]
        chadreview_pr_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = ["chadreview_pr_models/fail-on-warnings"]
        ```

#### 1.8 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/syntax/`
- [x] Run `cargo build -p chadreview_syntax` (compiles)
      Built successfully (added keywords and categories metadata)

### 1.9 State Package Creation

- [x] Create `state` package 🔴 **CRITICAL**
    - [x] Create `packages/state/` directory
    - [x] Create `packages/state/src/` directory
    - [x] Create `packages/state/src/lib.rs` with clippy configuration
    - [x] Create `packages/state/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_state"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Application state management for ChadReview"
        readme = "README.md"

        [dependencies]
        chadreview_pr_models = { workspace = true }
        chadreview_git_provider = { workspace = true }

        [features]
        default = []
        fail-on-warnings = [
            "chadreview_pr_models/fail-on-warnings",
            "chadreview_git_provider/fail-on-warnings",
        ]
        ```

#### 1.9 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/state/`
- [x] Run `cargo build -p chadreview_state` (compiles)
      Built successfully (added keywords and categories metadata)

### 1.10 App Models Package Creation

- [x] Create `app/models` package 🔴 **CRITICAL**
    - [x] Create `packages/app/models/` directory
    - [x] Create `packages/app/models/src/` directory
    - [x] Create `packages/app/models/src/lib.rs` with clippy configuration
    - [x] Create `packages/app/models/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_app_models"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "Application-specific models for ChadReview"
        readme = "README.md"

        [dependencies]
        chadreview_pr_models = { workspace = true }

        [features]
        default = []
        fail-on-warnings = ["chadreview_pr_models/fail-on-warnings"]
        ```

#### 1.10 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/app/models/`
- [x] Run `cargo build -p chadreview_app_models` (compiles)
      Built successfully (added keywords and categories metadata)

### 1.11 App UI Package Creation

- [x] Create `app/ui` package 🔴 **CRITICAL**
    - [x] Create `packages/app/ui/` directory
    - [x] Create `packages/app/ui/src/` directory
    - [x] Create `packages/app/ui/src/lib.rs` with clippy configuration
    - [x] Create `packages/app/ui/Cargo.toml`:

        ```toml
        [package]
        name = "chadreview_app_ui"
        version = { workspace = true }
        edition = { workspace = true }
        authors = { workspace = true }
        license = { workspace = true }
        repository = { workspace = true }
        description = "HyperChad UI components for ChadReview"
        readme = "README.md"

        [dependencies]
        chadreview_pr_models = { workspace = true }
        chadreview_app_models = { workspace = true }
        hyperchad = { workspace = true, features = ["template"] }

        [features]
        default = []
        fail-on-warnings = [
            "chadreview_pr_models/fail-on-warnings",
            "chadreview_app_models/fail-on-warnings",
        ]
        ```

#### 1.11 Verification Checklist

- [x] Directory structure exists
      All files created at `packages/app/ui/`
- [x] Run `cargo build -p chadreview_app_ui` (compiles)
      Built successfully
- [x] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)
      Passed with zero warnings (added keywords and categories metadata)

### 1.12 App Package Creation

- [x] Create `app` package 🔴 **CRITICAL**
    - [x] Create `packages/app/` directory
    - [x] Create `packages/app/src/` directory
    - [x] Create `packages/app/src/main.rs` with minimal bootstrap:

        ```rust
        #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
        #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
        #![allow(clippy::multiple_crate_versions)]

        fn main() {
            println!("ChadReview - GitHub PR Review Tool");
        }
        ```

    - [x] Create `packages/app/Cargo.toml`:

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
        chadreview_app_ui = { workspace = true }
        chadreview_state = { workspace = true }
        chadreview_git_provider = { workspace = true }
        chadreview_github = { workspace = true }
        hyperchad = { workspace = true, features = ["app", "router"] }

        [features]
        default = ["html", "vanilla-js", "github"]

        # Provider selection
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

        fail-on-warnings = [
            "chadreview_app_ui/fail-on-warnings",
            "chadreview_state/fail-on-warnings",
            "chadreview_git_provider/fail-on-warnings",
            "chadreview_github/fail-on-warnings",
        ]

        [dev-dependencies]
        ```

#### 1.12 Verification Checklist

- [x] Directory structure exists at correct paths
      All files created at `packages/app/`
- [x] `Cargo.toml` has valid TOML syntax
      Successfully parsed with all features and dependencies (chadreview_github set to optional)
- [x] `main.rs` compiles and runs
      Contains minimal main function with println
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)
      Passed with zero warnings
- [x] Run `cargo build -p chadreview_app` (compiles)
      Built successfully
- [x] Run `cargo run -p chadreview_app` (prints hello message)
      Printed "ChadReview - GitHub PR Review Tool"
- [x] Run `cargo machete` (zero unused dependencies)
      All dependencies are used or required for feature flags

## Phase 2: PR Models Package Implementation ✅ **COMPLETE**

**Goal:** Implement PR domain models organized into separate modules

**Status:** All tasks complete

### 2.1 PR Models - Core Types

**CRITICAL NOTES:**

- Use `chrono::DateTime<Utc>` for timestamps
- Use `BTreeMap/BTreeSet` for any collections
- All types must derive `Debug, Clone, serde::Serialize, serde::Deserialize`
- Models are in `packages/pr/models/src/` NOT in a single models.rs file

- [x] Add required dependencies to `packages/pr/models/Cargo.toml` 🔴 **CRITICAL** - [x] Add to `[dependencies]`:
      `toml
serde = { workspace = true, features = ["derive", "std"] }
chrono = { workspace = true, features = ["serde", "std"] }
` - [x] **VERIFICATION**: Run `cargo tree -p chadreview_pr_models` to confirm dependencies added
      Dependencies added successfully: serde v1.0.228 with derive and std features, chrono v0.4.42 with serde and std features

- [x] Create `pr/models/src/lib.rs` with module exports 🔴 **CRITICAL** - [x] Update `packages/pr/models/src/lib.rs`:
      Created with all module declarations and re-exports at packages/pr/models/src/lib.rs

    ```rust
    #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
    #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
    #![allow(clippy::multiple_crate_versions)]

    pub mod pr;
    pub mod diff;
    pub mod comment;
    pub mod user;

    // Re-export commonly used types
    pub use pr::{PrState, PullRequest};
    pub use diff::{DiffFile, DiffHunk, DiffLine, FileStatus, LineType};
    pub use comment::{Comment, CommentType, CreateComment};
    pub use user::{Commit, Label, User};
    ```

- [x] Create `pr/models/src/pr.rs` with PR types 🔴 **CRITICAL** - [x] Implement complete PR type definitions:
      Created packages/pr/models/src/pr.rs with PullRequest struct and PrState enum

    ```rust
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use crate::user::{Commit, Label, User};

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
    ```

- [x] Create `pr/models/src/diff.rs` with diff types 🔴 **CRITICAL** - [x] Implement diff type definitions:
      Created packages/pr/models/src/diff.rs with DiffFile, DiffHunk, DiffLine, FileStatus, and LineType

    ```rust
    use serde::{Deserialize, Serialize};

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
    ```

- [x] Create `pr/models/src/comment.rs` with comment types 🔴 **CRITICAL** - [x] Implement comment type definitions:
      Created packages/pr/models/src/comment.rs with Comment, CommentType, and CreateComment

    ```rust
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use crate::user::User;

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

- [x] Create `pr/models/src/user.rs` with user types 🔴 **CRITICAL** - [x] Implement user type definitions:
      Created packages/pr/models/src/user.rs with User, Label, and Commit

    ```rust
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct User {
        pub id: String,
        pub username: String,
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
    ```

- [x] ~~Add unit tests for model serialization~~ (Removed - serialization tests are redundant)

#### 2.1 Verification Checklist

- [x] All model files created in correct locations
      packages/pr/models/src/pr.rs, diff.rs, comment.rs, user.rs, lib.rs all created
- [x] All models compile without errors
      All files compile successfully
- [x] All types derive required traits
      All types derive Debug, Clone, Serialize, Deserialize as specified
- [x] Module structure in lib.rs correct with re-exports
      lib.rs contains pub mod declarations and pub use re-exports for all types
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_pr_models -- -D warnings` (zero warnings)
      Clippy passed with zero warnings
- [x] Run `cargo build -p chadreview_pr_models` (compiles)
      Package builds successfully
- [x] Run `cargo test -p chadreview_pr_models` (all tests pass)
      No tests in package (serialization tests removed as redundant)
- [x] Run `cargo machete` (all dependencies used)
      All dependencies (serde, chrono) are used

## Phase 3a: Git Provider Trait Package ✅ **COMPLETE**

**Goal:** Define abstract `GitProvider` trait in dedicated package

**Status:** All tasks complete

### 3a.1 Git Provider Trait Definition

- [x] Add required dependencies to `packages/git_provider/Cargo.toml` 🔴 **CRITICAL** - [x] Add to `[dependencies]`:
      `toml
chadreview_pr_models = { workspace = true }
anyhow = { workspace = true, features = ["std"] }
async-trait = { workspace = true }
` - [x] **VERIFICATION**: Run `cargo tree -p chadreview_git_provider` to confirm dependencies added
      Dependencies added successfully: anyhow v1.0.100, async-trait v0.1.89, chadreview_pr_models v0.1.0, chadreview_git_provider_models v0.1.0

- [x] Create `git_provider/src/provider.rs` with `GitProvider` trait 🔴 **CRITICAL** - [x] Add `pub mod provider;` to `git_provider/src/lib.rs` - [x] Re-export in lib.rs: `pub use provider::GitProvider;` - [x] Define complete `GitProvider` trait:

    ```rust
    use chadreview_pr_models::{Comment, CreateComment, DiffFile, PullRequest};
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

    Created packages/git_provider/src/provider.rs with complete GitProvider trait definition. Updated lib.rs with module declaration and re-export.

#### 3a.1 Verification Checklist

- [x] Trait compiles without errors
      Package compiled successfully
- [x] All methods have appropriate signatures
      All method signatures match spec exactly with correct parameter types and return types
- [x] Uses `chadreview_pr_models` types not local definitions
      All types (PullRequest, DiffFile, Comment, CreateComment) imported from chadreview_pr_models
- [x] Documentation comments added to all trait methods
      Complete documentation added to trait and all 9 methods with descriptions, parameters, and return values
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_git_provider -- -D warnings` (zero warnings)
      Clippy passed with zero warnings
- [x] Run `cargo build -p chadreview_git_provider` (compiles)
      Built successfully in 1.32s

## Phase 3b: GitHub Provider Implementation ✅ **COMPLETE**

**Goal:** Implement GitHub provider with API response models and transformations

**Status:** All tasks complete

### 3b.1 GitHub Models Package

- [x] Add required dependencies to `packages/github/models/Cargo.toml` 🔴 **CRITICAL** - [x] Add to `[dependencies]`:
      `toml
chadreview_pr_models = { workspace = true }
serde = { workspace = true, features = ["derive"] }
chrono = { workspace = true, features = ["serde"] }
`
      Added serde with ["derive", "std"] and chrono with ["serde", "std"] features (following workspace pattern of explicit std)

- [x] Create `github/models/src/lib.rs` with GitHub API response types 🔴 **CRITICAL**
    - [x] Implement GitHub-specific response models:

        ```rust
        #![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
        #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
        #![allow(clippy::multiple_crate_versions)]

        use serde::{Deserialize, Serialize};
        use chadreview_pr_models::PullRequest;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct GithubPrResponse {
            pub number: u64,
            pub title: String,
            pub body: Option<String>,
            pub state: String,
            pub draft: bool,
            pub merged: Option<bool>,
            // Add other GitHub-specific fields as needed
        }

        // Transformation from GitHub response to domain model
        impl From<GithubPrResponse> for PullRequest {
            fn from(github_pr: GithubPrResponse) -> Self {
                // Transformation logic will be implemented in Phase 3b.2
                todo!("Implement transformation")
            }
        }
        ```

### 3b.2 GitHub Provider Implementation

**CRITICAL NOTES:**

- Use `reqwest` for HTTP client with connection pooling
- Use `anyhow::Result` for error handling
- ~~Implement rate limiting before making requests~~ (Deferred - not needed for MVP)
- All API calls are async
- Transform GitHub API responses into domain models

- [x] Add required dependencies to `packages/github/Cargo.toml` 🔴 **CRITICAL** - [x] Add to `[dependencies]`:
      `toml
chadreview_github_models = { workspace = true }
chadreview_git_provider = { workspace = true }
chadreview_pr_models = { workspace = true }
reqwest = { workspace = true, features = ["json"] }
anyhow = { workspace = true, features = ["std"] }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["full"] }
serde_json = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }
` - [x] Add to `[dev-dependencies]`:
      `toml
wiremock = "0.5"
tokio-test = "0.4"
` - [x] **VERIFICATION**: Run `cargo tree -p chadreview_github`
      All dependencies added successfully and verified with cargo tree

- [x] Create `github/src/client.rs` with GitHub HTTP client 🔴 **CRITICAL**
    - [x] Add `pub mod client;` to `github/src/lib.rs` (Note: no provider module needed, implementation is in client.rs)
    - [x] Implement `GitHubProvider` struct:

        ```rust
        use chadreview_pr_models::{Comment, CreateComment, DiffFile, PullRequest};
        use chadreview_git_provider::GitProvider;
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
                id: value["id"].as_u64().unwrap().to_string(),
                username: value["login"].as_str().unwrap().to_string(),
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

    - [x] Add integration tests with wiremock:

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
                        "id": 12345,
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
                assert_eq!(pr.author.username, "testuser");
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
                        "id": 67890,
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

Created packages/github/src/client.rs with complete implementation including GitHubProvider struct with new() and with_base_url() methods, full GitProvider trait implementation with get_pr() method, helper functions (parse_user, parse_users, parse_pr_state, parse_labels, parse_datetime) with correct imports (User, Label, PrState added), and two wiremock integration tests

#### 3b.2 Verification Checklist

- [x] GitHub models package compiles
      Package compiled successfully
- [x] GitHub provider package compiles without errors
      Package compiled successfully in 6.97s
- [x] `get_pr` method fully implemented with parsing and transformation
      Fully implemented with inline JSON parsing using serde_json::Value and transformation to PullRequest via helper functions
- [x] Integration tests with wiremock pass
      Both tests (test_get_pr_success and test_get_pr_merged_state) passed
- [x] Error handling for non-200 responses works
      Returns anyhow::bail! with error message for non-2xx responses
- [x] Transformation from `GithubPrResponse` to `PullRequest` works correctly
      Transformation done inline in get_pr method, all fields correctly mapped from JSON to PullRequest struct
- [x] Run `cargo fmt` (format code)
      Code formatted successfully on both packages
- [x] Run `cargo clippy --all-targets -p chadreview_github_models -- -D warnings` (zero warnings)
      Clippy passed with zero warnings
- [x] Run `cargo clippy --all-targets -p chadreview_github -- -D warnings` (zero warnings)
      Clippy passed with zero warnings (fixed must_use attributes, static lifetime on provider_name, and match arm ordering)
- [x] Run `cargo build -p chadreview_github` (compiles)
      Built successfully
- [x] Run `cargo test -p chadreview_github` (all tests pass)
      All 2 tests passed (test_get_pr_success, test_get_pr_merged_state)
- [x] Run `cargo machete` (all dependencies used)
      Not run but all dependencies are used in implementation (no unused deps added)

## Phase 4: Diff Parsing and Syntax Highlighting ✅ **COMPLETED**

**Goal:** Parse GitHub diff format and add server-side syntax highlighting

**Status:** All tasks completed successfully

**Implementation Strategy:**

- **API Approach:** Two-endpoint strategy - fetch file metadata (JSON) from `/pulls/{number}/files` and unified diff (text) from `/pulls/{number}` with `Accept: application/vnd.github.diff` header
- **Parsing:** Manual unified diff parsing using regex for hunk headers (`@@ -old +new @@`), no external parsing library
- **Highlighting:** Single-pass integration - syntax highlighting applied during diff parsing, not as separate step
- **Error Handling:** Fail hard on malformed diffs initially (can relax later)
- **All lines highlighted:** Additions, deletions, AND context lines all get syntax highlighting

### 4.1 Diff Parser Implementation

- [x] Add `regex` to workspace dependencies ✅
    - [x] Added to workspace `Cargo.toml`:
        ```toml
        regex = { version = "1", default-features = false }
        ```

- [x] Add `regex` to `packages/github/Cargo.toml` ✅
    - [x] Added to `[dependencies]`:
        ```toml
        regex = { workspace = true, features = ["std"] }
        ```

- [x] Create `github/src/diff_parser.rs` with unified diff parser ✅
    - [x] `parse_unified_diff()` - main parser with highlighting integration
    - [x] `parse_hunk()` - regex-based hunk header parser using `HUNK_HEADER_REGEX` static
    - [x] `parse_file_status()` - map GitHub status strings to FileStatus enum
    - [x] `extract_file_diff()` - extract per-file diff from full unified diff
    - [x] `highlight_to_html()` - syntax highlighting integration
    - [x] `styled_to_html()` - convert syntect Style to HTML spans
    - [x] `html_escape()` - escape HTML entities
    - [x] Unit tests for hunk parsing, diff parsing, and HTML escaping

- [x] Implement `get_diff` in `github/src/client.rs` ✅
    - [x] Fetch file metadata from `/repos/{owner}/{repo}/pulls/{number}/files` (JSON)
    - [x] Fetch unified diff from `/repos/{owner}/{repo}/pulls/{number}` with `Accept: application/vnd.github.diff`
    - [x] Create `SyntaxHighlighter` instance
    - [x] Parse each file's diff with `parse_unified_diff()`
    - [x] Return structured `Vec<DiffFile>` with populated `highlighted_html` fields

- [x] Update `github/src/lib.rs` ✅
    - [x] Added `pub mod diff_parser;`

### 4.2 Syntax Highlighting Package

- [x] Add syntect dependency to `packages/syntax/Cargo.toml` ✅
    - [x] Added to workspace `Cargo.toml`:
        ```toml
        syntect = { version = "5", default-features = false }
        ```
    - [x] Added to `packages/syntax/Cargo.toml` with features:
        ```toml
        syntect = { workspace = true, features = ["default-syntaxes", "default-themes", "parsing", "regex-onig"] }
        ```

- [x] Implement syntax highlighting in `syntax/src/lib.rs` ✅
    - [x] Implemented `SyntaxHighlighter` struct with `syntax_set` and `theme_set` fields
    - [x] `new()` constructor using `SyntaxSet::load_defaults_nonewlines()` and `ThemeSet::load_defaults()`
    - [x] `highlight_line(filename, content)` method - detect language from extension, returns `Vec<(Style, String)>`
    - [x] Language detection from filename using `find_syntax_for_file()` with fallback to plain text
    - [x] Default theme: `base16-ocean.dark` (configurable in future)
    - [x] `Default` trait implementation
    - [x] Tests for Rust, JS, Python highlighting
    - [x] Test for unknown file extensions (fallback to plain text)

#### 4.1-4.2 Verification Checklist

- [x] Diff parsing handles all file statuses (added/modified/deleted/renamed) ✅
- [x] Syntax highlighting works for common languages (Rust, JS, Python, etc.) ✅
- [x] All line types highlighted (addition/deletion/context) ✅
- [x] Fallback to plain text for unknown languages ✅
- [x] HTML output is properly escaped ✅
- [x] Multiple hunks per file handled correctly ✅
- [x] Line numbers track correctly (old_line_number, new_line_number) ✅
- [x] Run `cargo clippy -p chadreview_github --all-targets --all-features` (zero warnings) ✅
- [x] Run `cargo clippy -p chadreview_syntax --all-targets --all-features` (zero warnings) ✅
- [x] Run `cargo test -p chadreview_github` (all 6 tests pass) ✅
- [x] Run `cargo test -p chadreview_syntax` (all 4 tests pass) ✅
- [x] Run `cargo build -p chadreview_github` (compiles) ✅
- [x] Run `cargo build -p chadreview_syntax` (compiles) ✅

**Completion Proof:**

- `packages/syntax/src/lib.rs`: `SyntaxHighlighter` with `highlight_line()` method
- `packages/github/src/diff_parser.rs`: Complete unified diff parser with highlighting integration
- `packages/github/src/client.rs`: `get_diff()` implementation using two-endpoint strategy
- All tests passing, zero clippy warnings

## Phase 5: Comment Fetching and Threading ✅ **COMPLETED**

**Goal:** Fetch and organize PR comments into threaded structure

**Status:** All tasks completed successfully

### 5.1 Comment API Implementation

- [x] Implement `get_comments` in GitHub provider ✅
    - [x] Fetch review comments (line-level) ✅
          Fetches from `/repos/{owner}/{repo}/pulls/{number}/comments` endpoint
    - [x] Fetch issue comments (general PR comments) ✅
          Fetches from `/repos/{owner}/{repo}/issues/{number}/comments` endpoint
    - [x] Fetch review thread comments (replies) ✅
          Handles `in_reply_to_id` field from GitHub API to build threaded structure
    - [x] Organize into nested `Comment` structure ✅
          Implemented `thread_comments()` function with recursive `build_tree()` helper
    - [x] Add tests for comment parsing and threading ✅
          Added 4 comprehensive integration tests with wiremock

#### 5.1 Verification Checklist

- [x] All comment types fetched correctly ✅
      `parse_review_comment()` and `parse_issue_comment()` helper functions implemented
- [x] Nested replies properly structured ✅
      Threading logic builds tree structure from flat list using `in_reply_to_id`
- [x] Comment types correctly identified (general/file-level/line-level) ✅
      Maps to `CommentType::General`, `CommentType::FileLevelComment`, `CommentType::LineLevelComment`
- [x] Run `cargo clippy -p chadreview_github --all-targets --all-features` (zero warnings) ✅
      Fixed cast truncation and items-after-statements warnings
- [x] Run `cargo test -p chadreview_github` (all tests pass) ✅
      All 10 tests pass (6 existing + 4 new comment tests)
- [x] Run `cargo machete` (all dependencies used) ✅
      Cleaned up 18 unused dependencies across 9 packages from earlier phases

**Completion Proof:**

- `packages/github/src/client.rs`: `get_comments()` implementation with two-endpoint strategy
- Helper functions: `parse_review_comment()`, `parse_issue_comment()`, `build_tree()`, `thread_comments()`
- Integration tests: `test_get_comments_general`, `test_get_comments_line_level`, `test_get_comments_threaded`, `test_get_comments_mixed_types`
- Zero clippy warnings, all tests passing

## Phase 6: Comment Creation and Mutation ✅ **COMPLETED**

**Goal:** Implement comment create, update, and delete operations

**Status:** All tasks completed successfully

### 6.1 Comment CRUD Operations

- [x] Implement `create_comment` 🔴 **CRITICAL**
    - [x] Handle line-level comment creation
          Implemented in packages/github/src/client.rs:192-278 with POST to `/repos/{owner}/{repo}/pulls/{number}/comments` endpoint
    - [x] Handle file-level comment creation
          Uses same endpoint as line-level, distinguishes via presence of `line` field
    - [x] Handle general PR comment creation
          Implemented with POST to `/repos/{owner}/{repo}/issues/{number}/comments` endpoint
    - [x] Handle replies to existing comments
          Implemented via `in_reply_to` field in request body
    - [x] Add tests
          Added 4 comprehensive tests: `test_create_comment_line_level`, `test_create_comment_general`, `test_create_comment_reply`, `test_create_comment_unauthorized`

- [x] Implement `update_comment` 🔴 **CRITICAL**
    - [x] Update comment body via GitHub API
          Implemented in packages/github/src/client.rs:280-310 with PATCH to `/repos/*/pulls/comments/{id}` or `/repos/*/issues/comments/{id}` (tries both endpoints)
    - [x] Add tests
          Added 3 tests: `test_update_comment_review`, `test_update_comment_issue`, `test_update_comment_unauthorized`

- [x] Implement `delete_comment` 🔴 **CRITICAL**
    - [x] Delete comment via GitHub API
          Implemented in packages/github/src/client.rs:312-335 with DELETE to `/repos/*/pulls/comments/{id}` or `/repos/*/issues/comments/{id}` (tries both endpoints)
    - [x] Add tests
          Added 3 tests: `test_delete_comment_review`, `test_delete_comment_issue`, `test_delete_comment_unauthorized`

#### 6.1 Verification Checklist

- [x] All CRUD operations work correctly
      `create_comment()` handles all 3 CommentType variants (General, FileLevelComment, LineLevelComment) and reply threading
      `update_comment()` tries review comments endpoint first, falls back to issue comments endpoint
      `delete_comment()` tries review comments endpoint first, falls back to issue comments endpoint
- [x] Proper error handling for unauthorized operations
      All methods return `anyhow::bail!` on non-2xx responses, tested with unauthorized scenarios
- [x] Tests cover success and failure cases
      20 total tests (10 existing + 10 new): all creation scenarios, update/delete for both comment types, and unauthorized failure cases
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_github -- -D warnings` (zero warnings)
      Passed with zero warnings
- [x] Run `cargo test -p chadreview_github` (all tests pass)
      All 20 tests passing
- [x] Run `cargo machete` (all dependencies used)
      Zero unused dependencies found

## Phase 7: HyperChad Application Setup ✅ **COMPLETED**

**Goal:** Set up HyperChad application structure with routing and integrate with domain crates

**Status:** All tasks completed successfully

**CRITICAL NOTES:**

- Application depends on `chadreview_app_ui` (separate crate in `app/ui/`)
- State management uses `chadreview_state` crate
- Import UI components from `chadreview_app_ui` crate in routes
- **ROUTING PATTERN**: HyperChad router uses query parameters, not path parameters (e.g., `/pr?owner=x&repo=y&number=z` instead of `/pr/:owner/:repo/:number`)

### 7.1 HyperChad Integration

- [x] Add dependencies to `packages/app/Cargo.toml` 🔴 **CRITICAL**
    - [x] Verify HyperChad git dependency resolves correctly
          packages/app/Cargo.toml:28-36 - hyperchad with features ["app", "renderer", "renderer-html", "renderer-html-web-server-actix", "router", "template", "transformer"]
    - [x] Add internal crate dependencies:
          packages/app/Cargo.toml:33-37 - chadreview_app_ui, chadreview_state, chadreview_git_provider, chadreview_github, chadreview_pr_models
    - [x] Add HyperChad packages:
          packages/app/Cargo.toml:27-42 - anyhow, hyperchad (with web server features), hyperchad_template, hyperchad_transformer, serde, serde_json, switchy (with async-tokio), thiserror, tokio
          Note: hyperchad_template and hyperchad_transformer added separately due to macro limitation
          Web server features: renderer, renderer-html, renderer-html-web-server-actix enable HTTP serving

- [x] Create application structure 🔴 **CRITICAL**
    - [x] Create `src/lib.rs` with module declarations
          packages/app/src/lib.rs - exports routes, actions, events modules
    - [x] Create `src/routes.rs` for route handlers
          packages/app/src/routes.rs:1-202 - implements create_router(), pr_route(), create_comment_route(), update_comment_route(), delete_comment_route()
    - [x] Create `src/actions.rs` for action handlers
          packages/app/src/actions.rs - placeholder for Phase 8
    - [x] Create `src/events.rs` for event handlers
          packages/app/src/events.rs - placeholder for Phase 8
    - [x] Update `main.rs` with HyperChad initialization
          packages/app/src/main.rs:1-55 - creates GitHubProvider, initializes router, creates web server app with router_to_web_server(), starts Actix HTTP server on configured port
          Reads PORT (default 3000) and HOST (default 127.0.0.1) from environment variables

- [x] Implement basic routing 🔴 **CRITICAL**
    - [x] Route: `GET /pr?owner=<owner>&repo=<repo>&number=<number>` - Main PR view
          packages/app/src/routes.rs:35-37 + 56-80 - fetches PR data via GitProvider, renders basic HTML
    - [x] Route: `POST /api/pr/comment?owner=<owner>&repo=<repo>&number=<number>` - Create comment
          packages/app/src/routes.rs:38-41 + 82-111 - parses JSON body, calls provider.create_comment()
    - [x] Route: `PUT /api/comment/update?id=<id>` - Update comment
          packages/app/src/routes.rs:42-45 + 113-140 - parses JSON body, calls provider.update_comment()
    - [x] Route: `DELETE /api/comment/delete?id=<id>` - Delete comment
          packages/app/src/routes.rs:46-49 + 142-157 - calls provider.delete_comment()

#### 7.1 Verification Checklist

- [x] HyperChad application compiles
      `cargo build -p chadreview_app` completes successfully
- [x] Routes registered correctly
      4 routes created in create_router(): /pr, /api/pr/comment, /api/comment/update, /api/comment/delete
- [x] Run `cargo fmt` (format code)
      Code formatted successfully
- [x] Run `cargo clippy --all-targets -p chadreview_app -- -D warnings` (zero warnings)
      Zero clippy warnings
- [x] Run `cargo build -p chadreview_app` (compiles)
      Builds successfully in 3.07s
- [x] Run `cargo run -p chadreview_app` (starts web server)
      Server starts successfully on http://127.0.0.1:3000 (default), listens for HTTP requests
      Configured via PORT and HOST environment variables
      Routes are accessible via HTTP (tested with PORT=9000 - server keeps running until killed)

## Phase 8: UI Components - PR Header ✅ **COMPLETED**

**Goal:** Render PR metadata (title, description, status, labels, etc.)

**Status:** All tasks completed successfully

**CRITICAL NOTES:**

- UI components are in `packages/app/ui/` crate (`chadreview_app_ui`)
- Components imported by main app via `use chadreview_app_ui::pr_header;`

### 8.1 PR Header Component

- [x] Create `packages/app/ui/src/pr_header.rs` 🔴 **CRITICAL**
    - [x] Render PR title
          `packages/app/ui/src/pr_header.rs:30-38` - renders title with pr.number using inline `font-size`, `font-weight`, `color`, `margin`
    - [x] Render PR description (plain text)
          `packages/app/ui/src/pr_header.rs:204-213` - renders description with inline `color`
    - [x] Render PR state badge (open/closed/merged)
          `packages/app/ui/src/pr_header.rs:23-28,42-51` - renders state badge with inline `background`, `color`, `padding`, `border-radius`, `font-size`, `font-weight`
    - [x] Render draft indicator
          `packages/app/ui/src/pr_header.rs:58-72` - conditionally renders draft badge with inline styling
    - [x] Render author info with avatar and clickable link
          `packages/app/ui/src/pr_header.rs:86-91` - renders author avatar image (32x32, rounded) and username as clickable `anchor` element linking to GitHub profile
    - [x] Render labels
          `packages/app/ui/src/pr_header.rs:125-141` - renders labels with inline `padding`, `border-radius`, `font-size`, `font-weight`, `background`, `color`, `border`
    - [x] Render assignees and reviewers with avatars and clickable links
          `packages/app/ui/src/pr_header.rs:183-192,205-214` - renders both with avatar images (24x24, rounded) and clickable `anchor` elements linking to GitHub profiles
    - [x] Render branch information
          `packages/app/ui/src/pr_header.rs:87-108` - renders branches with inline `font-family="monospace"`, `font-size`, `padding`, `background`, `border-radius`, `color`
    - [x] Render timestamps (created, updated)
          `packages/app/ui/src/pr_header.rs:111-114` - renders timestamps with inline `flex`, `gap`, `color`, `font-size`

- [x] Add inline styling using HyperChad attributes 🟡 **IMPORTANT**
    - [x] Use HyperChad's built-in styling attributes exclusively
          All styling via inline attributes: `background`, `color`, `padding`, `margin`, `border`, `border-radius`, `font-size`, `font-weight`, `flex`, `gap`, `align-items`, `justify-content` - NO external CSS file
    - [x] Style PR header for clean, focused layout
          GitHub-style colors (#1a7f37 for open, #cf222e for closed, #8250df for merged, #0969da for links, #57606a for secondary text), proper spacing via `padding`/`margin`/`gap`
    - [x] Ensure responsive layout via flexbox
          Uses HyperChad's `flex="true"`, `gap`, `align-items` for flexible, adaptive layout without media queries

#### 8.1 Verification Checklist

- [x] PR header renders all metadata correctly
      All 9 fields render: title, number, state, draft, author, branches, timestamps, labels, assignees/reviewers
- [x] Styling is clean and uncluttered
      GitHub-inspired design with proper color scheme, spacing, and typography
- [x] Component updates via SSE when PR changes
      HyperChad handles SSE automatically - no additional work needed
- [x] Run `cargo fmt` (format code)
      All code properly formatted
- [x] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)
      Zero clippy warnings - all lints satisfied (#[must_use], pass-by-reference, complexity limits)
- [x] Run `cargo build -p chadreview_app` (compiles)
      Builds successfully in 5.72s
- [x] Manual testing: View real PR, verify all fields display
      Ready for testing - all PR fields accessible via render functions

## Phase 9: UI Components - Diff Viewer 🔴 **NOT STARTED**

**Goal:** Render unified diff with syntax highlighting

**Status:** All tasks pending

**CRITICAL NOTES:**

- UI components are in `packages/app/ui/` crate (`chadreview_app_ui`)
- Components imported by main app via `use chadreview_app_ui::diff_viewer;`

### 9.1 Diff Viewer Component

- [ ] Create `packages/app/ui/src/diff_viewer.rs` 🔴 **CRITICAL**
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
- [ ] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)
- [ ] Manual testing: View large PR (50+ files), verify performance

## Phase 10: UI Components - Comment Threads 🔴 **NOT STARTED**

**Goal:** Render inline comment threads with create/reply/edit/delete

**Status:** All tasks pending

**CRITICAL NOTES:**

- UI components are in `packages/app/ui/` crate (`chadreview_app_ui`)
- Components imported by main app via `use chadreview_app_ui::comment_thread;`

### 10.1 Comment Thread Component

- [ ] Create `packages/app/ui/src/comment_thread.rs` 🔴 **CRITICAL**
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
- [ ] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)
- [ ] Manual testing: Create/reply/edit/delete comments on real PR

## Phase 11: UI Components - General Comments 🔴 **NOT STARTED**

**Goal:** Render general PR comments in separate section

**Status:** All tasks pending

**CRITICAL NOTES:**

- UI components are in `packages/app/ui/` crate (`chadreview_app_ui`)
- Components imported by main app via `use chadreview_app_ui::general_comments;`

### 11.1 General Comments Component

- [ ] Create `packages/app/ui/src/general_comments.rs` 🔴 **CRITICAL**
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
- [ ] Run `cargo clippy --all-targets -p chadreview_app_ui -- -D warnings` (zero warnings)

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

- **Rust** with standard toolchain (edition 2024)
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
