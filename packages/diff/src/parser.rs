//! Unified diff format parser with syntax highlighting support.
//!
//! This module parses standard unified diff format (as produced by `git diff`)
//! into structured `DiffFile`, `DiffHunk`, and `DiffLine` types.

use chadreview_pr_models::diff::{DiffFile, DiffHunk, DiffLine, FileStatus, LineType};
use chadreview_syntax::SyntaxHighlighter;
use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;
use syntect::highlighting::Style;

/// Regex for parsing unified diff hunk headers.
/// Format: `@@ -old_start,old_lines +new_start,new_lines @@`
static HUNK_HEADER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@@ -(\d+),?(\d*) \+(\d+),?(\d*) @@").unwrap());

/// Parse a unified diff into a structured `DiffFile`.
///
/// # Arguments
///
/// * `filename` - The name of the file being diffed (used for syntax detection)
/// * `status` - The file status (Added, Modified, Deleted, Renamed)
/// * `additions` - Count of added lines
/// * `deletions` - Count of deleted lines
/// * `diff_text` - The unified diff text (hunk headers and content)
/// * `highlighter` - Syntax highlighter for code coloring
///
/// # Returns
///
/// A `DiffFile` containing all parsed hunks with syntax-highlighted content.
///
/// # Errors
///
/// Returns an error if the diff cannot be parsed or highlighting fails.
pub fn parse_unified_diff(
    filename: &str,
    status: FileStatus,
    additions: u64,
    deletions: u64,
    diff_text: &str,
    highlighter: &SyntaxHighlighter,
) -> Result<DiffFile, String> {
    let mut hunks = Vec::new();
    let lines: Vec<&str> = diff_text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].starts_with("@@") {
            let hunk = parse_hunk(&lines, &mut i, filename, highlighter)?;
            hunks.push(hunk);
        } else {
            i += 1;
        }
    }

    Ok(DiffFile {
        filename: filename.to_string(),
        status,
        additions,
        deletions,
        hunks,
    })
}

/// Parse a single hunk from the diff.
fn parse_hunk(
    lines: &[&str],
    i: &mut usize,
    filename: &str,
    highlighter: &SyntaxHighlighter,
) -> Result<DiffHunk, String> {
    let header = lines[*i];
    let captures = HUNK_HEADER_REGEX
        .captures(header)
        .ok_or_else(|| format!("Invalid hunk header: {header}"))?;

    let old_start = captures[1].parse::<u64>().unwrap();
    let old_lines = if captures[2].is_empty() {
        1
    } else {
        captures[2].parse::<u64>().unwrap()
    };
    let new_start = captures[3].parse::<u64>().unwrap();
    let new_lines = if captures[4].is_empty() {
        1
    } else {
        captures[4].parse::<u64>().unwrap()
    };

    *i += 1;

    let mut hunk_lines = Vec::new();
    let mut old_line_num = old_start;
    let mut new_line_num = new_start;

    while *i < lines.len() && !lines[*i].starts_with("@@") {
        let line = lines[*i];
        if line.starts_with("diff --git") || line.starts_with("---") || line.starts_with("+++") {
            break;
        }

        let (line_type, old_line_number, new_line_number, content) = match line.chars().next() {
            Some('+') => {
                let content = &line[1..];
                let num = new_line_num;
                new_line_num += 1;
                (LineType::Addition, None, Some(num), content)
            }
            Some('-') => {
                let content = &line[1..];
                let num = old_line_num;
                old_line_num += 1;
                (LineType::Deletion, Some(num), None, content)
            }
            Some(' ') => {
                let content = &line[1..];
                let old_num = old_line_num;
                let new_num = new_line_num;
                old_line_num += 1;
                new_line_num += 1;
                (LineType::Context, Some(old_num), Some(new_num), content)
            }
            _ => {
                *i += 1;
                continue;
            }
        };

        let highlighted_html = highlight_to_html(highlighter, filename, content)?;

        hunk_lines.push(DiffLine {
            line_type,
            old_line_number,
            new_line_number,
            content: content.to_string(),
            highlighted_html,
        });

        *i += 1;
    }

    Ok(DiffHunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        lines: hunk_lines,
    })
}

/// Convert syntax-highlighted content to HTML.
fn highlight_to_html(
    highlighter: &SyntaxHighlighter,
    filename: &str,
    content: &str,
) -> Result<String, String> {
    let ranges = highlighter.highlight_line(filename, content)?;
    Ok(styled_to_html(&ranges))
}

/// Convert syntect style ranges to HTML with inline styles.
fn styled_to_html(ranges: &[(Style, String)]) -> String {
    let mut html = String::new();
    for (style, text) in ranges {
        let fg = style.foreground;
        write!(
            &mut html,
            r#"<span style="color:#{:02x}{:02x}{:02x}">{}</span>"#,
            fg.r,
            fg.g,
            fg.b,
            html_escape(text)
        )
        .unwrap();
    }
    html
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Count additions and deletions in a patch string.
///
/// This is useful when the counts aren't provided separately.
#[must_use]
pub fn count_additions_deletions(patch: &str) -> (u64, u64) {
    let mut additions = 0u64;
    let mut deletions = 0u64;
    for line in patch.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }
    (additions, deletions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header() {
        let header = "@@ -10,5 +12,7 @@ fn main() {";
        let captures = HUNK_HEADER_REGEX.captures(header).unwrap();
        assert_eq!(&captures[1], "10");
        assert_eq!(&captures[2], "5");
        assert_eq!(&captures[3], "12");
        assert_eq!(&captures[4], "7");
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let header = "@@ -10 +12 @@ fn main() {";
        let captures = HUNK_HEADER_REGEX.captures(header).unwrap();
        assert_eq!(&captures[1], "10");
        assert_eq!(&captures[2], "");
        assert_eq!(&captures[3], "12");
        assert_eq!(&captures[4], "");
    }

    #[test]
    fn test_parse_simple_diff() {
        let diff_text = r#"@@ -1,4 +1,4 @@
 fn main() {
-    println!("Hello");
+    println!("World");
 }"#;
        let highlighter = SyntaxHighlighter::new();
        let result = parse_unified_diff(
            "test.rs",
            FileStatus::Modified,
            1,
            1,
            diff_text,
            &highlighter,
        );
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].lines.len(), 4);
        assert_eq!(diff.hunks[0].lines[0].line_type, LineType::Context);
        assert_eq!(diff.hunks[0].lines[1].line_type, LineType::Deletion);
        assert_eq!(diff.hunks[0].lines[2].line_type, LineType::Addition);
        assert_eq!(diff.hunks[0].lines[3].line_type, LineType::Context);
    }

    #[test]
    fn test_parse_multiple_hunks() {
        let diff_text = r"@@ -1,3 +1,3 @@
 line1
-old2
+new2
 line3
@@ -10,3 +10,3 @@
 line10
-old11
+new11
 line12";
        let highlighter = SyntaxHighlighter::new();
        let result = parse_unified_diff(
            "test.txt",
            FileStatus::Modified,
            2,
            2,
            diff_text,
            &highlighter,
        );
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].old_start, 1);
        assert_eq!(diff.hunks[1].old_start, 10);
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<div>"), "&lt;div&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn test_count_additions_deletions() {
        let patch = r"@@ -1,3 +1,4 @@
 context
-deleted
+added1
+added2
 context";
        let (adds, dels) = count_additions_deletions(patch);
        assert_eq!(adds, 2);
        assert_eq!(dels, 1);
    }

    #[test]
    fn test_count_ignores_file_headers() {
        let patch = r"--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
-old
+new";
        let (adds, dels) = count_additions_deletions(patch);
        assert_eq!(adds, 1);
        assert_eq!(dels, 1);
    }
}
