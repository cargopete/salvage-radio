//! Declared-hosts enforcement for outgoing HTTP.
//!
//! `DeclaredHostsGuard` implements `WasiHttpHooks::send_request` and blocks
//! any outgoing request whose authority is not in the station's declared_hosts
//! list. The guard is stored in `StationData` and referenced by `WasiHttpView`.
//!
//! This is host-side enforcement — correct for v1, but advisory at the type level.
//! When wasi:http gains proper capability scoping, we move to that instead.

use wasmtime_wasi_http::p2::{
    HttpResult, WasiHttpHooks,
    body::HyperOutgoingBody,
    default_send_request,
    types::{HostFutureIncomingResponse, OutgoingRequestConfig},
};

/// Guards a single station's outgoing HTTP against its declared_hosts list.
pub struct DeclaredHostsGuard {
    pub declared_hosts: Vec<String>,
}

impl WasiHttpHooks for DeclaredHostsGuard {
    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        let host = request
            .uri()
            .authority()
            .map(|a| a.host().to_string())
            .unwrap_or_default();

        if self.declared_hosts.iter().any(|h| h == &host) {
            Ok(default_send_request(request, config))
        } else {
            Err(wasmtime_wasi_http::p2::HttpError::trap(
                std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("outgoing request to '{host}' blocked: not in declared_hosts"),
                ),
            ))
        }
    }
}

/// Returns true if the request target host is permitted by the station's declaration.
/// Used by tests and headless mode.
pub fn is_permitted(url: &str, declared_hosts: &[String]) -> bool {
    let Ok(parsed) = url::Url::parse(url) else { return false };
    let Some(host) = parsed.host_str() else { return false };
    declared_hosts.iter().any(|h| h == host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permits_declared_host() {
        let hosts = vec!["hnrss.org".to_string(), "lobste.rs".to_string()];
        assert!(is_permitted("https://hnrss.org/frontpage?points=150", &hosts));
        assert!(is_permitted("https://lobste.rs/rss", &hosts));
    }

    #[test]
    fn rejects_undeclared_host() {
        let hosts = vec!["hnrss.org".to_string()];
        assert!(!is_permitted("https://evil.example.com/exfil", &hosts));
    }

    #[test]
    fn rejects_malformed_url() {
        let hosts = vec!["hnrss.org".to_string()];
        assert!(!is_permitted("not-a-url", &hosts));
    }
}
