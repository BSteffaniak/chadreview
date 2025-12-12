#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Git backend trait abstraction for `ChadReview`.
//!
//! This crate defines the `GitBackend` and `GitRepository` traits that abstract
//! over git implementations (git2, CLI, mock, etc.) for testability.

mod backend;

pub use backend::{GitBackend, GitRepository};
pub use chadreview_git_backend_models::*;
