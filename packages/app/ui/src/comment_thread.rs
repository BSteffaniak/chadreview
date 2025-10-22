use chadreview_pr_models::{Comment, DiffLine, LineType};
use hyperchad::template::{Containers, container};
use hyperchad::transformer::models::SwapTarget;

#[must_use]
pub fn comment_id(comment: &Comment) -> String {
    format!("comment-{}", comment.id)
}

#[must_use]
pub fn render_comment_thread(
    comment: &Comment,
    depth: usize,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let margin_left = i32::try_from(depth * 20).unwrap_or(0);

    container! {
        div
            margin-left=(margin_left)
            border-left="2px solid #d0d7de"
            padding-left=12
            gap=12
        {
            (render_comment_item(owner, repo, number, comment))
            @for reply in &comment.replies {
                (render_comment_thread(reply, depth + 1, owner, repo, number))
            }
        }
        (render_reply_form(comment, owner, repo, number))
    }
}

#[must_use]
pub fn render_comment_item(owner: &str, repo: &str, number: u64, comment: &Comment) -> Containers {
    let formatted_time = format_timestamp(&comment.created_at);

    container! {
        div
            id=(comment_id(comment))
            padding=12
            background="#ffffff"
            border="1px solid #d0d7de"
            border-radius=6
            max-width=100%
            gap=8
        {
            div direction=row align-items=center gap=8 {
                image
                    width=24
                    height=24
                    border-radius=12
                    background="#d0d7de"
                    src=(comment.author.avatar_url)
                {}
                anchor
                    color="#0969da"
                    font-weight=600
                    font-size=14
                    href=(comment.author.html_url)
                {
                    (comment.author.username)
                }
                span font-size=12 color="#57606a" {
                    (formatted_time)
                }
            }
            div
                id=(format!("comment-{}-body", comment.id))
                color="#24292f"
                font-size=14
                white-space=preserve-wrap
                overflow-wrap=anywhere
            {
                (comment.body)
            }
            (render_edit_form(owner, repo, number, comment))
            div direction=row gap=12 {
                (render_reply_button(comment))
                (render_edit_button(comment))
                (render_delete_button(owner, repo, number, comment))
            }
        }
    }
}

fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%b %d, %Y").to_string()
}

#[must_use]
pub fn comment_form_id(file_path: &str, line: &DiffLine) -> String {
    format!("comment-form-{}-{line}", classify_name(file_path))
}

/// # Panics
///
/// * If cannot find corresponding line number
#[must_use]
pub fn render_create_comment_form(
    owner: &str,
    repo: &str,
    number: u64,
    commit_sha: &str,
    file_path: &str,
    line: &DiffLine,
) -> Containers {
    let form_id = comment_form_id(file_path, line);
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");

    let (side, line) = match line.line_type {
        LineType::Addition => (
            "new",
            line.new_line_number.expect("Missing new line number"),
        ),
        LineType::Deletion => (
            "old",
            line.old_line_number.expect("Missing old line number"),
        ),
        LineType::Context => (
            "new",
            line.new_line_number
                .or(line.old_line_number)
                .expect("Missing line number"),
        ),
    };

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
        {
            div
                padding=12
                background="#ffffff"
                border="1px solid #d0d7de"
                border-radius=6
                direction=column
                gap=8
            {
                input type=hidden name="commit_sha" value=(commit_sha);
                input type=hidden name="path" value=(file_path);
                input type=hidden name="line" value=(line);
                input type=hidden name="side" value=(side);
                input type=hidden name="comment_type" value="line_level_comment";
                textarea name="body" placeholder="Add a comment..." height=80;
                div direction=row gap=8 {
                    button
                        type=submit
                        background="#1a7f37"
                        color="#ffffff"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        font-weight=600
                        cursor=pointer
                    {
                        "Comment"
                    }
                    button
                        type=button
                        background="transparent"
                        color="#57606a"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        cursor=pointer
                        fx-click=fx { element(form_id).no_display() }
                    {
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn render_reply_form(
    parent_comment: &Comment,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let form_id = format!("reply-form-{}", parent_comment.id);
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
        {
            div
                background="#f6f8fa"
                border="1px solid #d0d7de"
                border-radius=6
                padding=12
                direction=column
                gap=8
            {
                input type=hidden name="in_reply_to" value=(parent_comment.id);
                input type=hidden name="comment_type" value="reply";
                textarea name="body" placeholder="Reply..." height=80;
                div direction=row gap=8 {
                    button
                        type=submit
                        background="#1a7f37"
                        color="#ffffff"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        font-weight=600
                        cursor=pointer
                    {
                        "Reply"
                    }
                    button
                        type=button
                        background="transparent"
                        color="#57606a"
                        padding-x=16
                        padding-y=8
                        border-radius=6
                        cursor=pointer
                        fx-click=fx { element(form_id).no_display() }
                    {
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn render_edit_form(owner: &str, repo: &str, number: u64, comment: &Comment) -> Containers {
    let form_id = format!("edit-form-{}", comment.id);
    let target_id = format!("comment-{}", comment.id);
    let api_url = format!(
        "/api/comment/update?owner={owner}&repo={repo}&number={number}&id={}",
        comment.id
    );

    container! {
        form
            id=(form_id)
            hidden
            hx-put=(api_url)
            hx-swap=(SwapTarget::Id(target_id))
            padding=12
            background="#ffffff"
            border="1px solid #d0d7de"
            border-radius=6
            direction=column
            gap=8
        {
            textarea name="body" height=80 { (comment.body) }
            div direction=row gap=8 {
                button
                    type=submit
                    background="#1a7f37"
                    color="#ffffff"
                    padding-x=16
                    padding-y=8
                    border-radius=6
                    font-weight=600
                    cursor=pointer
                {
                    "Save"
                }
                button
                    type=button
                    background="transparent"
                    color="#57606a"
                    padding-x=16
                    padding-y=8
                    border-radius=6
                    cursor=pointer
                    fx-click=fx { element(form_id).no_display() }
                {
                    "Cancel"
                }
            }
        }
    }
}

#[must_use]
pub fn render_reply_button(comment: &Comment) -> Containers {
    let form_id = format!("reply-form-{}", comment.id);

    container! {
        button
            type=button
            background="transparent"
            color="#0969da"
            padding-x=8
            padding-y=4
            cursor=pointer
            font-size=12
            fx-click=fx { element(form_id).display() }
        {
            "Reply"
        }
    }
}

#[must_use]
pub fn render_edit_button(comment: &Comment) -> Containers {
    let button_id = format!("edit-form-{}", comment.id);
    let body_id = format!("comment-{}-body", comment.id);

    container! {
        button
            type=button
            background="transparent"
            color="#0969da"
            padding-x=8
            padding-y=4
            cursor=pointer
            font-size=12
            fx-click=fx { element(button_id).display(); element(body_id).no_display() }
        {
            "Edit"
        }
    }
}

#[must_use]
pub fn render_delete_button(owner: &str, repo: &str, number: u64, comment: &Comment) -> Containers {
    let target_id = format!("comment-{}", comment.id);
    let api_url = format!(
        "/api/comment/delete?owner={owner}&repo={repo}&number={number}&id={}",
        comment.id
    );

    container! {
        form
            hx-delete=(api_url)
            hx-swap=(SwapTarget::Id(target_id))
            direction=row
        {
            button
                type=submit
                background="transparent"
                color="#cf222e"
                padding-x=8
                padding-y=4
                cursor=pointer
                font-size=12
            {
                "Delete"
            }
        }
    }
}

#[must_use]
pub fn add_comment_button_id(file_path: &str, line: &DiffLine) -> String {
    format!("add-comment-{}-{line}", classify_name(file_path))
}

#[must_use]
pub fn classify_name<T: AsRef<str>>(class: T) -> String {
    let class = class.as_ref();
    class
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
}

#[must_use]
pub fn render_add_comment_button(file_path: &str, line: &DiffLine) -> Containers {
    let button_id = add_comment_button_id(file_path, line);
    let form_id = comment_form_id(file_path, line);
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
            background=#1f6feb
            border-radius=3
            color=white
            cursor=pointer
            font-size=12
            opacity=0.6
            user-select=none
            fx-click=fx { element(form_id).display() }
        {
            "+"
        }
    }
}
