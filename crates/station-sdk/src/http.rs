//! HTTP helper over wasi:http.
//!
//! On wasm32-wasip2: wraps wasi:http outgoing-handler into a single blocking fetch().
//! On native: stub only — stations always run as wasm32 components.

/// Fetch a URL and return the response body as bytes.
///
/// Handles gzip and redirects via host-side wasi:http implementation.
/// Only usable inside a station component (wasm32-wasip2 target).
pub fn fetch(url: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        fetch_wasm(url)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = url;
        anyhow::bail!("http::fetch is only available inside a station component (wasm32 target)")
    }
}

#[cfg(target_arch = "wasm32")]
fn fetch_wasm(url: &str) -> anyhow::Result<Vec<u8>> {
    // TODO M1: implement via wasi:http/outgoing-handler generated bindings.
    // Rough shape:
    //
    //   use wasi::http::outgoing_handler;
    //   use wasi::http::types::{Fields, OutgoingRequest, Scheme};
    //
    //   let headers = Fields::new();
    //   let req = OutgoingRequest::new(headers);
    //   req.set_method(&Method::Get).unwrap();
    //   req.set_scheme(Some(&Scheme::Https)).unwrap();
    //   req.set_authority(Some(authority)).unwrap();
    //   req.set_path_with_query(Some(path_and_query)).unwrap();
    //   let resp = outgoing_handler::handle(req, None)?;
    //   // poll for response, read body stream...
    let _ = url;
    anyhow::bail!("http::fetch not yet implemented (TODO M1)")
}
