//! Now-playing panel: the right-hand main content area.
//!
//! Shows the current broadcast for the tuned station.
//! On new broadcast: title flashes copper_bright for ~50ms, then returns to copper.
//! This is the only piece of motion in the running app.
//!
//! [ < ] [ > ] in keybindings scrolls through the station's broadcast buffer (last 50).

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

pub struct NowPlaying<'a> {
    pub callsign:      &'a str,
    pub frequency:     &'a str,
    pub title:         Option<&'a str>,
    pub body:          Option<&'a str>,
    pub source:        Option<&'a str>,
    pub relative_time: Option<&'a str>,
    pub read_time:     Option<&'a str>,
    pub tags:          &'a [String],
    pub scroll_offset: u16,
    /// True for ~one render frame on new broadcast arrival. Flashes title copper_bright.
    pub flash:         bool,
}

impl<'a> Widget for NowPlaying<'a> {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO M3:
        // - header: "NOW BROADCASTING ── {NAME} ── {FREQ}" in brass, ── as separators
        // - title: copper (or copper_bright if flash), ellipsized
        // - body: parchment, word-wrapped, scrollable
        // - footer: foreground_dim "source · relative_time · read_time"
        // - tags: tarnish
        let _ = &THEME;
    }
}
