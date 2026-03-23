#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
#![forbid(unsafe_code)]

//! # nexcore-fda-guidance
//!
//! Search and retrieval for 2,794+ FDA guidance documents.
//! Data embedded at compile time from FDA's static JSON endpoint.

pub mod format;
pub mod index;
pub mod refresh;
pub mod types;

pub use types::{FdaGuidanceDoc, FdaGuidanceError};
