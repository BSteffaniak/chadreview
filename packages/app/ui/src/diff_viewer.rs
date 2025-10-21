use chadreview_pr_models::{
    Comment, CommentType, DiffFile, DiffHunk, DiffLine, FileStatus, LineType,
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
        section padding=20 {
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
    let file_stats = render_file_stats(file);
    let file_header = render_file_header(file, &file_stats);

    container! {
        div margin-bottom=24 border="1px solid #d0d7de" border-radius=6 {
            (file_header)
            (render_file_level_comments(comments, &file.filename, owner, repo, number))
            div direction=row {
                div flex-shrink=1 flex-grow=0 {
                    @for hunk in &file.hunks {
                        (render_hunk_header_for_line_column(hunk))
                        @for line in &hunk.lines {
                            (render_line_numbers_cell(line))
                        }
                    }
                }
                div overflow-x=auto flex=1 direction=row {
                    div direction=column {
                        @for hunk in &file.hunks {
                            (render_hunk_header_prefix_column(hunk))
                            @for line in &hunk.lines {
                                (render_prefix_cell(line))
                            }
                        }
                    }
                    div direction=column flex=1 {
                        @for hunk in &file.hunks {
                            (render_hunk_header_for_code_column(hunk))
                            @for line in &hunk.lines {
                                (render_code_content_cell(line))
                                @if let Some(new_line_num) = line.new_line_number {
                                    (render_line_comments(comments, &file.filename, new_line_num, owner, repo, number))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_file_header(file: &DiffFile, file_stats: &Containers) -> Containers {
    let (status_text, status_color) = match file.status {
        FileStatus::Added => ("Added", "#1a7f37"),
        FileStatus::Modified => ("Modified", "#0969da"),
        FileStatus::Deleted => ("Deleted", "#cf222e"),
        FileStatus::Renamed => ("Renamed", "#8250df"),
    };

    container! {
        div
            padding=12
            background="#f6f8fa"
            border-bottom="1px solid #d0d7de"
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
            (file_stats)
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

fn render_hunk_header_for_line_column(_hunk: &DiffHunk) -> Containers {
    container! {
        div
            padding-y=4
            padding-x=8
            background="#f6f8fa"
            border-top="1px solid #d0d7de"
            border-bottom="1px solid #d0d7de"
            font-family="monospace"
            font-size=12
            color="#57606a"
        {
            "@@"
        }
    }
}

fn render_hunk_header_prefix_column(_hunk: &DiffHunk) -> Containers {
    container! {
        div
            width=20
            padding-y=4
            font-size=12
            background="#f6f8fa"
            border-top="1px solid #d0d7de"
            border-bottom="1px solid #d0d7de"
            white-space=preserve
        {
            " "
        }
    }
}

fn render_hunk_header_for_code_column(hunk: &DiffHunk) -> Containers {
    container! {
        div
            padding-y=4
            padding-x=12
            background="#f6f8fa"
            border-top="1px solid #d0d7de"
            border-bottom="1px solid #d0d7de"
            font-family="monospace"
            font-size=12
            color="#57606a"
        {
            "@@ -" (hunk.old_start) "," (hunk.old_lines) " +" (hunk.new_start) "," (hunk.new_lines) " @@"
        }
    }
}

fn render_line_numbers_cell(line: &DiffLine) -> Containers {
    let bg_color = match line.line_type {
        LineType::Addition => "#e6ffec",
        LineType::Deletion => "#ffebe9",
        LineType::Context => "#ffffff",
    };

    let old_num = render_line_number(line.old_line_number);
    let new_num = render_line_number(line.new_line_number);

    container! {
        div
            direction=row
            background=(bg_color)
            font-family="monospace"
            font-size=12
            min-height=24
        {
            div
                width=40
                text-align=end
                padding-x=4
                color="#57606a"
                background="#f6f8fa"
                border-bottom="1px solid #d0d7de"
                align-items=end
                justify-content=center
            {
                (old_num)
            }
            div
                width=40
                text-align=end
                padding-x=4
                color="#57606a"
                background="#f6f8fa"
                border-left="1px solid #d0d7de"
                border-bottom="1px solid #d0d7de"
                border-right="1px solid #d0d7de"
                align-items=end
                justify-content=center
            {
                (new_num)
            }
        }
    }
}

fn render_prefix_cell(line: &DiffLine) -> Containers {
    let (bg_color, prefix, prefix_color) = match line.line_type {
        LineType::Addition => ("#e6ffec", "+", "#1a7f37"),
        LineType::Deletion => ("#ffebe9", "-", "#cf222e"),
        LineType::Context => ("#ffffff", " ", "#57606a"),
    };

    container! {
        div
            width=20
            text-align=center
            background=(bg_color)
            color=(prefix_color)
            font-weight=600
            font-family="monospace"
            font-size=12
            min-height=24
            justify-content=center
        {
            (prefix)
        }
    }
}

fn render_code_content_cell(line: &DiffLine) -> Containers {
    let bg_color = match line.line_type {
        LineType::Addition => "#e6ffec",
        LineType::Deletion => "#ffebe9",
        LineType::Context => "#ffffff",
    };

    container! {
        div
            background=(bg_color)
            padding-x=4
            font-family="monospace"
            font-size=12
            min-height=24
            white-space=preserve
            direction=row
            align-items=center
        {
            (line.highlighted_html.as_str())
        }
    }
}

fn render_line_number(num: Option<usize>) -> String {
    num.map_or_else(String::new, |n| n.to_string())
}

fn render_line_comments(
    comments: &[Comment],
    file_path: &str,
    line: usize,
    owner: &str,
    repo: &str,
    number: u64,
) -> Containers {
    let mut line_comments = comments
        .iter()
        .filter(|c| matches!(&c.comment_type, CommentType::LineLevelComment { path, line: l } if path == file_path && *l == line))
        .peekable();

    let target_id = format!("line-{line}-comments");

    container! {
        (comment_thread::render_add_comment_button(file_path, line))
        @if line_comments.peek().is_some() {
            div id=(target_id) direction=column gap=8 margin-top=8 position=relative {
                @for comment in line_comments {
                    (comment_thread::render_comment_thread(comment, 0, owner, repo, number))
                }
                (comment_thread::render_create_comment_form(owner, repo, number, file_path, line))
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
    let file_comments: Vec<&Comment> = comments
        .iter()
        .filter(|c| matches!(&c.comment_type, CommentType::FileLevelComment { path } if path == file_path))
        .collect();

    if file_comments.is_empty() {
        return container! { div {} };
    }

    container! {
        div direction=column gap=12 padding=12 background="#f6f8fa" margin-bottom=12 {
            @for comment in file_comments {
                (comment_thread::render_comment_thread(comment, 0, owner, repo, number))
            }
        }
    }
}
