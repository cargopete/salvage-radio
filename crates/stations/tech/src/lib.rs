//! TECH — 104.7 MHz
//!
//! Reference implementation station. Engineering blogs, deep-dive writeups,
//! primary sources. HN >= 150 points, lobste.rs >= 10 votes.

#[allow(warnings)]
mod bindings;

use bindings::exports::radio::station::station::{Broadcast, Guest, Metadata, Signal};
use bindings::radio::station::host_cache;
use bindings::wasi::http::{
    outgoing_handler,
    types::{Fields, IncomingBody, Method, OutgoingRequest, Scheme},
};
use bindings::wasi::io::{poll::poll, streams::StreamError};
use salvage_sdk::{html_to_text, parse_rss, RawItem, Result};

struct TechStation;

impl Guest for TechStation {
    fn describe() -> Metadata {
        Metadata {
            callsign: "TECH".to_string(),
            name: "Tech".to_string(),
            description: "Engineering deep-dives. HN >= 150 points, lobste.rs >= 10 votes."
                .to_string(),
            frequency: "104.7".to_string(),
            operator: "Pete".to_string(),
            cadence_seconds: 600,
            declared_hosts: vec!["hnrss.org".to_string(), "lobste.rs".to_string()],
        }
    }

    fn tune() -> Signal {
        let last_seen: Vec<String> = host_cache::get("last_seen")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();

        let items = match fetch_all() {
            Ok(items) => items,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        let fresh: Vec<_> = items
            .into_iter()
            .filter(|i| !last_seen.contains(&i.id))
            .collect();

        match fresh.into_iter().next() {
            Some(item) => {
                let mut new_seen = last_seen;
                new_seen.push(item.id.clone());
                new_seen.truncate(200);
                if let Ok(bytes) = serde_json::to_vec(&new_seen) {
                    host_cache::set("last_seen", &bytes);
                }
                Signal::OnAir(Broadcast {
                    id: item.id,
                    title: item.title,
                    body: html_to_text(&item.body_html),
                    source: item.source_name,
                    permalink: item.permalink,
                    published: item.published,
                    tags: vec!["tech".to_string()],
                })
            }
            None => Signal::Static,
        }
    }
}

fn fetch_all() -> Result<Vec<RawItem>> {
    let hn = http_get("https://hnrss.org/frontpage?points=150")?;
    let lo = http_get("https://lobste.rs/rss")?;

    let mut items = parse_rss(&hn)?;
    items.extend(parse_rss(&lo)?);
    items.sort_by_key(|i| std::cmp::Reverse(i.published));
    Ok(items)
}

/// Blocking HTTP GET using wasi:http/outgoing-handler.
/// Polls synchronously — suitable for station components (no async executor).
fn http_get(url: &str) -> anyhow::Result<Vec<u8>> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| anyhow::anyhow!("unsupported scheme: {url}"))?;

    let is_https = url.starts_with("https://");
    let (authority, path_query) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    let headers = Fields::new();
    let req = OutgoingRequest::new(headers);
    let scheme = if is_https { Scheme::Https } else { Scheme::Http };
    req.set_method(&Method::Get).ok();
    req.set_scheme(Some(&scheme)).ok();
    req.set_authority(Some(authority)).ok();
    req.set_path_with_query(Some(path_query)).ok();

    let fut = outgoing_handler::handle(req, None)
        .map_err(|e| anyhow::anyhow!("http handle: {e:?}"))?;

    {
        let p = fut.subscribe();
        poll(&[&p]);
    }

    let resp = match fut.get() {
        Some(Ok(Ok(r))) => r,
        Some(Ok(Err(e))) => return Err(anyhow::anyhow!("HTTP error: {e:?}")),
        Some(Err(())) => return Err(anyhow::anyhow!("future already consumed")),
        None => return Err(anyhow::anyhow!("response not ready after poll")),
    };

    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(anyhow::anyhow!("HTTP {status}"));
    }

    let body = resp.consume().map_err(|_| anyhow::anyhow!("consume() failed"))?;
    let stream = body.stream().map_err(|_| anyhow::anyhow!("stream() failed"))?;

    let mut data = Vec::new();
    loop {
        let p = stream.subscribe();
        poll(&[&p]);
        drop(p);
        match stream.read(64 * 1024) {
            Ok(chunk) => data.extend(chunk),
            Err(StreamError::Closed) => break,
            Err(StreamError::LastOperationFailed(e)) => {
                return Err(anyhow::anyhow!("stream read: {e:?}"))
            }
        }
    }
    drop(stream);
    IncomingBody::finish(body);

    Ok(data)
}

bindings::export!(TechStation with_types_in bindings);
