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

impl Widget for StationList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, THEME.fg());

        // Header
        buf.set_string(area.left() + 1, area.top(), "STATIONS", THEME.brass());

        for (i, station) in self.stations.iter().enumerate() {
            let y = area.top() + 2 + i as u16;
            if y >= area.bottom() {
                break;
            }

            let is_active = i == self.active_idx;

            if is_active {
                // Flood the row with the active background first.
                for x in area.left()..area.right() {
                    buf.cell_mut((x, y))
                        .map(|c| c.set_char(' ').set_style(THEME.active_station()));
                }
            }

            let (glyph, glyph_style, text_style) = if is_active {
                ("●", THEME.active_station(), THEME.active_station())
            } else {
                match station.status {
                    StationStatus::OnAir  => ("●", THEME.brass(),     THEME.fg_dim()),
                    StationStatus::Static => ("○", THEME.tarnish(),   THEME.fg_dim()),
                    StationStatus::OffAir => ("×", THEME.rust_warn(), THEME.fg_dim()),
                }
            };

            buf.set_string(area.left() + 1, y, glyph, glyph_style);
            buf.set_string(area.left() + 3, y, &station.callsign, text_style);
        }
    }
}
