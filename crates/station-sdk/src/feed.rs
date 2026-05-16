//! RSS / Atom parsing via feed-rs.
//!
//! Both formats go through the same path — feed-rs detects the format.
//! Output is always a Vec<RawItem> sorted newest-first by the caller if needed.

use anyhow::Result;

/// A feed item before station-specific processing.
/// Body is raw HTML; call `html_to_text()` before broadcasting.
#[derive(Debug, Clone)]
pub struct RawItem {
    /// Stable ID from upstream GUID / entry id. Use as broadcast.id.
    pub id: String,
    pub title: String,
    /// Raw HTML body as received from the feed. Strip before broadcasting.
    pub body_html: String,
    pub permalink: String,
    /// Unix seconds. 0 if the feed doesn't provide a date.
    pub published: u64,
    /// Feed-level title used as broadcast.source.
    pub source_name: String,
}

pub fn parse_rss(bytes: &[u8]) -> Result<Vec<RawItem>> {
    parse_feed(bytes)
}

pub fn parse_atom(bytes: &[u8]) -> Result<Vec<RawItem>> {
    parse_feed(bytes)
}

fn parse_feed(bytes: &[u8]) -> Result<Vec<RawItem>> {
    let feed = feed_rs::parser::parse(bytes)?;
    let source = feed.title.map(|t| t.content).unwrap_or_default();

    let items = feed
        .entries
        .into_iter()
        .map(|e| {
            let id = if e.id.is_empty() {
                e.links.first().map(|l| l.href.clone()).unwrap_or_default()
            } else {
                e.id
            };

            let title = e.title.map(|t| t.content).unwrap_or_default();

            let body_html = e
                .content
                .and_then(|c| c.body)
                .or_else(|| e.summary.map(|s| s.content))
                .unwrap_or_default();

            let permalink = e
                .links
                .into_iter()
                .next()
                .map(|l| l.href)
                .unwrap_or_default();

            let published = e
                .published
                .or(e.updated)
                .map(|dt| dt.timestamp().max(0) as u64)
                .unwrap_or(0);

            RawItem {
                id,
                title,
                body_html,
                permalink,
                published,
                source_name: source.clone(),
            }
        })
        .collect();

    Ok(items)
}
