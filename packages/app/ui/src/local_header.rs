//! Local diff header component.
//!
//! Renders header information for local git diff views, including
//! repository path, diff specification, and commit information.

use chadreview_local_diff_models::LocalDiffInfo;
use hyperchad_template::{Containers, container};

use crate::local_comments;

/// Render the header for a local diff view.
///
/// Displays:
/// - Repository path
/// - Diff description (e.g., "main..feature", "Staged changes")
/// - Statistics (additions, deletions, files changed)
/// - Commit list (if applicable)
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_local_diff_header(info: &LocalDiffInfo) -> Containers {
    let additions = info.total_additions;
    let deletions = info.total_deletions;
    let files_changed = info.files_changed;

    container! {
        div
            padding=16
            background="#ffffff"
            border-bottom="1, #d0d7de"
            direction=column
            gap=12
        {
            // Repository path
            div
                font-size=13
                color="#57606a"
                font-family="monospace"
                padding=8
                background="#f6f8fa"
                border-radius=6
            {
                (info.repo_path.clone())
            }

            // Diff description
            div
                font-size=20
                font-weight=600
                color="#1f2328"
            {
                (info.description.clone())
            }

            // Stats row with collapse/expand controls
            div
                direction=row
                gap=16
                font-size=14
                align-items=center
                overflow-x=(hyperchad_template::LayoutOverflow::Wrap { grid: false })
            {
                span color="#1a7f37" font-weight=500 {
                    "+"
                    (additions.to_string())
                }
                span color="#cf222e" font-weight=500 {
                    "-"
                    (deletions.to_string())
                }
                span color="#57606a" {
                    (files_changed.to_string())
                    " files changed"
                }

                @if info.is_dirty {
                    span
                        color="#9a6700"
                        background="#fff8c5"
                        padding-x=8
                        padding-y=2
                        border-radius=12
                        font-size=12
                        font-weight=500
                    {
                        "Working tree dirty"
                    }
                }

                // Spacer to push collapse controls to the right
                div flex=1 {}

                // Collapse/Expand everything controls (files + comments, but not replies)
                (local_comments::render_header_collapse_everything_controls())
            }

            // Ref information (if available)
            @if info.base_ref.is_some() || info.head_ref.is_some() {
                div
                    direction=row
                    gap=8
                    align-items=center
                    font-size=13
                {
                    @if let Some(base) = &info.base_ref {
                        span
                            font-family="monospace"
                            background="#ddf4ff"
                            color="#0969da"
                            padding-x=8
                            padding-y=4
                            border-radius=6
                        {
                            (base.clone())
                        }
                    }

                    @if info.base_ref.is_some() && info.head_ref.is_some() {
                        span color="#57606a" { ".." }
                    }

                    @if let Some(head) = &info.head_ref {
                        span
                            font-family="monospace"
                            background="#ddf4ff"
                            color="#0969da"
                            padding-x=8
                            padding-y=4
                            border-radius=6
                        {
                            (head.clone())
                        }
                    }
                }
            }

            // Commits section (if any)
            @if !info.commits.is_empty() {
                (render_commits_section(&info.commits))
            }
        }
    }
}

/// Render the commits section showing individual commits in the diff.
fn render_commits_section(commits: &[chadreview_git_backend_models::CommitInfo]) -> Containers {
    container! {
        details
            open
            margin-top=8
        {
            summary
                cursor=pointer
                font-weight=600
                font-size=14
                padding=8
                color="#1f2328"
            {
                "Commits ("
                (commits.len().to_string())
                ")"
            }

            div
                direction=column
                gap=4
                padding-top=8
            {
                @for commit in commits.iter().take(20) {
                    div
                        direction=row
                        gap=12
                        padding=8
                        background="#f6f8fa"
                        border-radius=6
                        align-items=center
                    {
                        span
                            font-family="monospace"
                            color="#0969da"
                            font-size=13
                            min-width=70
                        {
                            (commit.short_sha.clone())
                        }

                        span
                            color="#1f2328"
                            font-size=14
                            flex=1
                        {
                            (commit.summary.clone())
                        }

                        span
                            color="#57606a"
                            font-size=12
                        {
                            (commit.author_name.clone())
                        }
                    }
                }

                @if commits.len() > 20 {
                    div
                        color="#57606a"
                        font-size=13
                        padding=8
                    {
                        "... and "
                        ((commits.len() - 20).to_string())
                        " more commits"
                    }
                }
            }
        }
    }
}
