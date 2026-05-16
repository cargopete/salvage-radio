//! Single-row status bar at the very bottom: keybindings in tarnish.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::theme::THEME;

pub struct StatusBar;

const KEYBINDS: &str =
    " [←/→] tune  [↑/↓] select  [space/b] scroll body  [enter] open  [q] off ";

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, THEME.tarnish());
        buf.set_string(area.left(), area.top(), KEYBINDS, THEME.tarnish());
    }
}
