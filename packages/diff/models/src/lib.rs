#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Shared diff models for `ChadReview`.
//!
//! This crate provides common diff-related types used across multiple packages,
//! including line number representations for both old and new file sides.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Represents a line number on either the old (removed) or new (added) side of a diff.
///
/// In unified diff format, each line can reference either the original file (old side)
/// or the modified file (new side). This enum captures that distinction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "side", rename_all = "snake_case")]
pub enum LineNumber {
    /// Line number on the old (original/removed) side of the diff.
    Old { line: u64 },
    /// Line number on the new (modified/added) side of the diff.
    New { line: u64 },
}

/// Error returned when parsing a `LineNumber` from a string fails.
#[derive(Debug, thiserror::Error)]
#[error("Invalid LineNumber format")]
pub struct ParseLineNumberError;

impl FromStr for LineNumber {
    type Err = ParseLineNumberError;

    /// Parse a `LineNumber` from a string.
    ///
    /// Format: `n<line>` for new side, `o<line>` for old side.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use chadreview_diff_models::LineNumber;
    ///
    /// let new_line = LineNumber::from_str("n42").unwrap();
    /// assert!(matches!(new_line, LineNumber::New { line: 42 }));
    ///
    /// let old_line = LineNumber::from_str("o10").unwrap();
    /// assert!(matches!(old_line, LineNumber::Old { line: 10 }));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('n') {
            Ok(Self::New {
                line: s.parse::<u64>().map_err(|_| ParseLineNumberError)?,
            })
        } else if let Some(s) = s.strip_prefix('o') {
            Ok(Self::Old {
                line: s.parse::<u64>().map_err(|_| ParseLineNumberError)?,
            })
        } else {
            Err(ParseLineNumberError)
        }
    }
}

impl LineNumber {
    /// Returns the numeric line value regardless of side.
    #[must_use]
    pub const fn number(&self) -> u64 {
        match self {
            Self::Old { line } | Self::New { line } => *line,
        }
    }

    /// Returns `true` if this is an old (original) side line number.
    #[must_use]
    pub const fn is_old(&self) -> bool {
        matches!(self, Self::Old { .. })
    }

    /// Returns `true` if this is a new (modified) side line number.
    #[must_use]
    pub const fn is_new(&self) -> bool {
        matches!(self, Self::New { .. })
    }
}

impl std::fmt::Display for LineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Old { line } => write!(f, "o{line}"),
            Self::New { line } => write!(f, "n{line}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new_line() {
        let line = LineNumber::from_str("n42").unwrap();
        assert!(matches!(line, LineNumber::New { line: 42 }));
        assert_eq!(line.number(), 42);
        assert!(line.is_new());
        assert!(!line.is_old());
    }

    #[test]
    fn test_parse_old_line() {
        let line = LineNumber::from_str("o10").unwrap();
        assert!(matches!(line, LineNumber::Old { line: 10 }));
        assert_eq!(line.number(), 10);
        assert!(line.is_old());
        assert!(!line.is_new());
    }

    #[test]
    fn test_parse_invalid() {
        assert!(LineNumber::from_str("42").is_err());
        assert!(LineNumber::from_str("x10").is_err());
        assert!(LineNumber::from_str("").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(LineNumber::New { line: 42 }.to_string(), "n42");
        assert_eq!(LineNumber::Old { line: 10 }.to_string(), "o10");
    }

    #[test]
    fn test_roundtrip() {
        let original = LineNumber::New { line: 123 };
        let parsed = LineNumber::from_str(&original.to_string()).unwrap();
        assert_eq!(original, parsed);
    }
}
