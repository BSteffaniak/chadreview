use chadreview_pr_models::comment::LineNumber;
use chadreview_pr_models::{Comment, CommentType};
use hyperchad::markdown::markdown_to_container;
use hyperchad::template::{Containers, container};
use hyperchad::transformer::models::Selector;

use crate::diff_viewer::comment_thread_container_id;

#[must_use]
pub fn comment_id(comment_id: u64) -> String {
    format!("comment-{comment_id}")
}

#[must_use]
pub fn comment_class(comment_id: u64) -> String {
    format!("comment-{comment_id}")
}

#[must_use]
pub fn comment_thread_id(comment_id: u64) -> String {
    format!("comment-thread-{comment_id}")
}

#[must_use]
pub fn render_comment_thread(
    root_comment_id: u64,
    comment: &Comment,
    depth: usize,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let margin_left = i32::try_from(depth * 20).unwrap_or(0);

    container! {
        div
            id=(comment_thread_id(comment.id))
            class=(comment_class(comment.id))
            margin-left=(margin_left)
            border-left="2, #d0d7de"
            padding-left=12
            gap=12
        {
            (render_comment_item(comment, root_comment_id == comment.id, owner, repo, number))
            (render_reply_form(root_comment_id, comment.id, owner, repo, number))
            @for reply in &comment.replies {
                (render_comment_thread(root_comment_id, reply, depth + 1, owner, repo, number))
            }
        }
    }
}

#[must_use]
pub fn render_comment_item(
    comment: &Comment,
    root: bool,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let formatted_time = format_timestamp(&comment.created_at);

    container! {
        div
            id=(comment_id(comment.id))
            class=(comment_class(comment.id))
            padding=12
            background="#ffffff"
            border="1, #d0d7de"
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
            {
                (markdown_to_container(&comment.body))
            }
            (render_edit_form(comment, root, owner, repo, number))
            div direction=row gap=12 {
                (render_reply_button(comment))
                (render_edit_button(comment))
                (render_delete_button(comment, root, owner, repo, number))
            }
        }
    }
}

fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%b %d, %Y").to_string()
}

#[must_use]
pub fn comment_form_id(file_path: &str, line: LineNumber) -> String {
    format!("comment-form-{}-{line}", classify_name(file_path))
}

#[must_use]
pub fn render_create_comment_form(
    owner: &str,
    repo: &str,
    number: u64,
    commit_sha: &str,
    file_path: &str,
    line: LineNumber,
) -> Containers {
    let form_id = comment_form_id(file_path, line);
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");

    let (side, line) = match line {
        LineNumber::New { line } => ("new", line),
        LineNumber::Old { line } => ("old", line),
    };

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
            hx-swap="beforebegin"
            fx-http-success=fx { no_display_self() }
        {
            div
                padding=12
                background="#ffffff"
                border="1, #d0d7de"
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
    root_comment_id: u64,
    comment_id: u64,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let form_id = reply_form_id(comment_id);
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");

    container! {
        form
            id=(form_id)
            hidden
            hx-post=(api_url)
            hx-swap=beforeend
            hx-target=(Selector::Id(comment_thread_id(root_comment_id)))
            fx-http-success=fx { no_display_self() }
        {
            div
                background=#f6f8fa
                border="1, #d0d7de"
                border-radius=6
                padding=12
                direction=column
                gap=8
            {
                input type=hidden name="root_comment_id" value=(root_comment_id);
                input type=hidden name="in_reply_to" value=(comment_id);
                input type=hidden name="comment_type" value="reply";
                textarea name="body" placeholder="Reply..." height=80;
                div direction=row gap=8 {
                    button
                        type=submit
                        background=#1a7f37
                        color=#ffffff
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
                        color=#57606a
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
pub fn render_edit_form(
    comment: &Comment,
    root: bool,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let form_id = format!("edit-form-{}", comment.id);
    let target_id = format!("comment-{}", comment.id);
    let api_url = format!(
        "/api/comment/update?owner={owner}&repo={repo}&number={number}&id={comment_id}&root={root}",
        comment_id = comment.id
    );

    container! {
        form
            id=(form_id)
            hidden
            hx-put=(api_url)
            hx-target=(Selector::Id(target_id))
            padding=12
            background="#ffffff"
            border="1, #d0d7de"
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

fn reply_form_id(comment_id: u64) -> String {
    format!("reply-form-{comment_id}")
}

#[must_use]
pub fn render_reply_button(comment: &Comment) -> Containers {
    let form_id = reply_form_id(comment.id);

    container! {
        button
            type=button
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
pub fn render_delete_button(
    comment: &Comment,
    root: bool,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    use std::fmt::Write;

    let mut api_url = format!(
        "/api/comment/delete?\
        id={id}&\
        owner={owner}&\
        repo={repo}&\
        number={number}&\
        root={root}",
        owner = urlencoding::encode(owner),
        repo = urlencoding::encode(repo),
        id = comment.id
    );
    match &comment.comment_type {
        CommentType::LineLevelComment {
            commit_sha,
            path,
            line,
        } => write!(
            api_url,
            "&path={}&line={line}&commit_sha={commit_sha}",
            urlencoding::encode(path)
        )
        .unwrap(),
        CommentType::FileLevelComment { path } => {
            write!(api_url, "&path={}", urlencoding::encode(path)).unwrap();
        }
        CommentType::General | CommentType::Reply { .. } => {}
    }
    let target = if root {
        Selector::Id(comment_thread_container_id(comment.id))
    } else {
        Selector::Id(comment_id(comment.id))
    };

    container! {
        form
            hx-delete=(api_url)
            hx-target=(target)
        {
            button
                type=submit
                color=#cf222e
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
pub fn add_comment_button_id(file_path: &str, line: LineNumber) -> String {
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
pub fn render_add_comment_button(file_path: &str, line: LineNumber) -> Containers {
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
