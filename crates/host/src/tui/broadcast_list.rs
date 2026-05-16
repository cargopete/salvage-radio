//! Broadcast list + body preview: the right-hand panel.
//!
//! Top: scrollable list of received broadcasts.
//! Bottom: body of the selected item, word-wrapped.
//!
//! ↑/↓ moves selection. Enter opens the selected permalink in a browser.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

use super::theme::THEME;

pub struct BroadcastRow<'a> {
    pub title:  &'a str,
    pub source: &'a str,
    pub time:   &'a str,
    pub body:   &'a str,
}

pub struct BroadcastList<'a> {
    pub callsign:  &'a str,
    pub frequency: &'a str,
    pub items:     &'a [BroadcastRow<'a>],
    pub selected:  usize,
    pub flash:     bool,
    pub body_scroll: u16,
}

impl Widget for BroadcastList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, THEME.fg());

        if area.height < 2 {
            return;
        }

        // ── Header ───────────────────────────────────────────────────────────
        let header = format!(" NOW BROADCASTING ── {} ── {} ", self.callsign, self.frequency);
        let fill = "─".repeat((area.width as usize).saturating_sub(header.len()));
        let header_line = Line::from(vec![
            Span::styled(header, THEME.brass()),
            Span::styled(fill, THEME.tarnish()),
        ]);
        buf.set_line(area.left(), area.top(), &header_line, area.width);

        if area.height < 4 {
            return;
        }

        // ── Layout: list (top) + separator + body (bottom) ───────────────────
        // List gets up to 40% of available height (min 3 rows), body gets the rest.
        let content_area = Rect { y: area.top() + 1, height: area.height - 1, ..area };
        let list_rows = ((content_area.height as usize * 2 / 5).max(3).min(15)) as u16;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(list_rows),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(content_area);

        let list_area = chunks[0];
        let sep_area  = chunks[1];
        let body_area = chunks[2];

        // ── Empty state ───────────────────────────────────────────────────────
        if self.items.is_empty() {
            let waiting = Line::from(Span::styled(
                " ── waiting for broadcast ───",
                THEME.tarnish(),
            ));
            buf.set_line(list_area.left(), list_area.top(), &waiting, list_area.width);
            return;
        }

        // ── Item list ─────────────────────────────────────────────────────────
        let list_height = list_area.height as usize;
        let top = if self.selected >= list_height {
            self.selected - list_height + 1
        } else {
            0
        };
        let visible = &self.items[top..self.items.len().min(top + list_height)];

        let right_width: usize = 26;
        let title_width = (list_area.width as usize).saturating_sub(right_width + 4);

        for (i, row) in visible.iter().enumerate() {
            let y = list_area.top() + i as u16;
            let abs_idx = top + i;
            let is_selected = abs_idx == self.selected;

            let item_style = if is_selected && self.flash {
                THEME.copper_bright()
            } else if is_selected {
                THEME.copper()
            } else {
                THEME.fg_dim()
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            let title = truncate(row.title, title_width);
            let right_raw = format!("{} · {}", truncate(row.source, 12), row.time);
            let right = format!("{:>width$}", right_raw, width = right_width);
            let gap = " ".repeat(
                (list_area.width as usize).saturating_sub(prefix.len() + title.len() + right.len()),
            );

            let line = Line::from(vec![
                Span::styled(prefix, item_style),
                Span::styled(title.to_string(), item_style),
                Span::styled(gap, THEME.bg_style()),
                Span::styled(right, THEME.fg_dim()),
            ]);
            buf.set_line(list_area.left(), y, &line, list_area.width);
        }

        // ── Separator ─────────────────────────────────────────────────────────
        let sep_line = Line::from(Span::styled(
            "─".repeat(sep_area.width as usize),
            THEME.tarnish(),
        ));
        buf.set_line(sep_area.left(), sep_area.top(), &sep_line, sep_area.width);

        // ── Body pane ─────────────────────────────────────────────────────────
        if let Some(item) = self.items.get(self.selected) {
            if item.body.is_empty() {
                let dim = Line::from(Span::styled(" ── no summary available ───", THEME.tarnish()));
                buf.set_line(body_area.left(), body_area.top(), &dim, body_area.width);
            } else {
                let mut lines: Vec<Line> = item.body
                    .lines()
                    .map(|l| Line::from(Span::styled(format!(" {l}"), THEME.fg())))
                    .collect();
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!(" {}", "─".repeat((body_area.width as usize).saturating_sub(2))),
                    THEME.tarnish(),
                )));
                Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .scroll((self.body_scroll, 0))
                    .render(body_area, buf);
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max.saturating_sub(1);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
