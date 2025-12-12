# chadreview_diff

Generic unified diff parsing library for ChadReview.

This package provides utilities for parsing unified diff format (the standard output of `git diff`) into structured data models with syntax highlighting support.

## Features

- Parse unified diff format into `DiffFile`, `DiffHunk`, and `DiffLine` structures
- Automatic syntax highlighting via `syntect`
- Support for all standard diff operations (additions, deletions, context lines)
- Handles multiple hunks per file

## Usage

```rust
use chadreview_diff::parser::parse_unified_diff;
use chadreview_pr_models::FileStatus;
use chadreview_syntax::SyntaxHighlighter;

let diff_text = r#"@@ -1,4 +1,4 @@
 fn main() {
-    println!("Hello");
+    println!("World");
 }"#;

let highlighter = SyntaxHighlighter::new();
let diff_file = parse_unified_diff(
    "test.rs",
    FileStatus::Modified,
    1,  // additions
    1,  // deletions
    diff_text,
    &highlighter,
).unwrap();
```
