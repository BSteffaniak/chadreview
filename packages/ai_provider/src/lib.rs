#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! AI provider abstraction for `ChadReview`.
//!
//! This crate provides the `AiProvider` trait for integrating with AI systems.

mod provider;

pub use chadreview_ai_provider_models as models;
pub use provider::{AiProvider, AiProviderError};

// Re-export the channel types for convenience
pub use switchy::unsync::sync::mpsc;
