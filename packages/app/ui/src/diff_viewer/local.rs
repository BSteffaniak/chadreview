//! Local git diff rendering with local comment support.
//!
//! This module provides UI components for rendering local git diffs
//! with local comment functionality (stored locally rather than on a
//! remote Git provider).

use chadreview_local_comment_models::{
    LineNumber as LocalLineNumber, LocalComment, LocalCommentType,
};
use chadreview_pr_models::{DiffFile, DiffLine, FileStatus, LineType};
use hyperchad::template::{Containers, LayoutOverflow, container};

use super::{
    render_diff_marker_inline, render_file_stats, render_hunk_header_row,
    render_line_numbers_inline,
};
use crate::local_comments;

/// Render diffs with local comment support.
///
/// This is used for local git diffs where comments are stored locally
/// rather than on a remote Git provider.
///
/// Files in `viewed_paths` will be rendered collapsed (header only) with
/// lazy loading for the content.
#[must_use]
pub fn render_local(
    diffs: &[DiffFile],
    comments: &[LocalComment],
    repo_path: &str,
    viewed_paths: &std::collections::HashSet<String>,
    viewed_reply_ids: &std::collections::HashSet<switchy::uuid::Uuid>,
) -> Containers {
    if diffs.is_empty() {
        return container! {
            div padding=20 color="#57606a" {
                "No changes in this diff."
            }
        };
    }

    container! {
        section padding=20 gap=24 {
            // Header with title and collapse/expand buttons
            div direction=row align-items=center justify-content=space-between margin-bottom=16 {
                h2 font-size=20 font-weight=600 color="#24292f" {
                    "Files changed"
                }
                div direction=row gap=8 overflow-x=(LayoutOverflow::Wrap { grid: false }) {
                    // Collapse/Expand everything (files + all comments + replies)
                    (local_comments::render_collapse_everything_controls())
                    // Collapse/Expand all files
                    (local_comments::render_collapse_all_files_controls())
                    // Collapse/Expand all file comments
                    (local_comments::render_collapse_all_file_comments_controls())
                }
            }
            @for diff_file in diffs {
                @let is_viewed = viewed_paths.contains(&diff_file.filename);
                @if is_viewed {
                    (render_file_collapsed(diff_file, repo_path))
                } @else {
                    (render_file_expanded(diff_file, comments, repo_path, false, viewed_reply_ids))
                }
            }
        }
    }
}

/// Generate a unique ID for a file container.
#[must_use]
pub fn file_container_id(path: &str) -> String {
    format!("file-{}", local_comments::classify_name(path))
}

/// Generate a unique ID for a file content container.
#[must_use]
pub fn file_content_id(path: &str) -> String {
    format!("file-content-{}", local_comments::classify_name(path))
}

/// Generate a unique ID for the collapse button (▼).
#[must_use]
pub fn file_collapse_btn_id(path: &str) -> String {
    format!("file-collapse-btn-{}", local_comments::classify_name(path))
}

/// Generate a unique ID for the expand button (▶).
#[must_use]
pub fn file_expand_btn_id(path: &str) -> String {
    format!("file-expand-btn-{}", local_comments::classify_name(path))
}

/// Render a collapsed file (header only, for viewed files).
///
/// The content is not rendered and will be lazy-loaded when expanded.
#[must_use]
pub fn render_file_collapsed(file: &DiffFile, repo_path: &str) -> Containers {
    let container_id = file_container_id(&file.filename);

    container! {
        div
            id=(container_id)
            border="1px solid #d0d7de"
            border-radius=6
        {
            (render_file_header_local(file, repo_path, true, true, true))
        }
    }
}

/// Render an expanded file (header + full diff content).
#[must_use]
pub fn render_file_expanded(
    file: &DiffFile,
    comments: &[LocalComment],
    repo_path: &str,
    is_viewed: bool,
    viewed_reply_ids: &std::collections::HashSet<switchy::uuid::Uuid>,
) -> Containers {
    let container_id = file_container_id(&file.filename);
    let content_id = file_content_id(&file.filename);

    container! {
        div
            id=(container_id)
            border="1px solid #d0d7de"
            border-radius=6
        {
            // Header (always visible)
            (render_file_header_local(file, repo_path, is_viewed, false, false))
            // Content container (collapsible via fx-click)
            div id=(content_id) class="file-content" {
                table width=100% {
                    // File-level comments
                    @let file_comments: Vec<_> = comments
                        .iter()
                        .filter(|c| {
                            matches!(
                                &c.comment_type,
                                LocalCommentType::FileLevelComment { path } if path == &file.filename
                            )
                        })
                        .collect();
                    @if !file_comments.is_empty() {
                        tbody {
                            tr {
                                td columns=3 {
                                    div
                                        id=(local_comments::local_file_comments_container_id(&file.filename))
                                        direction=column
                                        gap=12
                                        padding=12
                                        background="#f6f8fa"
                                        margin-bottom=12
                                    {
                                        @for comment in &file_comments {
                                            (local_comments::render_local_comment_with_reply(comment, repo_path, viewed_reply_ids))
                                        }
                                    }
                                }
                            }
                        }
                    } @else {
                        tbody {
                            tr {
                                td columns=3 {
                                    div id=(local_comments::local_file_comments_container_id(&file.filename)) {}
                                }
                            }
                        }
                    }
                    // File comment form
                    tbody {
                        tr {
                            td columns=3 {
                                (local_comments::render_local_file_comment_form(repo_path, &file.filename))
                            }
                        }
                    }
                    // Hunks
                    @for hunk in &file.hunks {
                        (render_hunk_header_row(hunk))
                        tbody font-family="monospace" font-size=12 {
                            @for line in &hunk.lines {
                                (render_line_row_local(&file.filename, line, comments, repo_path, viewed_reply_ids))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render file header with collapse/expand, viewed checkbox, and comment button.
///
/// # Arguments
/// * `file` - The diff file
/// * `repo_path` - Repository path (for API calls)
/// * `is_viewed` - Whether the file is marked as viewed
/// * `is_collapsed` - Whether the file content is currently collapsed
/// * `lazy_load` - If true, expand button uses hx-get; if false, uses client-side fx-click
#[allow(clippy::too_many_lines)]
fn render_file_header_local(
    file: &DiffFile,
    repo_path: &str,
    is_viewed: bool,
    is_collapsed: bool,
    lazy_load: bool,
) -> Containers {
    use hyperchad::transformer::models::Selector;

    let (status_text, status_color) = match file.status {
        FileStatus::Added => ("Added", "#1a7f37"),
        FileStatus::Modified => ("Modified", "#0969da"),
        FileStatus::Deleted => ("Deleted", "#cf222e"),
        FileStatus::Renamed => ("Renamed", "#8250df"),
    };

    let container_id = file_container_id(&file.filename);
    let content_id = file_content_id(&file.filename);
    let collapse_btn_id = file_collapse_btn_id(&file.filename);
    let expand_btn_id = file_expand_btn_id(&file.filename);

    // URL for lazy loading diff content (when expanding a collapsed file)
    let expand_url = format!(
        "/api/local/file/diff?repo={}&path={}",
        urlencoding::encode(repo_path),
        urlencoding::encode(&file.filename),
    );

    // URL for marking file as viewed/unviewed
    let view_url = format!(
        "/api/local/file/view?repo={}&path={}",
        urlencoding::encode(repo_path),
        urlencoding::encode(&file.filename),
    );

    container! {
        div
            padding=12
            background=(if is_viewed { "#f0f6fc" } else { "#f6f8fa" })
            direction=row
            align-items=center
            justify-content=space-between
        {
            div
                direction=row
                align-items=center
                gap=12
                overflow-x=(LayoutOverflow::Wrap { grid: false })
            {
                // Collapse/Expand toggle buttons
                // We use two buttons that swap visibility for client-side toggle
                @if lazy_load {
                    // Lazy load mode: only expand button, uses hx-get to fetch content
                    button
                        id=(expand_btn_id)
                        type=button
                        padding=4
                        cursor=pointer
                        background="transparent"
                        font-size=14
                        color="#57606a"
                        hx-get=(expand_url)
                        hx-target=(Selector::Id(container_id.clone()))
                        hx-swap="outerHTML"
                    {
                        "▶"
                    }
                } @else {
                    // Client-side toggle mode: two buttons that swap visibility
                    // Collapse button (▼) - visible when expanded
                    @let collapse_id_1 = collapse_btn_id.clone();
                    @let expand_id_1 = expand_btn_id.clone();
                    @let content_id_1 = content_id.clone();
                    button
                        id=(collapse_btn_id.clone())
                        class="file-collapse-btn"
                        type=button
                        padding=4
                        cursor=pointer
                        background="transparent"
                        font-size=14
                        color="#57606a"
                        hidden=(is_collapsed)
                        fx-click=fx { element_by_id(content_id_1).no_display(); element_by_id(collapse_id_1).no_display(); element_by_id(expand_id_1).display() }
                    {
                        "▼"
                    }
                    // Expand button (▶) - visible when collapsed
                    @let collapse_id_2 = collapse_btn_id.clone();
                    @let expand_id_2 = expand_btn_id.clone();
                    @let content_id_2 = content_id.clone();
                    button
                        id=(expand_btn_id.clone())
                        class="file-expand-btn"
                        type=button
                        padding=4
                        cursor=pointer
                        background="transparent"
                        font-size=14
                        color="#57606a"
                        hidden=(!is_collapsed)
                        fx-click=fx { element_by_id(content_id_2).display(); element_by_id(expand_id_2).no_display(); element_by_id(collapse_id_2).display() }
                    {
                        "▶"
                    }
                }

                // Viewed checkbox
                @if is_viewed {
                    button
                        type=button
                        padding-x=8
                        padding-y=2
                        cursor=pointer
                        background="#ddf4ff"
                        border="1px solid #0969da"
                        border-radius=4
                        font-size=12
                        color="#0969da"
                        hx-delete=(view_url)
                        hx-target=(Selector::Id(container_id.clone()))
                        hx-swap="outerHTML"
                    {
                        "✓ Viewed"
                    }
                } @else if lazy_load {
                    // No content loaded yet, just server request
                    button
                        type=button
                        padding-x=8
                        padding-y=2
                        cursor=pointer
                        background="#ffffff"
                        border="1px solid #d0d7de"
                        border-radius=4
                        font-size=12
                        color="#57606a"
                        hx-post=(view_url)
                        hx-target=(Selector::Id(container_id))
                        hx-swap="outerHTML"
                    {
                        "Mark as viewed"
                    }
                } @else {
                    // Content loaded - immediate collapse + server request
                    @let content_id_3 = content_id.clone();
                    @let collapse_id_3 = collapse_btn_id.clone();
                    @let expand_id_3 = expand_btn_id.clone();
                    button
                        type=button
                        padding-x=8
                        padding-y=2
                        cursor=pointer
                        background="#ffffff"
                        border="1px solid #d0d7de"
                        border-radius=4
                        font-size=12
                        color="#57606a"
                        hx-post=(view_url)
                        hx-target=(Selector::Id(container_id))
                        hx-swap="outerHTML"
                        fx-click=fx { element_by_id(content_id_3).no_display(); element_by_id(collapse_id_3).no_display(); element_by_id(expand_id_3).display() }
                    {
                        "Mark as viewed"
                    }
                }

                // Status badge
                span
                    padding-y=2
                    padding-x=8
                    border-radius=4
                    font-size=12
                    font-weight=600
                    background=(status_color)
                    color="#ffffff"
                {
                    (status_text)
                }

                // Filename
                div overflow-x=(LayoutOverflow::Wrap { grid: true }) {
                    span
                        font-family="monospace"
                        font-size=14
                        font-weight=600
                        color=(if is_viewed { "#57606a" } else { "#24292f" })
                        overflow-x=hidden
                        text-overflow=ellipsis
                    {
                        (file.filename)
                    }
                }
            }
            div direction=row align-items=center gap=12 {
                @if !is_collapsed {
                    (local_comments::render_file_comment_controls(&file.filename))
                    (local_comments::render_local_file_comment_button(&file.filename))
                }
                (render_file_stats(file))
            }
        }
    }
}

/// Render a line row with local comment support.
fn render_line_row_local(
    file_path: &str,
    diff_line: &DiffLine,
    comments: &[LocalComment],
    repo_path: &str,
    viewed_reply_ids: &std::collections::HashSet<switchy::uuid::Uuid>,
) -> Containers {
    let line = diff_line_to_local_line_number(diff_line);
    let bg_color = match diff_line.line_type {
        LineType::Addition => "#e6ffec",
        LineType::Deletion => "#ffebe9",
        LineType::Context => "#ffffff",
    };

    let add_comment_button_id = local_comments::local_add_comment_button_id(file_path, line);

    container! {
        tr {
            (render_line_numbers_inline(diff_line))

            td {
                div
                    direction=row
                    position=relative
                    fx-hover=fx { element_by_id(add_comment_button_id).display() }
                {
                    div
                        width=20
                        background=(bg_color)
                        padding-y=4
                        color="#57606a"
                        user-select=none
                        justify-content=center
                        align-items=center
                    {
                        (render_diff_marker_inline(diff_line))
                    }

                    (local_comments::render_local_add_comment_button(file_path, line))

                    div
                        flex=1
                        flex-shrink=0
                        background=(bg_color)
                        padding-x=4
                        justify-content=center
                    {
                        div
                            white-space=preserve-wrap
                            user-select=text
                            font-family="monospace"
                            font-size=12
                            overflow-wrap=anywhere
                        {
                            raw { (diff_line.highlighted_html) }
                        }
                    }
                }
            }
        }
        // Line comments row
        tr {
            td columns=3 {
                (local_comments::render_local_line_comments(comments, file_path, line, repo_path, viewed_reply_ids))
                (local_comments::render_local_create_comment_form(repo_path, file_path, line))
            }
        }
    }
}

/// Convert a `DiffLine` to a `LocalLineNumber`.
const fn diff_line_to_local_line_number(diff_line: &DiffLine) -> LocalLineNumber {
    // Prefer new line number, fall back to old
    if let Some(new) = diff_line.new_line_number {
        LocalLineNumber::New { line: new }
    } else if let Some(old) = diff_line.old_line_number {
        LocalLineNumber::Old { line: old }
    } else {
        // Context line without numbers - use 0 as fallback
        LocalLineNumber::New { line: 0 }
    }
}
