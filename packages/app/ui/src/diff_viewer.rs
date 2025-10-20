use chadreview_pr_models::{DiffFile, DiffHunk, DiffLine, FileStatus, LineType};
use hyperchad::router::Container;
use hyperchad::template::container;

#[must_use]
pub fn render(diffs: &[DiffFile]) -> Container {
    if diffs.is_empty() {
        return container! {
            div padding=20 color="#57606a" {
                "No changes in this pull request."
            }
        }
        .into();
    }

    container! {
        section padding=20 {
            h2 font-size=20 font-weight=600 color="#24292f" margin-bottom=16 {
                "Files changed"
            }
            @for diff_file in diffs {
                (render_file(diff_file))
            }
        }
    }
    .into()
}

fn render_file(file: &DiffFile) -> Container {
    let file_stats = render_file_stats(file);
    let file_header = render_file_header(file, &file_stats);

    container! {
        div margin-bottom=24 border="1px solid #d0d7de" border-radius=6 {
            (file_header)
            @for hunk in &file.hunks {
                (render_hunk(hunk))
            }
        }
    }
    .into()
}

fn render_file_header(file: &DiffFile, file_stats: &Container) -> Container {
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
    .into()
}

fn render_file_stats(file: &DiffFile) -> Container {
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
    .into()
}

fn render_hunk(hunk: &DiffHunk) -> Container {
    container! {
        div {
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
            @for line in &hunk.lines {
                (render_line(line))
            }
        }
    }
    .into()
}

fn render_line(line: &DiffLine) -> Container {
    let (bg_color, prefix, prefix_color) = match line.line_type {
        LineType::Addition => ("#e6ffec", "+", "#1a7f37"),
        LineType::Deletion => ("#ffebe9", "-", "#cf222e"),
        LineType::Context => ("#ffffff", " ", "#57606a"),
    };

    let old_num = render_line_number(line.old_line_number);
    let new_num = render_line_number(line.new_line_number);

    container! {
        div
            direction=row
            background=(bg_color)
            font-family="monospace"
            font-size=12
        {
            div
                width=40
                text-align=end
                padding-y=2
                padding-x=4
                color="#57606a"
                background="#f6f8fa"
                border-right="1px solid #d0d7de"
            {
                (old_num)
            }
            div
                width=40
                text-align=end
                padding-y=2
                padding-x=4
                color="#57606a"
                background="#f6f8fa"
                border-right="1px solid #d0d7de"
            {
                (new_num)
            }
            div
                width=20
                text-align=center
                padding-y=2
                color=(prefix_color)
                font-weight=600
            {
                (prefix)
            }
            div padding-y=2 padding-x=4 flex=1 white-space=preserve {
                (render_highlighted_content(&line.highlighted_html))
            }
        }
    }
    .into()
}

fn render_line_number(num: Option<usize>) -> String {
    num.map_or_else(String::new, |n| n.to_string())
}

fn render_highlighted_content(html: &str) -> Container {
    use hyperchad::template::container;

    container! {
        span {
            (html)
        }
    }
    .into()
}
