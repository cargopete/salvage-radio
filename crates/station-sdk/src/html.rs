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
    // Collapse whitespace
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
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
