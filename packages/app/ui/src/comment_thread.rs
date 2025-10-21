use chadreview_pr_models::Comment;
use hyperchad::template::{Containers, container};
use hyperchad::transformer::models::SwapTarget;

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
        {
            (render_comment_item(comment, owner, repo, number))
            @for reply in &comment.replies {
                (render_comment_thread(reply, depth + 1, owner, repo, number))
            }
        }
    }
}

#[must_use]
pub fn render_comment_item(
    comment: &Comment,
    _owner: &str,
    _repo: &str,
    _number: u64,
) -> Containers {
    let formatted_time = format_timestamp(&comment.created_at);

    container! {
        div
            margin-bottom=12
            padding=12
            background="#ffffff"
            border="1px solid #d0d7de"
            border-radius=6
            max-width=100%
        {
            div direction=row align-items=center gap=8 margin-bottom=8 {
                image
                    width=24
                    height=24
                    border-radius=12
                    background="#d0d7de"
                    src=(comment.author.avatar_url.as_str())
                {}
                anchor
                    color="#0969da"
                    font-weight=600
                    font-size=14
                    href=(comment.author.html_url.as_str())
                {
                    (comment.author.username.as_str())
                }
                span font-size=12 color="#57606a" {
                    (formatted_time.as_str())
                }
            }
            div
                id=(format!("comment-{}-body", comment.id))
                color="#24292f"
                font-size=14
                margin-bottom=8
                white-space=preserve-wrap
            {
                (comment.body.as_str())
            }
            div direction=row gap=12 {
                (render_reply_button(comment))
                (render_edit_button(comment))
                (render_delete_button(comment))
            }
        }
    }
}

fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%b %d, %Y").to_string()
}

#[must_use]
pub fn render_create_comment_form(
    owner: &str,
    repo: &str,
    number: u64,
    file_path: &str,
    line: usize,
) -> Containers {
    let form_id = format!("comment-form-{file_path}-{line}");
    let target_id = format!("line-{line}-comments");
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");
    let line_str = line.to_string();

    container! {
        form
            id=(form_id.as_str())
            hidden
            hx-post=(api_url.as_str())
            hx-swap=(SwapTarget::Id(target_id))
            padding=12
            background="#ffffff"
            border="1px solid #d0d7de"
            border-radius=6
            margin-top=8
            direction=column
            gap=8
        {
            input type=hidden name="path" value=(file_path);
            input type=hidden name="line" value=(line_str.as_str());
            input type=hidden name="comment_type" value="LineLevelComment";
            textarea name="body" placeholder="Add a comment..." height=80 width=100%;
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
                    fx-click=fx { hide(form_id) }
                {
                    "Cancel"
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
    let target_id = format!("comment-{}-replies", parent_comment.id);
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");
    let parent_id_str = parent_comment.id.to_string();

    container! {
        form
            id=(form_id.as_str())
            hidden
            hx-post=(api_url.as_str())
            hx-swap=(SwapTarget::Id(target_id))
            padding=12
            background="#f6f8fa"
            border="1px solid #d0d7de"
            border-radius=6
            margin-top=8
            direction=column
            gap=8
        {
            input type=hidden name="in_reply_to" value=(parent_id_str.as_str());
            textarea name="body" placeholder="Reply..." height=80 width=100%;
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
                    fx-click=fx { hide(form_id) }
                {
                    "Cancel"
                }
            }
        }
    }
}

#[must_use]
pub fn render_edit_form(comment: &Comment) -> Containers {
    let form_id = format!("edit-form-{}", comment.id);
    let target_id = format!("comment-{}", comment.id);
    let api_url = format!("/api/comment/update?id={}", comment.id);

    container! {
        form
            id=(form_id.as_str())
            hidden
            hx-put=(api_url.as_str())
            hx-swap=(SwapTarget::Id(target_id))
            padding=12
            background="#ffffff"
            border="1px solid #d0d7de"
            border-radius=6
            margin-top=8
            direction=column
            gap=8
        {
            textarea name="body" height=80 width=100% { (comment.body.as_str()) }
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
                    fx-click=fx { hide(form_id) }
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
            fx-click=fx { element(form_id).toggle_visibility() }
        {
            "Reply"
        }
    }
}

#[must_use]
pub fn render_edit_button(comment: &Comment) -> Containers {
    let form_id = format!("edit-form-{}", comment.id);
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
            fx-click=fx { element(form_id).toggle_visibility(); element(body_id).hide() }
        {
            "Edit"
        }
    }
}

#[must_use]
pub fn render_delete_button(comment: &Comment) -> Containers {
    let target_id = format!("comment-{}", comment.id);
    let api_url = format!("/api/comment/delete?id={}", comment.id);

    container! {
        form
            hx-delete=(api_url.as_str())
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
pub fn render_add_comment_button(file_path: &str, line: usize) -> Containers {
    let form_id = format!("comment-form-{file_path}-{line}");

    container! {
        button
            type=button
            position=absolute
            left=0
            top=0
            background="transparent"
            color="#0969da"
            padding-x=4
            padding-y=2
            cursor=pointer
            font-size=12
            opacity=0.6
            fx-click=fx { element(form_id).toggle_visibility() }
        {
            "+"
        }
    }
}
