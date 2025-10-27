# chadreview_markdown

Markdown to HTML rendering for ChadReview, with GitHub Flavored Markdown support.

## Features

- **Tables**: Full table support with alignment
- **Strikethrough**: `~~text~~` syntax
- **Task lists**: `- [x]` and `- [ ]` checkboxes
- **Footnotes**: Reference-style footnotes
- **Smart punctuation**: Automatic quotes and dashes
- **XSS Protection**: Raw HTML is stripped for security

## Usage

```rust
use chadreview_markdown::render_markdown;

let markdown = "**Hello** _world_!";
let html = render_markdown(markdown);
```

## Security

This crate prioritizes security:

- Raw HTML tags in markdown are **not rendered** (filtered out)
- Only safe markdown elements are converted to HTML
- Prevents XSS attacks from user-generated content
