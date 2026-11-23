//! LG ThinQ Connect: the official REST API LG opened in December 2024.
//!
//! A personal access token from <https://connect-pat.lgthinq.com> is all it takes; no
//! reverse-engineered endpoints, no LG password.

pub mod client;
pub mod kinds;
pub mod model;
pub mod region;
pub mod secret;
pub mod spec;

pub use client::{ApiError, Client};
pub use region::Country;
pub use secret::Secret;
