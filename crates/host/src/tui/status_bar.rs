//! Single-row status bar at the very bottom: keybindings in tarnish.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

pub struct StatusBar;

const KEYBINDS: &str =
    " [←/→] tune  [↑/↓] scroll  [ [ / ] ] prev/next  [enter] open  [r] refresh  [q] off  [?] help ";

impl Widget for StatusBar {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO M3: fill area with THEME.background, render KEYBINDS in tarnish style.
        let _ = (KEYBINDS, &THEME);
    }
}
