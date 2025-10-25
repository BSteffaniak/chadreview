#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use chadreview_pr_models::{Comment, CommentType};
use hyperchad::template::{Containers, container};
use hyperchad::transformer::models::Selector;

use crate::comment_thread;

#[must_use]
pub fn render_general_comments_section(
    comments: &[Comment],
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let general_comments: Vec<&Comment> = comments
        .iter()
        .filter(|c| matches!(c.comment_type, CommentType::General))
        .collect();

    let count = general_comments.len();

    container! {
        div
            border="1, #d0d7de"
            border-radius=6
            padding=16
            margin-top=16
            margin-bottom=16
            background=#ffffff
        {
            details open {
                summary
                    cursor=pointer
                    font-weight=600
                    font-size=16
                    padding=8
                    user-select=none
                {
                    (format!("General Comments ({})", count))
                }
                div
                    id="general-comments-list"
                    direction=column
                    gap=16
                    margin-top=16
                {
                    @for comment in general_comments {
                        (comment_thread::render_comment_thread(
                            0,
                            comment,
                            0,
                            owner,
                            repo,
                            number,
                        ))
                    }
                }
                (render_create_general_comment_form(owner, repo, number))
            }
        }
    }
}

#[must_use]
pub fn render_create_general_comment_form(owner: &str, repo: &str, number: u64) -> Containers {
    let api_url = format!("/api/pr/comment?owner={owner}&repo={repo}&number={number}");

    container! {
        form
            hx-post=(api_url)
            hx-swap=beforeend
            hx-target=(Selector::Id(String::from("general-comments-list")))
        {
            div
                margin-top=16
                background=#f6f8fa
                border="1, #d0d7de"
                border-radius=6
                padding=12
                direction=column
                gap=8
            {
                input type=hidden name="comment_type" value="general";
                textarea
                    name="body"
                    placeholder="Add a general comment..."
                    height=80
                    border="1, #d0d7de"
                    border-radius=6
                    padding=8
                    font-size=14;
                button
                    type=submit
                    background=#1a7f37
                    color=#ffffff
                    padding-x=16
                    padding-y=8
                    border-radius=6
                    font-size=14
                    font-weight=600
                    cursor=pointer
                {
                    "Comment"
                }
            }
        }
    }
}
