#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use pulldown_cmark::{Event, Options, Parser, html};

#[must_use]
pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();

    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, options);

    let safe_parser = SanitizingParser::new(parser);

    let mut html_output = String::new();
    html::push_html(&mut html_output, safe_parser);

    html_output
}

struct SanitizingParser<'a, I> {
    inner: I,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<I> SanitizingParser<'_, I> {
    const fn new(inner: I) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, I> Iterator for SanitizingParser<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            Event::Html(_) | Event::InlineHtml(_) => self.next(),
            event => Some(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let md = "**bold** and *italic*";
        let html = render_markdown(md);
        assert_eq!(html, "<p><strong>bold</strong> and <em>italic</em></p>\n");
    }

    #[test]
    fn test_strikethrough() {
        let md = "~~strikethrough~~";
        let html = render_markdown(md);
        assert_eq!(html, "<p><del>strikethrough</del></p>\n");
    }

    #[test]
    fn test_links() {
        let md = "[link](https://example.com)";
        let html = render_markdown(md);
        assert_eq!(html, "<p><a href=\"https://example.com\">link</a></p>\n");
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = render_markdown(md);
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_inline_code() {
        let md = "`code`";
        let html = render_markdown(md);
        assert_eq!(html, "<p><code>code</code></p>\n");
    }

    #[test]
    fn test_task_list() {
        let md = "- [x] Done\n- [ ] Todo";
        let html = render_markdown(md);
        assert!(html.contains("checked"));
        assert!(html.contains("checkbox"));
    }

    #[test]
    fn test_table() {
        let md = "| Header |\n|--------|\n| Cell   |";
        let html = render_markdown(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn test_xss_protection() {
        let md = "<script>alert('xss')</script>";
        let html = render_markdown(md);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;") || !html.contains("alert"));
    }

    #[test]
    fn test_html_in_markdown_stripped() {
        let md = "Normal text <div onclick='evil()'>div</div> more text";
        let html = render_markdown(md);
        assert!(!html.contains("<div"));
        assert!(!html.contains("onclick"));
    }

    #[test]
    fn test_autolink() {
        let md = "https://example.com";
        let html = render_markdown(md);
        assert!(html.contains("example.com"));
    }

    #[test]
    fn test_multiline_markdown() {
        let md = "# Heading\n\nParagraph with **bold**.\n\n- Item 1\n- Item 2";
        let html = render_markdown(md);
        assert!(html.contains("<h1>"));
        assert!(html.contains("<p>"));
        assert!(html.contains("<strong>"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
    }

    #[test]
    fn test_empty_string() {
        let html = render_markdown("");
        assert_eq!(html, "");
    }
}
