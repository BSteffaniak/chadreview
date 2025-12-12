#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Generic unified diff parsing library for `ChadReview`.
//!
//! This crate provides utilities for parsing unified diff format into structured
//! data models with syntax highlighting support.

pub mod parser;

pub use parser::parse_unified_diff;
