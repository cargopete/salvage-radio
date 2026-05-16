//! Declared-hosts enforcement for outgoing HTTP.
//!
//! Wraps the wasi:http outgoing-handler: inspects outgoing requests and rejects
//! any whose host is not in the station's declared_hosts list.
//!
//! This is host-side enforcement — correct for v1, but advisory at the type level.
//! When wasi:http gains proper capability scoping, we move to that instead.

/// Returns true if the request target host is permitted by the station's declaration.
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
