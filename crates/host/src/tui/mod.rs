//! TUI layer — ratatui + crossterm.
//!
//! Main thread only (crossterm requirement).
//! Receives Events from the scheduler via mpsc, re-renders on each event.

pub mod dial;
pub mod now_playing;
pub mod station_list;
pub mod status_bar;
pub mod theme;
pub mod warmup;

// TODO M3: App struct — holds UI state (active station, broadcast buffer, scroll offsets)
// TODO M3: run(rx: mpsc::Receiver<Event>) -> Result<()>
//          - enter raw mode, alternate screen
//          - warmup::play()
//          - event loop: crossterm key events + scheduler Events
//          - on quit: quit animation, restore terminal
