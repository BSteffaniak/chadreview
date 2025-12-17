# ChadReview Diff Models

Shared diff-related data models for ChadReview.

## Overview

This crate provides common types used across multiple ChadReview packages for working with diffs:

- `LineNumber` - Represents a line number on either the old or new side of a diff

## Usage

```rust
use chadreview_diff_models::LineNumber;
use std::str::FromStr;

// Parse from string format
let new_line = LineNumber::from_str("n42").unwrap();
let old_line = LineNumber::from_str("o10").unwrap();

// Access the line number
assert_eq!(new_line.number(), 42);

// Check which side
assert!(new_line.is_new());
assert!(old_line.is_old());

// Convert back to string
assert_eq!(new_line.to_string(), "n42");
```
