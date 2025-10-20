#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

pub mod comment;
pub mod diff;
pub mod pr;
pub mod user;

pub use comment::{Comment, CommentType, CreateComment};
pub use diff::{DiffFile, DiffHunk, DiffLine, FileStatus, LineType};
pub use pr::{PrState, PullRequest};
pub use user::{Commit, Label, User};
