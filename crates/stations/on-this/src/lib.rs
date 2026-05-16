//! ON-THIS — 112.0 MHz
//!
//! Wikipedia "On This Day" — one historical event per 6-hour window.
//! This station demonstrates that the WIT interface isn't RSS-shaped:
//! it fetches JSON from the Wikipedia REST API and formats the result.
//! Cadence: 6 hours. Up to four events surfaced per calendar day.

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
use serde::Deserialize;

struct OnThisStation;

// ── Wikipedia API response types ─────────────────────────────────────────────

#[derive(Deserialize)]
struct OnThisDay {
    events: Vec<WikiEvent>,
}

#[derive(Deserialize)]
struct WikiEvent {
    year:  serde_json::Value, // API returns int or string
    text:  String,
    pages: Vec<WikiPage>,
}

#[derive(Deserialize)]
struct WikiPage {
    title:        String,
    extract:      Option<String>,
    content_urls: Option<WikiContentUrls>,
}

#[derive(Deserialize)]
struct WikiContentUrls {
    desktop: Option<WikiDesktop>,
}

#[derive(Deserialize)]
struct WikiDesktop {
    page: Option<String>,
}

// ── Station impl ──────────────────────────────────────────────────────────────

impl Guest for OnThisStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "ON-THIS".to_string(),
            name:            "On This Day".to_string(),
            description:     "One Wikipedia historical event per 6-hour window.".to_string(),
            frequency:       "112.0".to_string(),
            operator:        "Pete".to_string(),
            cadence_seconds: 21600, // 6 hours
            declared_hosts:  vec!["en.wikipedia.org".to_string()],
        }
    }

    fn tune() -> Signal {
        let now = wall_clock::now().seconds;
        let (month, day) = unix_to_month_day(now);

        let url = format!(
            "https://en.wikipedia.org/api/rest_v1/feed/onthisday/events/{month}/{day}"
        );

        let bytes = match http_get(&url) {
            Ok(b) => b,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        let parsed: OnThisDay = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(e) => return Signal::OffAir(format!("parse failed: {e}")),
        };

        if parsed.events.is_empty() {
            return Signal::Static;
        }

        let last_seen: Vec<String> = host_cache::get("last_seen")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();

        // Pick an unseen event. Rotate through the list using the 6-hour window
        // index as a hint so different windows naturally surface different events.
        let window = (now / 21600) as usize;
        let n = parsed.events.len();
        let fresh = (0..n)
            .map(|i| &parsed.events[(window + i) % n])
            .find(|ev| {
                let id = event_id(ev);
                !last_seen.contains(&id)
            });

        let event = match fresh {
            Some(e) => e,
            None => return Signal::Static,
        };

        let id = event_id(event);
        let page = event.pages.first();
        let year = event.year.as_i64().unwrap_or_else(||
            event.year.as_str().and_then(|s| s.parse().ok()).unwrap_or(0)
        );

        let title = format!("{year} — {}", event.text);
        let body = page
            .and_then(|p| p.extract.as_deref())
            .unwrap_or("")
            .to_string();
        let permalink = page
            .and_then(|p| p.content_urls.as_ref())
            .and_then(|u| u.desktop.as_ref())
            .and_then(|d| d.page.as_deref())
            .unwrap_or("")
            .to_string();
        let source_name = page
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Wikipedia".to_string());

        let mut new_seen = last_seen;
        new_seen.push(id.clone());
        new_seen.truncate(100);
        if let Ok(b) = serde_json::to_vec(&new_seen) {
            host_cache::set("last_seen", &b);
        }

        Signal::OnAir(bindings::exports::radio::station::station::Broadcast {
            id,
            title,
            body,
            source: source_name,
            permalink,
            published: now,
            tags: vec!["on-this-day".to_string()],
        })
    }
}

fn event_id(event: &WikiEvent) -> String {
    let year = event.year.as_i64().unwrap_or(0);
    format!("on-this:{year}:{}", &event.text[..event.text.len().min(60)])
}

// ── Date helpers ──────────────────────────────────────────────────────────────

fn unix_to_month_day(secs: u64) -> (u8, u8) {
    let mut days = (secs / 86400) as u32;
    let mut year = 1970u32;
    loop {
        let ydays = if is_leap(year) { 366 } else { 365 };
        if days < ydays {
            break;
        }
        days -= ydays;
        year += 1;
    }
    let month_lens: [u8; 12] = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 0usize;
    let mut rem = days as u8;
    while month < 12 && rem >= month_lens[month] {
        rem -= month_lens[month];
        month += 1;
    }
    (month as u8 + 1, rem + 1)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ── HTTP GET (same pattern as all other stations) ─────────────────────────────

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
    req.set_method(&Method::Get).ok();
    req.set_scheme(Some(&if is_https { Scheme::Https } else { Scheme::Http })).ok();
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
        None => return Err(anyhow::anyhow!("response not ready")),
    };
    if !(200..300).contains(&resp.status()) {
        return Err(anyhow::anyhow!("HTTP {}", resp.status()));
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
                return Err(anyhow::anyhow!("read: {e:?}"))
            }
        }
    }
    drop(stream);
    IncomingBody::finish(body);
    Ok(data)
}

bindings::export!(OnThisStation with_types_in bindings);
