use serde::{Deserialize, Serialize};

use crate::comment::LineNumber;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub filename: String,
    pub status: FileStatus,
    pub additions: u64,
    pub deletions: u64,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u64,
    pub old_lines: u64,
    pub new_start: u64,
    pub new_lines: u64,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: LineType,
    pub old_line_number: Option<u64>,
    pub new_line_number: Option<u64>,
    pub content: String,
    pub highlighted_html: String,
}

impl std::fmt::Display for DiffLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        if let Some(line_num) = self.old_line_number {
            f.write_fmt(format_args!("{line_num}"))?;
        } else {
            f.write_char('u')?;
        }
        f.write_char('-')?;
        if let Some(line_num) = self.new_line_number {
            f.write_fmt(format_args!("{line_num}"))?;
        } else {
            f.write_char('u')?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LineType {
    Addition,
    Deletion,
    Context,
}

impl From<&DiffLine> for LineNumber {
    fn from(line: &DiffLine) -> Self {
        match line.line_type {
            LineType::Addition => Self::New {
                line: line.new_line_number.expect("Missing new line number"),
            },
            LineType::Deletion => Self::Old {
                line: line.old_line_number.expect("Missing old line number"),
            },
            LineType::Context => Self::Old {
                line: line
                    .old_line_number
                    .or(line.new_line_number)
                    .expect("Missing line number"),
            },
        }
    }
}
