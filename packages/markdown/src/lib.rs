#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use gh_emoji::Replacer;
use pulldown_cmark::{Options, Parser, html};

#[must_use]
pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();

    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let replacer = Replacer::new();
    let markdown_with_emoji = replacer.replace_all(markdown);

    let parser = Parser::new_ext(&markdown_with_emoji, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    filter_dangerous_html(&html_output)
}

fn filter_dangerous_html(html: &str) -> String {
    const DANGEROUS_TAGS: &[(&str, &str)] = &[
        ("title", "TITLE"),
        ("textarea", "TEXTAREA"),
        ("style", "STYLE"),
        ("xmp", "XMP"),
        ("iframe", "IFRAME"),
        ("noembed", "NOEMBED"),
        ("noframes", "NOFRAMES"),
        ("script", "SCRIPT"),
        ("plaintext", "PLAINTEXT"),
    ];

    let mut result = html.to_string();
    for (lower, upper) in DANGEROUS_TAGS {
        result = result.replace(&format!("<{lower}"), &format!("&lt;{lower}"));
        result = result.replace(&format!("<{upper}"), &format!("&lt;{upper}"));
        result = result.replace(&format!("</{lower}>"), &format!("&lt;/{lower}&gt;"));
        result = result.replace(&format!("</{upper}>"), &format!("&lt;/{upper}&gt;"));
    }
    result
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
        assert!(html.contains("&lt;script>"));
    }

    #[test]
    fn test_safe_html_allowed() {
        let md = "Normal text <kbd>Ctrl</kbd> more text";
        let html = render_markdown(md);
        assert!(html.contains("<kbd>"));
        assert!(html.contains("Ctrl"));
    }

    #[test]
    fn test_dangerous_html_filtered() {
        let md = "Text <script>alert('xss')</script> more";
        let html = render_markdown(md);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script>"));
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

    #[test]
    fn test_emoji_shortcodes() {
        let md = ":white_check_mark: Done! :loudspeaker: Announcement";
        let html = render_markdown(md);
        assert!(html.contains("✅"));
        assert!(html.contains("📢"));
        assert!(!html.contains(":white_check_mark:"));
        assert!(!html.contains(":loudspeaker:"));
    }

    #[test]
    fn test_emoji_with_markdown() {
        let md = "**Status:** :rocket: Deployed!";
        let html = render_markdown(md);
        assert!(html.contains("<strong>Status:</strong>"));
        assert!(html.contains("🚀"));
    }

    #[test]
    fn test_invalid_emoji_shortcode() {
        let md = "This :not_a_real_emoji: should stay";
        let html = render_markdown(md);
        assert!(html.contains(":not_a_real_emoji:"));
    }
}
