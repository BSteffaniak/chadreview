use chadreview_pr_models::{
    Comment, CommentType, DiffFile, DiffHunk, DiffLine, FileStatus, LineType, comment::LineNumber,
};
use hyperchad::template::{Containers, container};

use crate::comment_thread;

#[must_use]
pub fn render(
    diffs: &[DiffFile],
    comments: &[Comment],
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    if diffs.is_empty() {
        return container! {
            div padding=20 color="#57606a" {
                "No changes in this pull request."
            }
        };
    }

    container! {
        section padding=20 gap=24 {
            h2 font-size=20 font-weight=600 color="#24292f" margin-bottom=16 {
                "Files changed"
            }
            @for diff_file in diffs {
                (render_file(diff_file, comments, owner, repo, number))
            }
        }
    }
}

fn render_file(
    file: &DiffFile,
    comments: &[Comment],
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    container! {
        div border="1px solid #d0d7de" border-radius=6 {
            table width=100% {
                (render_file_header(file))
                (render_file_level_comments(comments, &file.filename, owner, repo, number))
                @for hunk in &file.hunks {
                    (render_hunk_header_row(hunk))
                    tbody font-family="monospace" font-size=12 {
                        @for line in &hunk.lines {
                            (render_line_row(&file.filename, line))
                            (render_line_comments(comments, &file.filename, line, owner, repo, number))
                        }
                    }
                }
            }
        }
    }
}

fn render_file_header(file: &DiffFile) -> Containers {
    let (status_text, status_color) = match file.status {
        FileStatus::Added => ("Added", "#1a7f37"),
        FileStatus::Modified => ("Modified", "#0969da"),
        FileStatus::Deleted => ("Deleted", "#cf222e"),
        FileStatus::Renamed => ("Renamed", "#8250df"),
    };

    container! {
        thead {
            th columns=3 {
                div
                    padding=12
                    background="#f6f8fa"
                    direction=row
                    align-items=center
                    justify-content=space-between
                {
                    div direction=row align-items=center gap=12 {
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
                        span font-family="monospace" font-size=14 font-weight=600 color="#24292f" {
                            (file.filename.as_str())
                        }
                    }
                    (render_file_stats(file))
                }
            }
        }
    }
}

fn render_line_row(file_path: &str, line: &DiffLine) -> Containers {
    let bg_color = match line.line_type {
        LineType::Addition => "#e6ffec",
        LineType::Deletion => "#ffebe9",
        LineType::Context => "#ffffff",
    };

    let add_comment_button_id = comment_thread::add_comment_button_id(file_path, line);

    container! {
        tr {
            (render_line_numbers_inline(line))

            td {
                div
                    direction=row
                    position=relative
                    fx-hover=fx { element(add_comment_button_id).display() }
                {
                    div
                        width=20
                        background=(bg_color)
                        padding-y=4
                        color=#57606a
                        user-select=none
                        justify-content=center
                        align-items=center
                    {
                        (render_diff_marker_inline(line))
                    }

                    (comment_thread::render_add_comment_button(file_path, line))

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
                        {
                            (line.highlighted_html.as_str())
                        }
                    }
                }
            }
        }
    }
}

fn render_line_numbers_inline(line: &DiffLine) -> Containers {
    container! {
        td
            background="#f6f8fa"
            border-right="1px solid #d0d7de"
            padding-y=4
            padding-x=8
            font-size=12
            text-align=end
            color="#57606a"
            user-select=none
            width=1%
        {
            @if let Some(old) = line.old_line_number {
                (old)
            }
        }
        td
            background="#f6f8fa"
            border-right="1px solid #d0d7de"
            padding-y=4
            padding-x=8
            font-size=12
            text-align=end
            color="#57606a"
            user-select=none
            width=1%
        {
            @if let Some(new) = line.new_line_number {
                (new)
            }
        }
    }
}

const fn render_diff_marker_inline(line: &DiffLine) -> &'static str {
    match line.line_type {
        LineType::Addition => "+",
        LineType::Deletion => "-",
        LineType::Context => " ",
    }
}

fn render_hunk_header_row(hunk: &DiffHunk) -> Containers {
    container! {
        thead {
            tr
                background="#f6f8fa"
                border-top="1px solid #d0d7de"
                border-bottom="1px solid #d0d7de"
                padding-y=4
            {
                th
                    padding-x=8
                    font-family="monospace"
                    font-size=12
                    color="#57606a"
                    user-select=none
                    columns=2
                    width=1%
                {
                    div min-height=24 { "..." }
                }
                th
                    padding-x=12
                    font-family="monospace"
                    font-size=12
                    color="#57606a"
                    user-select=none
                    text-align=start
                {
                    (format!("@@ -{},{} +{},{} @@",
                        hunk.old_start, hunk.old_lines,
                        hunk.new_start, hunk.new_lines).as_str())
                }
            }
        }
    }
}
fn render_file_stats(file: &DiffFile) -> Containers {
    container! {
        div direction=row align-items=center gap=8 font-size=13 {
            span color="#1a7f37" font-weight=600 {
                "+" (file.additions)
            }
            span color="#cf222e" font-weight=600 {
                "-" (file.deletions)
            }
        }
    }
}

fn render_line_comments(
    comments: &[Comment],
    file_path: &str,
    line: &DiffLine,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let mut line_comments = comments
        .iter()
        .filter(|c| {
            matches!(
                &c.comment_type,
                CommentType::LineLevelComment {
                    path,
                    line: l,
                } if path == file_path
                    && (line.new_line_number.is_some_and(|n| *l == LineNumber::New(n))
                        || line.old_line_number.is_some_and(|n| *l == LineNumber::Old(n)))
            )
        })
        .peekable();

    if line_comments.peek().is_none() {
        return vec![];
    }

    let target_id = format!("line-{line}-comments");

    container! {
        tr {
            td columns=3 {
                div padding=8 direction=column gap=8 {
                    div id=(target_id) direction=column gap=8 {
                        @for comment in line_comments {
                            (comment_thread::render_comment_thread(comment, 0, owner, repo, number))
                        }
                    }
                    (comment_thread::render_create_comment_form(owner, repo, number, file_path, line))
                }
            }
        }
    }
}

fn render_file_level_comments(
    comments: &[Comment],
    file_path: &str,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let mut file_comments = comments
        .iter()
        .filter(|c| matches!(&c.comment_type, CommentType::FileLevelComment { path } if path == file_path))
        .peekable();

    if file_comments.peek().is_none() {
        return vec![];
    }

    container! {
        tbody {
            tr {
                td {
                    div direction=column gap=12 padding=12 background="#f6f8fa" margin-bottom=12 {
                        @for comment in file_comments {
                            (comment_thread::render_comment_thread(comment, 0, owner, repo, number))
                        }
                    }
                }
            }
        }
    }
}
