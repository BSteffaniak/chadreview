#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_nonewlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// # Errors
    /// Returns an error if highlighting fails
    pub fn highlight_line(
        &self,
        filename: &str,
        content: &str,
    ) -> Result<Vec<(Style, String)>, String> {
        let syntax = self
            .syntax_set
            .find_syntax_for_file(filename)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut result = Vec::new();
        for line in LinesWithEndings::from(content) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .map_err(|e| format!("Failed to highlight line: {e}"))?;
            result.extend(
                ranges
                    .into_iter()
                    .map(|(style, text)| (style, text.to_string())),
            );
        }

        Ok(result)
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_code() {
        let h = SyntaxHighlighter::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}\n";
        let result = h.highlight_line("test.rs", content);
        assert!(result.is_ok());
        let ranges = result.unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_highlight_javascript_code() {
        let h = SyntaxHighlighter::new();
        let content = "function hello() {\n    console.log('Hello');\n}\n";
        let result = h.highlight_line("test.js", content);
        assert!(result.is_ok());
        let ranges = result.unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_highlight_python_code() {
        let h = SyntaxHighlighter::new();
        let content = "def hello():\n    print('Hello')\n";
        let result = h.highlight_line("test.py", content);
        assert!(result.is_ok());
        let ranges = result.unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_highlight_unknown_extension() {
        let h = SyntaxHighlighter::new();
        let content = "some plain text\n";
        let result = h.highlight_line("test.unknown", content);
        assert!(result.is_ok());
        let ranges = result.unwrap();
        assert!(!ranges.is_empty());
    }
}
