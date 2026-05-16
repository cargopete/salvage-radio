//! The frequency dial — the visual centrepiece.
//!
//! Four fixed rows at the bottom of the layout:
//!   Row 1: frequency markings + ▼ indicator over the active station
//!   Row 2: tuning track — ╪╪╪ under active, · under others
//!   Row 3: callsigns
//!   Row 4: separator
//!
//! Scrolls horizontally when stations outnumber available columns.
//! Stations sorted by frequency (their aesthetic position), not registration order.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

pub struct DialStation {
    pub callsign:  String,
    pub frequency: String,
}

pub struct Dial<'a> {
    pub stations:      &'a [DialStation],
    pub active_idx:    usize,
    pub scroll_offset: usize,
}

impl<'a> Widget for Dial<'a> {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO M3:
        // - compute column positions for each station (fixed width per station)
        // - row 0: freq labels + ▼ over active (copper_bright)
        // - row 1: · · · ╪╪╪ · · · (copper for active, tarnish for others)
        // - row 2: callsigns (copper for active, foreground_dim for others)
        let _ = &THEME;
    }
}
