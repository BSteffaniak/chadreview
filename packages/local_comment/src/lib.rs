#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Local comment storage for `ChadReview`.
//!
//! This crate provides XDG-compliant file-based storage for local diff comments.

mod store;

pub use chadreview_local_comment_models as models;
pub use store::{LocalCommentStore, LocalCommentStoreError};
