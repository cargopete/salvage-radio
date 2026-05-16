//! The frequency dial — the visual centrepiece.
//!
//! Three rows rendered from top of the allocated area:
//!   Row 0: frequency labels; ▼ immediately before the active freq
//!   Row 1: tuning track — ╪╪╪ under active, · under others
//!   Row 2: callsigns (truncated to 6 chars)
//!
//! Each station slot is COL_WIDTH columns wide.
//! Scrolls horizontally when stations outnumber available columns.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

const COL_WIDTH: u16 = 10;

pub struct DialStation {
    pub callsign:  String,
    pub frequency: String,
}

pub struct Dial<'a> {
    pub stations:      &'a [DialStation],
    pub active_idx:    usize,
    pub scroll_offset: usize,
}

impl Widget for Dial<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, THEME.bg());

        if area.height < 3 || self.stations.is_empty() {
            return;
        }

        let freq_row = area.top();
        let track_row = area.top() + 1;
        let call_row = area.top() + 2;

        let visible = (area.width / COL_WIDTH) as usize;

        for slot in 0..visible {
            let idx = self.scroll_offset + slot;
            let Some(station) = self.stations.get(idx) else {
                break;
            };

            let col_left = area.left() + slot as u16 * COL_WIDTH;
            let center = col_left + COL_WIDTH / 2;
            let is_active = idx == self.active_idx;

            let (freq_style, track_style, call_style) = if is_active {
                (THEME.copper(), THEME.copper(), THEME.copper())
            } else {
                (THEME.tarnish(), THEME.tarnish(), THEME.tarnish())
            };

            // Row 0: freq + ▼ indicator
            if is_active {
                let label = format!("▼ {}", station.frequency);
                let x = center.saturating_sub(label.len() as u16 / 2);
                buf.set_string(x.max(col_left), freq_row, &label, freq_style);
            } else {
                let x = center.saturating_sub(station.frequency.len() as u16 / 2);
                buf.set_string(x.max(col_left), freq_row, &station.frequency, freq_style);
            }

            // Row 1: track
            if is_active {
                buf.set_string(center.saturating_sub(1), track_row, "╪╪╪", track_style);
            } else {
                buf.set_string(center, track_row, "·", track_style);
            }

            // Row 2: callsign (max 6 chars)
            let short = if station.callsign.len() > 6 {
                &station.callsign[..6]
            } else {
                &station.callsign
            };
            let x = center.saturating_sub(short.len() as u16 / 2);
            buf.set_string(x.max(col_left), call_row, short, call_style);
        }
    }
}
