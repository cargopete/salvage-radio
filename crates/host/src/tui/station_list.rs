//! Left panel: list of all stations with status glyphs.
//!
//! Glyphs:
//!   ●  brass   — on air (has broadcast in the last cadence window)
//!   ○  tarnish — static (healthy but quiet)
//!   ×  rust    — off air (failing or unreachable)
//!
//! Active station: copper background, parchment text.
//! Inactive stations: foreground_dim text.
//! Panel width: 14 columns (fixed).

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StationStatus {
    OnAir,
    Static,
    OffAir,
}

pub struct StationEntry {
    pub callsign: String,
    pub status:   StationStatus,
}

pub struct StationList<'a> {
    pub stations:   &'a [StationEntry],
    pub active_idx: usize,
}

impl<'a> Widget for StationList<'a> {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO M3: for each station, render "● CALLSIGN" with appropriate style.
        // Active row: THEME.active_station()
        // Glyph colours: brass=on-air, tarnish=static, rust=off-air
        let _ = &THEME;
    }
}
