//! Palette — the single most load-bearing aesthetic decision in the project.
//!
//! Goal: a piece of recovered industrial equipment that someone has cleaned,
//! kept working, and grown fond of. Brass, not gold. Oxidation, not damage.
//! Restraint above all. No gear emoji.
//!
//! Truecolor. 256-colour fallback: copper→166, brass→137, rust→130, tarnish→240, parchment→187.

use ratatui::style::{Color, Style};

pub struct Theme {
    pub background:    Color,
    pub surface:       Color,
    pub foreground:    Color,
    pub foreground_dim: Color,
    pub copper:        Color,
    pub copper_bright: Color,
    pub brass:         Color,
    pub rust:          Color,
    pub tarnish:       Color,
    pub verdigris:     Color,
}

pub const THEME: Theme = Theme {
    background:     Color::Rgb(26,  20,  16),   // #1a1410 — deep coffee / oil-stained wood
    surface:        Color::Rgb(34,  26,  21),   // #221a15 — inner panels
    foreground:     Color::Rgb(212, 184, 150),  // #d4b896 — aged parchment — body text
    foreground_dim: Color::Rgb(138, 117, 89),   // #8a7559 — secondary text
    copper:         Color::Rgb(200, 117, 51),   // #c87533 — active station, current frequency
    copper_bright:  Color::Rgb(232, 145, 73),   // #e89149 — new-broadcast flash (one frame)
    brass:          Color::Rgb(139, 111, 63),   // #8b6f3f — signal indicators, frame highlights
    rust:           Color::Rgb(160, 82,  45),   // #a0522d — warnings, off-air indicators
    tarnish:        Color::Rgb(90,  74,  58),   // #5a4a3a — inactive stations, keybindings
    verdigris:      Color::Rgb(74,  107, 90),   // #4a6b5a — fresh broadcast badge (use sparingly)
};

impl Theme {
    pub fn bg(&self) -> Style {
        Style::new().bg(self.background)
    }
    pub fn bg_style(&self) -> Style {
        Style::new().bg(self.background)
    }
    pub fn fg(&self) -> Style {
        Style::new().fg(self.foreground).bg(self.background)
    }
    pub fn fg_dim(&self) -> Style {
        Style::new().fg(self.foreground_dim).bg(self.background)
    }
    pub fn copper(&self) -> Style {
        Style::new().fg(self.copper).bg(self.background)
    }
    pub fn copper_bright(&self) -> Style {
        Style::new().fg(self.copper_bright).bg(self.background)
    }
    pub fn brass(&self) -> Style {
        Style::new().fg(self.brass).bg(self.background)
    }
    pub fn rust_warn(&self) -> Style {
        Style::new().fg(self.rust).bg(self.background)
    }
    pub fn tarnish(&self) -> Style {
        Style::new().fg(self.tarnish).bg(self.background)
    }
    pub fn verdigris(&self) -> Style {
        Style::new().fg(self.verdigris).bg(self.background)
    }
    /// Highlight style for the active station row.
    pub fn active_station(&self) -> Style {
        Style::new().fg(self.foreground).bg(self.copper)
    }
}
