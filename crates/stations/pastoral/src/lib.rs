//! PASTORAL — 92.4 MHz
//!
//! Small-press essays and country-life writing.
//! Sources: Emergence Magazine, Orion Magazine.
//!
//! At most two broadcasts per day. Quiet is the default state.
//! That's the whole point.

#[allow(warnings)]
mod bindings;

use bindings::exports::radio::station::station::{Broadcast, Guest, Metadata, Signal};
use bindings::radio::station::host_cache;
use bindings::wasi::clocks::wall_clock;
use bindings::wasi::http::{
    outgoing_handler,
    types::{Fields, IncomingBody, Method, OutgoingRequest, Scheme},
};
use bindings::wasi::io::{poll::poll, streams::StreamError};
use salvage_sdk::{html_to_text, parse_rss, RawItem, Result};

const MAX_PER_DAY: u64 = 2;

struct PastoralStation;

impl Guest for PastoralStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "PASTORAL".to_string(),
            name:            "Pastoral".to_string(),
            description:     "Small-press essays and country-life writing. At most two broadcasts a day — quiet by design.".to_string(),
            frequency:       "92.4".to_string(),
            operator:        "Pete".to_string(),
            cadence_seconds: 3600,
            declared_hosts:  vec![
                "emergencemagazine.org".to_string(),
                "orionmagazine.org".to_string(),
            ],
        }
    }

    fn tune() -> Signal {
        // Daily budget: unix-day number as the period key.
        let today: u64 = wall_clock::now().seconds / 86400;
        let (cached_day, count): (u64, u64) = host_cache::get("daily")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or((0, 0));

        if cached_day == today && count >= MAX_PER_DAY {
            return Signal::Static;
        }

        let last_seen: Vec<String> = host_cache::get("last_seen")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();

        let items = match fetch_all() {
            Ok(items) => items,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        let fresh = items
            .into_iter()
            .filter(|i| !last_seen.contains(&i.id))
            .next();

        match fresh {
            Some(item) => {
                let new_count = if cached_day == today { count + 1 } else { 1 };
                if let Ok(b) = serde_json::to_vec(&(today, new_count)) {
                    host_cache::set("daily", &b);
                }
                let mut new_seen = last_seen;
                new_seen.push(item.id.clone());
                new_seen.truncate(100);
                if let Ok(b) = serde_json::to_vec(&new_seen) {
                    host_cache::set("last_seen", &b);
                }
                Signal::OnAir(Broadcast {
                    id:        item.id,
                    title:     item.title,
                    body:      html_to_text(&item.body_html),
                    source:    item.source_name,
                    permalink: item.permalink,
                    published: item.published,
                    tags:      vec!["pastoral".to_string()],
                })
            }
            None => Signal::Static,
        }
    }
}

fn fetch_all() -> Result<Vec<RawItem>> {
    let emergence = http_get("https://emergencemagazine.org/feed/")?;
    let orion = http_get("https://orionmagazine.org/feed/")?;
    let mut items = parse_rss(&emergence)?;
    items.extend(parse_rss(&orion)?);
    items.sort_by_key(|i| std::cmp::Reverse(i.published));
    Ok(items)
}

/// Blocking HTTP GET using wasi:http/outgoing-handler.
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

bindings::export!(PastoralStation with_types_in bindings);
