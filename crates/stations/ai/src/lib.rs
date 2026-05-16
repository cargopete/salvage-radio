//! AI — 96.7 MHz
//!
//! Machine learning and AI research.
//! Sources: ArXiv cs.AI, ArXiv cs.LG, Hugging Face blog.

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

struct AiStation;

impl Guest for AiStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "AI".to_string(),
            name:            "AI".to_string(),
            description:     "Machine learning and AI research — ArXiv cs.AI/cs.LG, Hugging Face.".to_string(),
            frequency:       "96.7".to_string(),
            operator:        "Pete".to_string(),
            cadence_seconds: 3600,
            declared_hosts:  vec![
                "export.arxiv.org".to_string(),
                "huggingface.co".to_string(),
            ],
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

        let fresh = items.into_iter().find(|i| !last_seen.contains(&i.id));

        match fresh {
            Some(item) => {
                let mut new_seen = last_seen;
                new_seen.push(item.id.clone());
                new_seen.truncate(200);
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
                    tags:      vec!["ai".to_string()],
                })
            }
            None => Signal::Static,
        }
    }
}

fn fetch_all() -> Result<Vec<RawItem>> {
    let mut items = Vec::new();
    for url in &[
        "http://export.arxiv.org/rss/cs.AI",
        "http://export.arxiv.org/rss/cs.LG",
        "https://huggingface.co/blog/feed.xml",
    ] {
        match http_get(url).and_then(|b| parse_rss(&b)) {
            Ok(feed) => items.extend(feed),
            Err(_) => {}
        }
    }
    if items.is_empty() {
        anyhow::bail!("all feeds failed");
    }
    items.sort_by_key(|i| std::cmp::Reverse(i.published));
    Ok(items)
}

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
    { let p = fut.subscribe(); poll(&[&p]); }
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
        let p = stream.subscribe(); poll(&[&p]); drop(p);
        match stream.read(64 * 1024) {
            Ok(chunk) => data.extend(chunk),
            Err(StreamError::Closed) => break,
            Err(StreamError::LastOperationFailed(e)) => return Err(anyhow::anyhow!("read: {e:?}")),
        }
    }
    drop(stream);
    IncomingBody::finish(body);
    Ok(data)
}

bindings::export!(AiStation with_types_in bindings);
