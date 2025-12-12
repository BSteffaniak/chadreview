#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Local git diff provider for `ChadReview`.
//!
//! This crate provides a `LocalDiffProvider` that extracts diffs from local
//! git repositories using the `GitBackend` trait abstraction.

mod provider;

pub use provider::LocalDiffProvider;
