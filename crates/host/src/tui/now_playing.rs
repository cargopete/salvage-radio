//! Now-playing panel: the right-hand main content area.
//!
//! Header:  "NOW BROADCASTING ── {CALLSIGN} ── {FREQ}"
//! Title:   copper (copper_bright on new-broadcast flash — M4)
//! Body:    parchment, word-wrapped, scrollable via Paragraph
//! Footer:  source · relative_time · read_time

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

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
    /// True for ~one render frame on new broadcast arrival (M4).
    pub flash:         bool,
}

impl Widget for NowPlaying<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, THEME.fg());

        if area.height < 3 {
            return;
        }

        // Split: [header=1] [title=1] [gap=1] [body=fill] [footer=1]
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(1), // title
                Constraint::Length(1), // gap / separator
                Constraint::Min(0),    // body
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Header
        let header = format!(
            " NOW BROADCASTING ── {} ── {} ",
            self.callsign, self.frequency
        );
        let header_line = Line::from(vec![
            Span::styled(&header, THEME.brass()),
            Span::styled(
                "─".repeat(area.width.saturating_sub(header.len() as u16) as usize),
                THEME.tarnish(),
            ),
        ]);
        Paragraph::new(Text::from(header_line))
            .style(Style::default())
            .render(chunks[0], buf);

        // Title
        let title_style = if self.flash { THEME.copper_bright() } else { THEME.copper() };
        let title_text = self.title.unwrap_or("─── waiting for broadcast ───");
        Paragraph::new(Text::from(Line::from(Span::styled(
            format!(" {title_text}"),
            title_style,
        ))))
        .render(chunks[1], buf);

        // Separator line
        buf.set_string(
            chunks[2].left(),
            chunks[2].top(),
            " ".repeat(chunks[2].width as usize),
            THEME.fg(),
        );

        // Body
        if let Some(body) = self.body {
            let mut lines: Vec<Line> = body
                .lines()
                .map(|l| Line::from(Span::styled(format!(" {l}"), THEME.fg())))
                .collect();
            // End-of-transmission marker — visually closes short content
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" {}", "─".repeat((chunks[3].width as usize).saturating_sub(2))),
                THEME.tarnish(),
            )));
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: false })
                .scroll((self.scroll_offset, 0))
                .render(chunks[3], buf);
        }

        // Footer
        let mut parts: Vec<&str> = Vec::new();
        if let Some(s) = self.source { parts.push(s); }
        if let Some(t) = self.relative_time { parts.push(t); }
        if let Some(r) = self.read_time { parts.push(r); }
        if !parts.is_empty() {
            let footer = format!(" {} ", parts.join(" · "));
            Paragraph::new(Text::from(Line::from(Span::styled(footer, THEME.fg_dim()))))
                .render(chunks[4], buf);
        }
    }
}
