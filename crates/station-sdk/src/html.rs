//! HTML-to-text rendering.
//!
//! Converts RSS body HTML into clean plain text suitable for broadcast bodies.
//! No images. Whitespace normalised. Links preserved as [text](url).
//!
//! v0.1: simple tag-stripping placeholder.
//! TODO: replace with a proper implementation (html2text or similar) once we've
//!       confirmed the crate is wasm32-wasip2 compatible.

/// Convert an HTML string to plain text. Never returns HTML markup.
pub fn html_to_text(html: &str) -> String {
    let stripped = strip_tags(html);
    let decoded = decode_entities(&stripped);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decode common HTML entities. Handles named entities and &#NNN; / &#xNN; numeric forms.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp + 1..];
        // Scan for the closing ';', capped at 12 chars
        if let Some(semi) = rest[..rest.len().min(12)].find(';') {
            let entity = &rest[..semi];
            rest = &rest[semi + 1..];
            let replacement: Option<&str> = match entity {
                "quot"           => Some("\""),
                "amp"            => Some("&"),
                "lt"             => Some("<"),
                "gt"             => Some(">"),
                "apos"           => Some("'"),
                "nbsp"           => Some(" "),
                "ndash" | "#8211" => Some("–"),
                "mdash" | "#8212" => Some("—"),
                "lsquo" | "#8216" => Some("'"),
                "rsquo" | "#8217" => Some("'"),
                "ldquo" | "#8220" => Some("\u{201C}"),
                "rdquo" | "#8221" => Some("\u{201D}"),
                "hellip"| "#8230" => Some("…"),
                _ => None,
            };
            if let Some(r) = replacement {
                out.push_str(r);
            } else if let Some(n_str) = entity.strip_prefix('#') {
                // Numeric entity: &#NNN; or &#xNN;
                let code = if let Some(hex) = n_str.strip_prefix('x').or_else(|| n_str.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    n_str.parse::<u32>().ok()
                };
                if let Some(ch) = code.and_then(char::from_u32) {
                    out.push(ch);
                } else {
                    out.push('&'); out.push_str(entity); out.push(';');
                }
            } else {
                // Unknown named entity — pass through
                out.push('&'); out.push_str(entity); out.push(';');
            }
        } else {
            // No closing ';' found nearby — literal '&'
            out.push('&');
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_basic_tags() {
        assert_eq!(html_to_text("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(html_to_text("  foo   bar  "), "foo bar");
    }
}
