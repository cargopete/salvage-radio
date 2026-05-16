//! Startup warm-up animation — ~600ms total.
//!
//! Sequence (once per session, sets the tone):
//!   1.  Empty frame                                                (~0ms)
//!   2.  ▓ blocks fill the status bar left-to-right               (~300ms)
//!   3.  Dial illuminates: freq markings appear, callsigns fade
//!       tarnish → brass                                           (~150ms, 4 frames)
//!   4.  Station list populates line by line                       (~20ms/line)
//!
//! This is the only piece of motion besides the new-broadcast flicker.
//! It happens once. It should feel like a machine coming to life.

use std::time::Duration;

use anyhow::Result;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};

use super::dial::DialStation;
use super::station_list::{StationEntry, StationList};
use super::theme::THEME;

pub async fn play(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    dial_stations: &[DialStation],
    station_entries: &[StationEntry],
    active_idx: usize,
) -> Result<()> {
    let size = terminal.size()?;
    let width = size.width as usize;

    // ── Phase 1: blank frame ─────────────────────────────────────────────────
    terminal.draw(|f| {
        use ratatui::widgets::Block;
        f.render_widget(Block::default().style(THEME.bg()), f.area());
    })?;

    // ── Phase 2: fill status bar with ▓ left-to-right ───────────────────────
    // Cap steps so it's not agonisingly slow on ultra-wide terminals.
    let steps = width.min(100);
    let delay = Duration::from_millis(300) / steps as u32;
    let bar_y = size.height.saturating_sub(1);

    for filled in 1..=steps {
        terminal.draw(|f| {
            let chars = filled.min(f.area().width as usize);
            let bar: String = "▓".repeat(chars);
            f.buffer_mut().set_string(0, bar_y, &bar, THEME.brass());
        })?;
        tokio::time::sleep(delay).await;
    }

    // ── Phase 3: dial illuminates over 4 frames ──────────────────────────────
    // Frame 0: just track marks (· / ╪╪╪) in tarnish, no labels
    // Frame 1: + frequency numbers in tarnish
    // Frame 2: + callsigns in tarnish
    // Frame 3: active station highlights to copper
    let dial_top = size.height.saturating_sub(4);
    let dial_area = Rect { x: 0, y: dial_top, width: size.width, height: 3 };

    for frame in 0u8..=3 {
        terminal.draw(|f| {
            render_dial_frame(f.buffer_mut(), dial_area, dial_stations, active_idx, frame);
        })?;
        tokio::time::sleep(Duration::from_millis(37)).await;
    }

    // ── Phase 4: station list line by line ───────────────────────────────────
    let list_area = Rect {
        x: 0,
        y: 0,
        width: 14,
        height: size.height.saturating_sub(4),
    };

    for count in 1..=station_entries.len() {
        terminal.draw(|f| {
            f.render_widget(
                StationList {
                    stations:   &station_entries[..count],
                    active_idx: active_idx.min(count.saturating_sub(1)),
                },
                list_area,
            );
        })?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    Ok(())
}

/// Render the dial at different brightness stages.
///
/// frame 0 — track marks only (no labels)
/// frame 1 — + frequency numbers (tarnish)
/// frame 2 — + callsigns (tarnish)
/// frame 3 — active station highlights to copper
fn render_dial_frame(
    buf: &mut ratatui::buffer::Buffer,
    area: Rect,
    stations: &[DialStation],
    active_idx: usize,
    frame: u8,
) {
    const COL_WIDTH: u16 = 10;

    buf.set_style(area, THEME.bg());

    let freq_row  = area.top();
    let track_row = area.top() + 1;
    let call_row  = area.top() + 2;

    for (i, station) in stations.iter().enumerate() {
        let col    = area.left() + i as u16 * COL_WIDTH;
        let center = col + COL_WIDTH / 2;
        if col + COL_WIDTH > area.right() { break; }

        let is_active = i == active_idx;
        let lit = frame >= 3 && is_active;
        let label_style = if lit { THEME.copper() } else { THEME.tarnish() };

        // Track (frame 0+)
        if is_active {
            buf.set_string(
                center.saturating_sub(1),
                track_row,
                "╪╪╪",
                if lit { THEME.copper() } else { THEME.tarnish() },
            );
        } else {
            buf.set_string(center, track_row, "·", THEME.tarnish());
        }

        // Frequency labels (frame 1+)
        if frame >= 1 {
            let prefix = if lit { "▼ " } else { "  " };
            let label = format!("{}{}", prefix, station.frequency);
            let x = center.saturating_sub(label.len() as u16 / 2);
            buf.set_string(x.max(col), freq_row, &label, label_style);
        }

        // Callsigns (frame 2+)
        if frame >= 2 {
            let short = if station.callsign.len() > 6 { &station.callsign[..6] } else { &station.callsign };
            let x = center.saturating_sub(short.len() as u16 / 2);
            buf.set_string(x.max(col), call_row, short, label_style);
        }
    }
}

/// Closing animation: "SIGNING OFF" centred on screen for ~300ms.
pub async fn sign_off(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    terminal.draw(|f| {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Paragraph};

        let area = f.area();
        f.render_widget(Block::default().style(THEME.bg()), area);

        if area.height < 1 { return; }
        let msg = "  SIGNING OFF  ";
        let x = area.width.saturating_sub(msg.len() as u16) / 2;
        let y = area.height / 2;
        f.buffer_mut().set_string(x, y, msg, THEME.brass());
        let _ = (Paragraph::new(Line::from(Span::raw(""))), area); // suppress unused import
    })?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    Ok(())
}
