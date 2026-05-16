//! Salvage SDK — helpers for station authors.
//!
//! A basic station should be under 100 lines including imports.
//! The SDK provides:
//!   - Feed parsing (RSS / Atom via feed-rs)
//!   - HTML-to-text rendering
//!   - HTTP fetch wrapper over wasi:http
//!   - Cache helper over host-cache

pub mod cache;
pub mod feed;
pub mod html;
pub mod http;

pub use anyhow::{anyhow, bail, Result};
pub use cache::Cache;
pub use feed::{parse_atom, parse_rss, RawItem};
pub use html::html_to_text;
