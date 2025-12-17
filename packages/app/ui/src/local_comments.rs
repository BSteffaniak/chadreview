//! Local comments UI components.
//!
//! This module provides UI components for rendering local comments,
//! including AI action status displays with real-time updates via SSE,
//! and line-level/file-level comment forms for local diffs.

use std::collections::HashSet;

use chadreview_local_comment_models::{
    AiExecutionStatus, ExecutionDetails, LineNumber, LocalComment, LocalCommentType, ProgressEntry,
    ThreadState,
};
use hyperchad::markdown::markdown_to_container;
use hyperchad::template::container;
use hyperchad::transformer::models::Selector;
use hyperchad_router::Container;
use switchy::uuid::Uuid;

// =============================================================================
// ID Generation Functions
// =============================================================================

/// Sanitize a string for use in element IDs.
#[must_use]
pub fn classify_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
}

/// Generate a unique `str_id` for an AI status container.
///
/// This ID is used by `HyperChad` SSE to target the container for partial updates.
#[must_use]
pub fn ai_status_str_id(comment_id: Uuid) -> String {
    format!("ai-status-{comment_id}")
}

/// Generate ID for the "+" button on a line.
#[must_use]
pub fn local_add_comment_button_id(file_path: &str, line: LineNumber) -> String {
    format!("local-add-comment-btn-{}-{line}", classify_name(file_path))
}

/// Generate ID for the comment form on a line.
#[must_use]
pub fn local_comment_form_id(file_path: &str, line: LineNumber) -> String {
    format!("local-comment-form-{}-{line}", classify_name(file_path))
}

/// Generate ID for the comment thread container on a line.
#[must_use]
pub fn local_line_comments_container_id(file_path: &str, line: LineNumber) -> String {
    format!("local-line-comments-{}-{line}", classify_name(file_path))
}

/// Generate ID for the file-level comment button.
#[must_use]
pub fn local_file_comment_button_id(file_path: &str) -> String {
    format!("local-file-comment-btn-{}", classify_name(file_path))
}

/// Generate ID for the file-level comment form.
#[must_use]
pub fn local_file_comment_form_id(file_path: &str) -> String {
    format!("local-file-comment-form-{}", classify_name(file_path))
}

/// Generate ID for the file-level comments container.
#[must_use]
pub fn local_file_comments_container_id(file_path: &str) -> String {
    format!("local-file-comments-{}", classify_name(file_path))
}

/// Generate ID for the reply form on a comment.
#[must_use]
pub fn local_reply_form_id(comment_id: Uuid) -> String {
    format!("local-reply-form-{comment_id}")
}

/// Generate ID for a comment thread container.
#[must_use]
pub fn local_comment_thread_id(comment_id: Uuid) -> String {
    format!("local-comment-thread-{comment_id}")
}

/// Generate ID for the replies container within a thread.
#[must_use]
pub fn local_thread_replies_id(comment_id: Uuid) -> String {
    format!("local-thread-replies-{comment_id}")
}

/// Generate ID for the collapse button (▼) on a comment thread.
#[must_use]
pub fn local_comment_collapse_btn_id(comment_id: Uuid) -> String {
    format!("local-comment-collapse-btn-{comment_id}")
}

/// Generate ID for the expand button (▶) on a comment thread.
#[must_use]
pub fn local_comment_expand_btn_id(comment_id: Uuid) -> String {
    format!("local-comment-expand-btn-{comment_id}")
}

/// Generate ID for the collapsible body container of a comment thread.
#[must_use]
pub fn local_comment_body_id(comment_id: Uuid) -> String {
    format!("local-comment-body-{comment_id}")
}

/// Generate ID for the collapse button (▼) on a reply.
#[must_use]
pub fn local_reply_collapse_btn_id(comment_id: Uuid) -> String {
    format!("local-reply-collapse-btn-{comment_id}")
}

/// Generate ID for the expand button (▶) on a reply.
#[must_use]
pub fn local_reply_expand_btn_id(comment_id: Uuid) -> String {
    format!("local-reply-expand-btn-{comment_id}")
}

/// Generate ID for the collapsible body container of a reply.
#[must_use]
pub fn local_reply_body_id(comment_id: Uuid) -> String {
    format!("local-reply-body-{comment_id}")
}

// =============================================================================
// Comment Thread Controls
// =============================================================================

/// Render "Collapse all" / "Expand all" buttons for general comment threads.
///
/// These buttons use CSS class selectors to toggle all general comment threads.
#[must_use]
pub fn render_general_comment_controls() -> Container {
    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".comment-general-body").no_display();
                    element(".comment-general-collapse-btn").no_display();
                    element(".comment-general-expand-btn").display()
                }
            {
                "Collapse all"
            }
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".comment-general-body").display();
                    element(".comment-general-expand-btn").no_display();
                    element(".comment-general-collapse-btn").display()
                }
            {
                "Expand all"
            }
        }
    }
    .into()
}

/// Render "Collapse comments" / "Expand comments" buttons for a specific file.
///
/// These buttons use file-specific CSS class selectors to toggle only that file's comments.
#[must_use]
pub fn render_file_comment_controls(file_path: &str) -> Container {
    let path_class = classify_name(file_path);
    let body_class = format!(".comment-file-body-{path_class}");
    let collapse_class = format!(".comment-file-collapse-btn-{path_class}");
    let expand_class = format!(".comment-file-expand-btn-{path_class}");

    // Clone for use in both fx-click handlers
    let body_class2 = body_class.clone();
    let collapse_class2 = collapse_class.clone();
    let expand_class2 = expand_class.clone();

    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=8
                padding-y=4
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(body_class).no_display();
                    element(collapse_class).no_display();
                    element(expand_class).display()
                }
            {
                "Collapse comments"
            }
            button
                type=button
                padding-x=8
                padding-y=4
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(body_class2).display();
                    element(expand_class2).no_display();
                    element(collapse_class2).display()
                }
            {
                "Expand comments"
            }
        }
    }
    .into()
}

/// Render "Collapse everything" / "Expand everything" buttons.
///
/// These buttons collapse/expand all files, all comment threads, and all replies.
#[must_use]
pub fn render_collapse_everything_controls() -> Container {
    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").no_display();
                    element(".file-collapse-btn").no_display();
                    element(".file-expand-btn").display();
                    element(".comment-thread-body").no_display();
                    element(".comment-thread-collapse-btn").no_display();
                    element(".comment-thread-expand-btn").display();
                    element(".comment-reply-body").no_display();
                    element(".comment-reply-collapse-btn").no_display();
                    element(".comment-reply-expand-btn").display()
                }
            {
                "Collapse everything"
            }
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").display();
                    element(".file-expand-btn").no_display();
                    element(".file-collapse-btn").display();
                    element(".comment-thread-body").display();
                    element(".comment-thread-expand-btn").no_display();
                    element(".comment-thread-collapse-btn").display();
                    element(".comment-reply-body").display();
                    element(".comment-reply-expand-btn").no_display();
                    element(".comment-reply-collapse-btn").display()
                }
            {
                "Expand everything"
            }
        }
    }
    .into()
}

/// Render "Collapse everything" / "Expand everything" buttons for the main header.
///
/// These buttons collapse/expand:
/// - All files (file diff content)
/// - All comment threads (general + file/line comments)
///
/// Unlike `render_collapse_everything_controls`, this does NOT affect replies,
/// so users can still see which replies they haven't read yet.
#[must_use]
pub fn render_header_collapse_everything_controls() -> Container {
    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").no_display();
                    element(".file-collapse-btn").no_display();
                    element(".file-expand-btn").display();
                    element(".comment-thread-body").no_display();
                    element(".comment-thread-collapse-btn").no_display();
                    element(".comment-thread-expand-btn").display()
                }
            {
                "Collapse everything"
            }
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").display();
                    element(".file-expand-btn").no_display();
                    element(".file-collapse-btn").display();
                    element(".comment-thread-body").display();
                    element(".comment-thread-expand-btn").no_display();
                    element(".comment-thread-collapse-btn").display()
                }
            {
                "Expand everything"
            }
        }
    }
    .into()
}

/// Render "Collapse all files" / "Expand all files" buttons.
#[must_use]
pub fn render_collapse_all_files_controls() -> Container {
    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").no_display();
                    element(".file-collapse-btn").no_display();
                    element(".file-expand-btn").display()
                }
            {
                "Collapse all files"
            }
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".file-content").display();
                    element(".file-expand-btn").no_display();
                    element(".file-collapse-btn").display()
                }
            {
                "Expand all files"
            }
        }
    }
    .into()
}

/// Render "Collapse all file comments" / "Expand all file comments" buttons.
///
/// These buttons collapse/expand all file-level and line-level comment threads (not general comments).
#[must_use]
pub fn render_collapse_all_file_comments_controls() -> Container {
    container! {
        div direction=row gap=8 {
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".comment-file-body").no_display();
                    element(".comment-file-collapse-btn").no_display();
                    element(".comment-file-expand-btn").display()
                }
            {
                "Collapse all file comments"
            }
            button
                type=button
                padding-x=12
                padding-y=6
                cursor=pointer
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                font-size=12
                color="#24292f"
                fx-click=fx {
                    element(".comment-file-body").display();
                    element(".comment-file-expand-btn").no_display();
                    element(".comment-file-collapse-btn").display()
                }
            {
                "Expand all file comments"
            }
        }
    }
    .into()
}

// =============================================================================
// Line-Level Comment UI
// =============================================================================

/// Render the "+" button for adding a line-level comment.
///
/// This button is hidden by default and shown on hover via `fx-hover`.
#[must_use]
pub fn render_local_add_comment_button(file_path: &str, line: LineNumber) -> Container {
    let button_id = local_add_comment_button_id(file_path, line);
    let form_id = local_comment_form_id(file_path, line);
    let size = 18;

    container! {
        button
            id=(button_id)
            hidden
            type=button
            position=absolute
            width=(size)
            height=(size)
            left=0
            top=calc(50% - size / 2)
            align-items=center
            justify-content=center
            background="#1f6feb"
            border-radius=3
            color=white
            cursor=pointer
            font-size=12
            opacity=0.8
            user-select=none
            fx-click=fx { element_by_id(form_id).display() }
        {
            "+"
        }
    }
    .into()
}

/// Render the comment form for a specific line.
///
/// This form is hidden by default and shown when the user clicks the "+" button.
#[must_use]
pub fn render_local_create_comment_form(
    repo_path: &str,
    file_path: &str,
    line: LineNumber,
) -> Container {
    let form_id = local_comment_form_id(file_path, line);
    let container_id = local_line_comments_container_id(file_path, line);
    let api_url = format!("/api/local/comment?repo={}", urlencoding::encode(repo_path));

    let (side, line_num) = match line {
        LineNumber::New { line } => ("new", line),
        LineNumber::Old { line } => ("old", line),
    };

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
            hx-swap="beforeend"
            hx-target=(Selector::Id(container_id))
            fx-http-success=fx { no_display_self() }
        {
            div
                padding=12
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                gap=8
                margin-top=8
                margin-bottom=8
            {
                input type=hidden name="type" value="line_level";
                input type=hidden name="path" value=(file_path);
                input type=hidden name="line" value=(line_num);
                input type=hidden name="side" value=(side);

                textarea
                    name="body"
                    placeholder="Add a comment..."
                    height=80
                    padding=8
                    border="1px solid #d0d7de"
                    border-radius=6
                    font-size=14;

                div margin-top=8 {
                    (render_ai_action_selector("ai_agent", DEFAULT_AGENTS))
                }

                div direction=row gap=8 margin-top=8 {
                    button
                        type=submit
                        background="#1a7f37"
                        color="#ffffff"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        font-weight=600
                        font-size=14
                        cursor=pointer
                    {
                        "Comment"
                    }
                    button
                        type=button
                        color="#57606a"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        cursor=pointer
                        font-size=14
                        fx-click=fx { element_by_id(form_id).no_display() }
                    {
                        "Cancel"
                    }
                }
            }
        }
    }
    .into()
}

/// Render existing line-level comments for a specific line.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn render_local_line_comments(
    comments: &[LocalComment],
    file_path: &str,
    line: LineNumber,
    repo_path: &str,
    viewed_reply_ids: &HashSet<Uuid>,
) -> Container {
    let container_id = local_line_comments_container_id(file_path, line);

    // Filter comments for this line
    let line_comments: Vec<_> = comments
        .iter()
        .filter(|c| {
            matches!(
                &c.comment_type,
                LocalCommentType::LineLevelComment { path, line: l }
                if path == file_path && *l == line
            )
        })
        .collect();

    container! {
        div id=(container_id) gap=8 {
            @for comment in &line_comments {
                (render_local_comment_with_reply(comment, repo_path, viewed_reply_ids))
            }
        }
    }
    .into()
}

// =============================================================================
// File-Level Comment UI
// =============================================================================

/// Render the file-level comment button (appears in file header).
#[must_use]
pub fn render_local_file_comment_button(file_path: &str) -> Container {
    let button_id = local_file_comment_button_id(file_path);
    let form_id = local_file_comment_form_id(file_path);

    container! {
        button
            id=(button_id)
            type=button
            padding-x=8
            padding-y=4
            background="#f6f8fa"
            border="1px solid #d0d7de"
            border-radius=6
            cursor=pointer
            font-size=12
            color="#57606a"
            fx-click=fx { element_by_id(form_id).display() }
        {
            "💬 Comment"
        }
    }
    .into()
}

/// Render the file-level comment form.
#[must_use]
pub fn render_local_file_comment_form(repo_path: &str, file_path: &str) -> Container {
    let form_id = local_file_comment_form_id(file_path);
    let container_id = local_file_comments_container_id(file_path);
    let api_url = format!("/api/local/comment?repo={}", urlencoding::encode(repo_path));

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
            hx-swap="beforeend"
            hx-target=(Selector::Id(container_id))
            fx-http-success=fx { no_display_self() }
        {
            div
                padding=12
                background="#f6f8fa"
                border="1px solid #d0d7de"
                border-radius=6
                gap=8
                margin=12
            {
                input type=hidden name="type" value="file_level";
                input type=hidden name="path" value=(file_path);

                textarea
                    name="body"
                    placeholder="Add a file-level comment..."
                    height=80
                    padding=8
                    border="1px solid #d0d7de"
                    border-radius=6
                    font-size=14
                    background="#ffffff";

                div margin-top=8 {
                    (render_ai_action_selector("ai_agent", DEFAULT_AGENTS))
                }

                div direction=row gap=8 margin-top=8 {
                    button
                        type=submit
                        background="#1a7f37"
                        color="#ffffff"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        font-weight=600
                        font-size=14
                        cursor=pointer
                    {
                        "Comment"
                    }
                    button
                        type=button
                        color="#57606a"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        cursor=pointer
                        font-size=14
                        fx-click=fx { element_by_id(form_id).no_display() }
                    {
                        "Cancel"
                    }
                }
            }
        }
    }
    .into()
}

/// Render existing file-level comments.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn render_local_file_comments(
    comments: &[LocalComment],
    file_path: &str,
    repo_path: &str,
    viewed_reply_ids: &HashSet<Uuid>,
) -> Container {
    let container_id = local_file_comments_container_id(file_path);

    // Filter comments for this file (file-level only)
    let file_comments: Vec<_> = comments
        .iter()
        .filter(|c| {
            matches!(
                &c.comment_type,
                LocalCommentType::FileLevelComment { path } if path == file_path
            )
        })
        .collect();

    container! {
        div id=(container_id) gap=8 padding=12 {
            @for comment in &file_comments {
                (render_local_comment_with_reply(comment, repo_path, viewed_reply_ids))
            }
        }
    }
    .into()
}

// =============================================================================
// Reply UI
// =============================================================================

/// Render the reply form for a comment thread.
#[must_use]
pub fn render_local_reply_form(repo_path: &str, thread_id: Uuid) -> Container {
    let form_id = local_reply_form_id(thread_id);
    let thread_container_id = local_comment_thread_id(thread_id);
    let api_url = format!(
        "/api/local/comment/reply?repo={}",
        urlencoding::encode(repo_path)
    );

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
            hx-swap="beforeend"
            hx-target=(Selector::Id(thread_container_id))
            fx-http-success=fx { no_display_self() }
        {
            div
                padding=12
                background="#f6f8fa"
                border="1px solid #d0d7de"
                border-radius=6
                gap=8
                margin-top=8
            {
                input type=hidden name="thread_id" value=(thread_id);

                textarea
                    name="body"
                    placeholder="Write a reply..."
                    height=60
                    padding=8
                    border="1px solid #d0d7de"
                    border-radius=6
                    font-size=14
                    background="#ffffff";

                div margin-top=8 {
                    (render_ai_action_selector("ai_agent", DEFAULT_AGENTS))
                }

                div direction=row gap=8 margin-top=8 {
                    button
                        type=submit
                        background="#1a7f37"
                        color="#ffffff"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        font-weight=600
                        font-size=14
                        cursor=pointer
                    {
                        "Reply"
                    }
                    button
                        type=button
                        color="#57606a"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        cursor=pointer
                        font-size=14
                        fx-click=fx { element_by_id(form_id).no_display() }
                    {
                        "Cancel"
                    }
                }
            }
        }
    }
    .into()
}

/// Render a reply button for a comment.
#[must_use]
pub fn render_local_reply_button(comment_id: Uuid) -> Container {
    let form_id = local_reply_form_id(comment_id);

    container! {
        button
            type=button
            color="#0969da"
            padding-x=8
            padding-y=4
            cursor=pointer
            font-size=12
            fx-click=fx { element_by_id(form_id).display() }
        {
            "Reply"
        }
    }
    .into()
}

/// Render a delete button for a comment.
///
/// For root comments (`thread_id` == `comment_id`), deletes the entire thread.
/// For replies, deletes just that reply.
#[must_use]
pub fn render_local_delete_button(thread_id: Uuid, comment_id: Uuid, repo_path: &str) -> Container {
    let delete_url = format!(
        "/api/local/comment/delete?repo={}&thread_id={}&comment_id={}",
        urlencoding::encode(repo_path),
        thread_id,
        comment_id
    );

    // Target the appropriate container for deletion
    let target_id = if thread_id == comment_id {
        // Deleting whole thread - target the thread container
        local_comment_thread_id(thread_id)
    } else {
        // Deleting a reply - target just the comment div
        format!("comment-{comment_id}")
    };

    container! {
        button
            type=button
            color="#57606a"
            padding-x=8
            padding-y=4
            cursor=pointer
            font-size=12
            hx-delete=(delete_url)
            hx-target=(Selector::Id(target_id))
            hx-swap="delete"
        {
            "Delete"
        }
    }
    .into()
}

/// Render state action buttons for a comment thread.
///
/// Shows different buttons based on current state:
/// - Open: "Resolve" (green) + "Later" (orange)
/// - Resolved: "Reopen" (gray)
/// - `SavedForLater`: "Reopen" (gray) + "Resolve" (green)
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_local_state_buttons(
    thread_id: Uuid,
    state: ThreadState,
    repo_path: &str,
) -> Container {
    let target_id = local_comment_thread_id(thread_id);
    let encoded_repo = urlencoding::encode(repo_path);

    let resolve_url = format!(
        "/api/local/comment/state?repo={encoded_repo}&thread_id={thread_id}&state=resolved"
    );
    let later_url = format!(
        "/api/local/comment/state?repo={encoded_repo}&thread_id={thread_id}&state=saved_for_later"
    );
    let reopen_url =
        format!("/api/local/comment/state?repo={encoded_repo}&thread_id={thread_id}&state=open");

    match state {
        ThreadState::Open => {
            // Show "Resolve" (green) + "Later" (orange)
            container! {
                div direction=row gap=4 {
                    button
                        type=button
                        color="#1a7f37"
                        background="#ffffff"
                        padding-x=8
                        padding-y=4
                        cursor=pointer
                        font-size=12
                        hx-post=(resolve_url)
                        hx-target=(Selector::Id(target_id.clone()))
                        hx-swap="outerHTML"
                    {
                        "Resolve"
                    }
                    button
                        type=button
                        color="#bf8700"
                        background="#ffffff"
                        padding-x=8
                        padding-y=4
                        cursor=pointer
                        font-size=12
                        hx-post=(later_url)
                        hx-target=(Selector::Id(target_id))
                        hx-swap="outerHTML"
                    {
                        "Later"
                    }
                }
            }
            .into()
        }
        ThreadState::Resolved => {
            // Show "Reopen" (gray)
            container! {
                button
                    type=button
                    color="#57606a"
                    background="#ffffff"
                    padding-x=8
                    padding-y=4
                    cursor=pointer
                    font-size=12
                    hx-post=(reopen_url)
                    hx-target=(Selector::Id(target_id))
                    hx-swap="outerHTML"
                {
                    "Reopen"
                }
            }
            .into()
        }
        ThreadState::SavedForLater => {
            // Show "Reopen" (gray) + "Resolve" (green)
            container! {
                div direction=row gap=4 {
                    button
                        type=button
                        color="#57606a"
                        background="#ffffff"
                        padding-x=8
                        padding-y=4
                        cursor=pointer
                        font-size=12
                        hx-post=(reopen_url)
                        hx-target=(Selector::Id(target_id.clone()))
                        hx-swap="outerHTML"
                    {
                        "Reopen"
                    }
                    button
                        type=button
                        color="#1a7f37"
                        background="#ffffff"
                        padding-x=8
                        padding-y=4
                        cursor=pointer
                        font-size=12
                        hx-post=(resolve_url)
                        hx-target=(Selector::Id(target_id))
                        hx-swap="outerHTML"
                    {
                        "Resolve"
                    }
                }
            }
            .into()
        }
    }
}

/// Render a "Viewed" / "Mark viewed" button for a reply.
///
/// Toggles the viewed state via POST/DELETE.
/// When marking as viewed, uses `fx-click` for immediate client-side collapse
/// while the server request happens in the background.
#[must_use]
pub fn render_local_reply_view_button(
    thread_id: Uuid,
    reply_id: Uuid,
    is_viewed: bool,
    repo_path: &str,
) -> Container {
    let view_url = format!(
        "/api/local/reply/view?repo={}&thread_id={}&reply_id={}",
        urlencoding::encode(repo_path),
        thread_id,
        reply_id
    );

    let target_id = format!("comment-{reply_id}");

    // Get element IDs for fx-click collapse
    let body_id = local_reply_body_id(reply_id);
    let collapse_btn_id = local_reply_collapse_btn_id(reply_id);
    let expand_btn_id = local_reply_expand_btn_id(reply_id);

    // When marking as viewed: collapse immediately via fx-click, then server updates
    // When unmarking: just server request (expand will happen via server response)
    if is_viewed {
        container! {
            button
                type=button
                color="#0969da"
                background="#ddf4ff"
                border="1px solid #0969da"
                padding-x=8
                padding-y=2
                cursor=pointer
                font-size=12
                border-radius=4
                hx-delete=(view_url)
                hx-target=(Selector::Id(target_id))
                hx-swap="outerHTML"
            {
                "Viewed"
            }
        }
        .into()
    } else {
        container! {
            button
                type=button
                color="#57606a"
                padding-x=8
                padding-y=4
                cursor=pointer
                font-size=12
                hx-post=(view_url)
                hx-target=(Selector::Id(target_id))
                hx-swap="outerHTML"
                fx-click=fx {
                    element_by_id(body_id).no_display();
                    element_by_id(collapse_btn_id).no_display();
                    element_by_id(expand_btn_id).display()
                }
            {
                "Mark viewed"
            }
        }
        .into()
    }
}

// =============================================================================
// Comment Rendering
// =============================================================================

/// Render a local comment thread with collapse/expand and reply functionality.
///
/// The thread header (author, time, resolved badge, toggle buttons) is always visible.
/// The body (comment content, replies, reply form) can be collapsed/expanded.
/// Resolved threads start collapsed by default.
///
/// Uses multiple CSS classes for different targeting contexts:
/// - `.comment-thread-body` - for "Collapse everything"
/// - `.comment-general-body` or `.comment-file-body` - for section-specific collapse
/// - `.comment-file-body-{path}` - for per-file collapse
#[must_use]
#[allow(clippy::implicit_hasher, clippy::too_many_lines)]
pub fn render_local_comment_with_reply(
    comment: &LocalComment,
    repo_path: &str,
    viewed_reply_ids: &HashSet<Uuid>,
) -> Container {
    let thread_container_id = local_comment_thread_id(comment.id);
    let body_id = local_comment_body_id(comment.id);
    let collapse_btn_id = local_comment_collapse_btn_id(comment.id);
    let expand_btn_id = local_comment_expand_btn_id(comment.id);

    // Resolved and SavedForLater threads start collapsed
    let is_collapsed = comment.state.is_collapsed();

    // Background color based on state
    let bg_color = match comment.state {
        ThreadState::Open => "#ffffff",
        ThreadState::Resolved => "#f0f6fc",      // Light blue
        ThreadState::SavedForLater => "#fff8e5", // Light amber/orange
    };

    let time_ago = format_time_ago(comment.created_at);

    // Build CSS classes based on comment type
    let (body_classes, collapse_btn_classes, expand_btn_classes) = match &comment.comment_type {
        LocalCommentType::General => (
            "comment-thread-body comment-general-body".to_string(),
            "comment-thread-collapse-btn comment-general-collapse-btn".to_string(),
            "comment-thread-expand-btn comment-general-expand-btn".to_string(),
        ),
        LocalCommentType::FileLevelComment { path }
        | LocalCommentType::LineLevelComment { path, .. } => {
            let path_class = classify_name(path);
            (
                format!("comment-thread-body comment-file-body comment-file-body-{path_class}"),
                format!(
                    "comment-thread-collapse-btn comment-file-collapse-btn comment-file-collapse-btn-{path_class}"
                ),
                format!(
                    "comment-thread-expand-btn comment-file-expand-btn comment-file-expand-btn-{path_class}"
                ),
            )
        }
        LocalCommentType::Reply { .. } => (
            "comment-thread-body".to_string(),
            "comment-thread-collapse-btn".to_string(),
            "comment-thread-expand-btn".to_string(),
        ),
    };

    // Clone IDs for use in fx-click closures
    let body_id_1 = body_id.clone();
    let collapse_id_1 = collapse_btn_id.clone();
    let expand_id_1 = expand_btn_id.clone();
    let body_id_2 = body_id.clone();
    let collapse_id_2 = collapse_btn_id.clone();
    let expand_id_2 = expand_btn_id.clone();

    container! {
        div
            id=(thread_container_id)
            class="comment-thread"
            background=(bg_color)
            border="1px solid #d0d7de"
            border-radius=6
            margin-bottom=8
        {
            // Thread header (always visible)
            div
                padding=12
                direction=row
                align-items=center
                gap=8
            {
                // Collapse button (▼) - visible when expanded
                button
                    id=(collapse_btn_id)
                    class=(collapse_btn_classes)
                    type=button
                    padding=4
                    cursor=pointer
                    font-size=14
                    color="#57606a"
                    hidden=(is_collapsed)
                    fx-click=fx { element_by_id(body_id_1).no_display(); element_by_id(collapse_id_1).no_display(); element_by_id(expand_id_1).display() }
                {
                    "▼"
                }
                // Expand button (▶) - visible when collapsed
                button
                    id=(expand_btn_id)
                    class=(expand_btn_classes)
                    type=button
                    padding=4
                    cursor=pointer
                    font-size=14
                    color="#57606a"
                    hidden=(!is_collapsed)
                    fx-click=fx { element_by_id(body_id_2).display(); element_by_id(expand_id_2).no_display(); element_by_id(collapse_id_2).display() }
                {
                    "▶"
                }

                // Author avatar
                div
                    width=24
                    height=24
                    border-radius=12
                    background="#d0d7de"
                    align-items=center
                    justify-content=center
                    font-size=12
                    color="#57606a"
                {
                    "👤"
                }
                span font-weight=600 font-size=14 color="#24292f" {
                    (&comment.author.name)
                }
                span font-size=12 color="#57606a" { (time_ago) }
                @match comment.state {
                    ThreadState::Resolved => {
                        span
                            padding-x=8
                            padding-y=2
                            background="#8250df"
                            color="#ffffff"
                            border-radius=4
                            font-size=12
                            font-weight=600
                        {
                            "Resolved"
                        }
                    }
                    ThreadState::SavedForLater => {
                        span
                            padding-x=8
                            padding-y=2
                            background="#bf8700"
                            color="#ffffff"
                            border-radius=4
                            font-size=12
                            font-weight=600
                        {
                            "Later"
                        }
                    }
                    ThreadState::Open => {}
                }

                // Spacer
                div flex=1 {}

                // Action buttons in header
                div direction=row gap=8 {
                    (render_local_state_buttons(comment.id, comment.state, repo_path))
                }
            }

            // Collapsible body (comment content, replies, reply form)
            div
                id=(body_id)
                class=(body_classes)
                padding=12
                padding-top=0
                gap=8
                hidden=(is_collapsed)
            {
                // Comment body content
                div color="#24292f" font-size=14 {
                    (markdown_to_container(&comment.body))
                }

                // AI action badge (if present)
                @if let Some(ref ai_action) = comment.ai_action {
                    div direction=row align-items=center gap=4 margin-top=4 {
                        span
                            padding-x=8
                            padding-y=2
                            background="#ddf4ff"
                            color="#0969da"
                            border-radius=4
                            font-size=12
                            font-weight=600
                        {
                            "🤖 " (&ai_action.provider) "/" (&ai_action.agent)
                        }
                    }
                }

                // AI status (if present)
                @if let Some(ref status) = comment.ai_status {
                    (render_ai_status_container(comment.id, status))
                }

                // Action buttons - Reply and Delete
                div direction=row gap=12 margin-top=4 {
                    (render_local_reply_button(comment.id))
                    (render_local_delete_button(comment.id, comment.id, repo_path))
                }

                // Replies
                (render_thread_replies(comment.id, &comment.replies, repo_path, viewed_reply_ids))

                // Reply form
                (render_local_reply_form(repo_path, comment.id))
            }
        }
    }
    .into()
}

/// Render the replies container for a thread.
///
/// This is a separate container with its own ID so it can be updated via SSE
/// when new replies are added (e.g., AI responses).
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn render_thread_replies(
    thread_id: Uuid,
    replies: &[LocalComment],
    repo_path: &str,
    viewed_reply_ids: &HashSet<Uuid>,
) -> Container {
    let replies_id = local_thread_replies_id(thread_id);

    container! {
        div id=(replies_id) gap=8 {
            @for reply in replies {
                @let is_viewed = viewed_reply_ids.contains(&reply.id);
                (render_local_comment_item(reply, thread_id, repo_path, is_viewed))
            }
        }
    }
    .into()
}

/// Render a single reply comment item with collapse/expand and viewed functionality.
///
/// Root comments are rendered by `render_local_comment_with_reply` which includes
/// the resolve functionality.
///
/// # Arguments
/// * `comment` - The comment to render
/// * `thread_id` - The root thread ID (for targeting the reply form and delete)
/// * `repo_path` - Repository path (for API calls)
/// * `is_viewed` - Whether this reply has been marked as viewed
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_local_comment_item(
    comment: &LocalComment,
    thread_id: Uuid,
    repo_path: &str,
    is_viewed: bool,
) -> Container {
    let time_ago = format_time_ago(comment.created_at);
    let comment_id_str = format!("comment-{}", comment.id);
    let body_id = local_reply_body_id(comment.id);
    let collapse_btn_id = local_reply_collapse_btn_id(comment.id);
    let expand_btn_id = local_reply_expand_btn_id(comment.id);

    // Viewed replies start collapsed
    let is_collapsed = is_viewed;

    // Background color: light blue for viewed, white for unviewed
    let bg_color = if is_viewed { "#f0f6fc" } else { "#ffffff" };

    // Clone IDs for use in fx-click closures
    let body_id_1 = body_id.clone();
    let collapse_id_1 = collapse_btn_id.clone();
    let expand_id_1 = expand_btn_id.clone();
    let body_id_2 = body_id.clone();
    let collapse_id_2 = collapse_btn_id.clone();
    let expand_id_2 = expand_btn_id.clone();

    container! {
        div
            id=(comment_id_str)
            background=(bg_color)
            border="1px solid #d0d7de"
            border-radius=6
            margin-bottom=8
            margin-left=20
        {
            // Header with author, time, collapse/expand, and viewed button
            div
                padding=12
                direction=row
                align-items=center
                gap=8
            {
                // Collapse button (▼) - visible when expanded
                button
                    id=(collapse_btn_id)
                    class="comment-reply-collapse-btn"
                    type=button
                    padding=4
                    cursor=pointer
                    font-size=14
                    color="#57606a"
                    hidden=(is_collapsed)
                    fx-click=fx { element_by_id(body_id_1).no_display(); element_by_id(collapse_id_1).no_display(); element_by_id(expand_id_1).display() }
                {
                    "▼"
                }
                // Expand button (▶) - visible when collapsed
                button
                    id=(expand_btn_id)
                    class="comment-reply-expand-btn"
                    type=button
                    padding=4
                    cursor=pointer
                    font-size=14
                    color="#57606a"
                    hidden=(!is_collapsed)
                    fx-click=fx { element_by_id(body_id_2).display(); element_by_id(expand_id_2).no_display(); element_by_id(collapse_id_2).display() }
                {
                    "▶"
                }

                div
                    width=24
                    height=24
                    border-radius=12
                    background="#d0d7de"
                    align-items=center
                    justify-content=center
                    font-size=12
                    color="#57606a"
                {
                    "👤"
                }
                span font-weight=600 font-size=14 color="#24292f" {
                    (&comment.author.name)
                }
                span font-size=12 color="#57606a" { (time_ago) }

                // Spacer
                div flex=1 {}

                // Viewed button in header
                (render_local_reply_view_button(thread_id, comment.id, is_viewed, repo_path))
            }

            // Collapsible body
            div
                id=(body_id)
                class="comment-reply-body"
                padding=12
                padding-top=0
                gap=8
                hidden=(is_collapsed)
            {
                // Comment body (rendered as markdown for AI responses)
                div color="#24292f" font-size=14 {
                    (markdown_to_container(&comment.body))
                }

                // AI action badge (if present)
                @if let Some(ref ai_action) = comment.ai_action {
                    div direction=row align-items=center gap=4 margin-top=4 {
                        span
                            padding-x=8
                            padding-y=2
                            background="#ddf4ff"
                            color="#0969da"
                            border-radius=4
                            font-size=12
                            font-weight=600
                        {
                            "🤖 " (&ai_action.provider) "/" (&ai_action.agent)
                        }
                    }
                }

                // AI status (if present)
                @if let Some(ref status) = comment.ai_status {
                    (render_ai_status_container(comment.id, status))
                }

                // Action buttons - Reply and Delete
                div direction=row gap=12 margin-top=4 {
                    (render_local_reply_button(thread_id))
                    (render_local_delete_button(thread_id, comment.id, repo_path))
                }
            }
        }
    }
    .into()
}

// =============================================================================
// AI Status UI
// =============================================================================

/// Render the AI status container for a comment.
///
/// This container has a `str_id` that allows `HyperChad` to push partial updates
/// to it via SSE without re-rendering the entire page.
///
/// # Panics
///
/// Panics if the container macro produces an empty iterator (should never happen).
#[must_use]
pub fn render_ai_status_container(comment_id: Uuid, status: &AiExecutionStatus) -> Container {
    container! {
        div id=(ai_status_str_id(comment_id)) padding=8 margin-top=8 {
            (render_ai_status_inner(status))
        }
    }
    .into_iter()
    .next()
    .unwrap()
}

/// Render the inner content of the AI status display.
///
/// This is separated from the container so we can push just the inner content
/// during updates while maintaining the same container `str_id`.
#[must_use]
pub fn render_ai_status_inner(status: &AiExecutionStatus) -> Container {
    match status {
        AiExecutionStatus::Pending => render_status_pending(),
        AiExecutionStatus::Running {
            started_at,
            progress,
        } => render_status_running(started_at, progress),
        AiExecutionStatus::Completed {
            finished_at,
            execution_details,
            ..
        } => render_status_completed(finished_at, execution_details.as_ref()),
        AiExecutionStatus::Failed { finished_at, error } => {
            render_status_failed(finished_at, error)
        }
    }
}

fn render_status_pending() -> Container {
    container! {
        div
            direction=row
            align-items=center
            gap=8
            padding=12
            background="#fff8c5"
            border="1px solid #d4a72c"
            border-radius=6
        {
            span font-size=16 { "⏳" }
            span font-size=14 color="#6e5a00" { "AI execution pending..." }
        }
    }
    .into()
}

fn render_status_running(
    started_at: &chrono::DateTime<chrono::Utc>,
    progress: &[ProgressEntry],
) -> Container {
    let elapsed = chrono::Utc::now()
        .signed_duration_since(*started_at)
        .num_seconds();

    container! {
        div
            padding=12
            background="#ddf4ff"
            border="1px solid #54aeff"
            border-radius=6
            gap=8
        {
            div direction=row align-items=center gap=8 {
                span font-size=16 { "⚙️" }
                span font-size=14 color="#0969da" font-weight=600 {
                    "AI working... (" (elapsed) "s)"
                }
            }
            @if !progress.is_empty() {
                div margin-top=8 gap=4 {
                    @for entry in progress.iter().rev().take(5) {
                        div
                            direction=row
                            gap=8
                            font-size=12
                            color="#57606a"
                            font-family="monospace"
                        {
                            span color="#0969da" { "[" (&entry.tool) "]" }
                            span { (&entry.title) }
                        }
                    }
                }
            }
        }
    }
    .into()
}

fn render_status_completed(
    _finished_at: &chrono::DateTime<chrono::Utc>,
    execution_details: Option<&ExecutionDetails>,
) -> Container {
    container! {
        div
            padding=12
            background="#dafbe1"
            border="1px solid #4ac26b"
            border-radius=6
            gap=8
        {
            div direction=row align-items=center gap=8 {
                span font-size=16 { "✅" }
                span font-size=14 color="#1a7f37" font-weight=600 {
                    "AI execution completed"
                }
            }
            @if let Some(details) = execution_details {
                (render_execution_details(details))
            }
        }
    }
    .into()
}

fn render_status_failed(_finished_at: &chrono::DateTime<chrono::Utc>, error: &str) -> Container {
    container! {
        div
            padding=12
            background="#ffebe9"
            border="1px solid #ff8182"
            border-radius=6
            gap=8
        {
            div direction=row align-items=center gap=8 {
                span font-size=16 { "❌" }
                span font-size=14 color="#cf222e" font-weight=600 {
                    "AI execution failed"
                }
            }
            div
                margin-top=8
                padding=8
                background="#ffffff"
                border-radius=4
                font-family="monospace"
                font-size=12
                color="#cf222e"
            {
                (error)
            }
        }
    }
    .into()
}

/// Render execution details ("How I worked on this" section).
#[must_use]
pub fn render_execution_details(details: &ExecutionDetails) -> Container {
    let cost_str = details
        .cost
        .map_or_else(|| "N/A".to_string(), |c| format!("${c:.4}"));

    container! {
        details margin-top=8 {
            summary
                cursor=pointer
                font-size=12
                color="#57606a"
            {
                "How I worked on this"
            }
            div padding=8 margin-top=8 background="#ffffff" border-radius=4 gap=4 {
                div direction=row gap=8 font-size=12 {
                    span color="#57606a" font-weight=600 { "Model:" }
                    span color="#24292f" { (&details.model_used) }
                }
                div direction=row gap=8 font-size=12 {
                    span color="#57606a" font-weight=600 { "Tokens:" }
                    span color="#24292f" {
                        (details.tokens.input) " in / " (details.tokens.output) " out"
                    }
                }
                div direction=row gap=8 font-size=12 {
                    span color="#57606a" font-weight=600 { "Cost:" }
                    span color="#24292f" { (cost_str) }
                }
                div direction=row gap=8 font-size=12 {
                    span color="#57606a" font-weight=600 { "Duration:" }
                    span color="#24292f" { (details.duration_seconds) "s" }
                }
                @if !details.tools_used.is_empty() {
                    div margin-top=8 gap=4 {
                        span font-size=12 color="#57606a" font-weight=600 { "Tools used:" }
                        ul margin-left=16 gap=2 {
                            @for tool in &details.tools_used {
                                li font-size=12 {
                                    span font-weight=600 color="#0969da" { (&tool.tool) }
                                    span color="#57606a" { ": " (&tool.title) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    .into()
}

// =============================================================================
// AI Action Selector
// =============================================================================

/// Render the AI action selector.
///
/// This is used in the comment form to allow users to select which AI agent
/// to use for processing their comment.
#[must_use]
pub fn render_ai_action_selector(
    field_name: &str,
    _available_agents: &[(&str, &str)], // (value, display_name) - not used yet
) -> Container {
    container! {
        div direction=row align-items=center gap=8 {
            span font-size=14 color="#57606a" { "AI Agent:" }
            input
                type=text
                name=(field_name)
                placeholder="e.g. opencode:code or leave empty"
                padding=8
                border="1px solid #d0d7de"
                border-radius=6
                font-size=14
                flex=1;
        }
    }
    .into()
}

/// Default available agents for the AI action selector.
pub const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("opencode:code", "OpenCode - Code (default)"),
    ("opencode:plan", "OpenCode - Plan (research, no edits)"),
];

// =============================================================================
// Utilities
// =============================================================================

/// Format a timestamp as a human-readable "time ago" string.
fn format_time_ago(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{mins}m ago")
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{hours}h ago")
    } else {
        let days = duration.num_days();
        format!("{days}d ago")
    }
}
