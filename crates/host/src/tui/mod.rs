//! TUI layer — ratatui + crossterm.
//!
//! Main thread only (crossterm requirement).
//! Receives Events from the scheduler via mpsc, re-renders on each event.

pub mod broadcast_list;
pub mod dial;
pub mod now_playing;
pub mod station_list;
pub mod status_bar;
pub mod theme;
pub mod warmup;

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::{
    event::{Event as CEvent, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use tokio::sync::mpsc;

use crate::scheduler::Event;
use broadcast_list::{BroadcastList, BroadcastRow};
use dial::{Dial, DialStation};
use station_list::{StationEntry, StationList, StationStatus};
use status_bar::StatusBar;
use theme::THEME;

/// Metadata about a loaded station, passed into the TUI at startup.
pub struct StationMeta {
    pub callsign:  String,
    pub name:      String,
    pub frequency: String,
}

pub struct BroadcastEntry {
    pub id:        String,
    pub title:     String,
    pub body:      String,
    pub source:    String,
    pub permalink: String,
    pub published: u64,
    pub tags:      Vec<String>,
}

struct AppStation {
    callsign:  String,
    frequency: String,
    status:    StationStatus,
    history:   Vec<BroadcastEntry>, // index 0 = most recent
}

struct App {
    stations:    Vec<AppStation>,
    active_idx:  usize,
    selected:    usize, // selected item index within active station's history
    body_scroll: u16,   // scroll offset for the body pane
    flash:       bool,
}

impl App {
    fn new(meta: Vec<StationMeta>) -> Self {
        let stations = meta
            .into_iter()
            .map(|m| AppStation {
                callsign:  m.callsign,
                frequency: m.frequency,
                status:    StationStatus::Static,
                history:   Vec::new(),
            })
            .collect();
        Self { stations, active_idx: 0, selected: 0, body_scroll: 0, flash: false }
    }

    fn tune_next(&mut self) {
        if self.active_idx + 1 < self.stations.len() {
            self.active_idx += 1;
        }
        self.reset_view();
    }

    fn tune_prev(&mut self) {
        if self.active_idx > 0 {
            self.active_idx -= 1;
        }
        self.reset_view();
    }

    fn reset_view(&mut self) {
        self.selected = 0;
        self.body_scroll = 0;
    }

    fn select_next(&mut self) {
        let hist_len = self.stations[self.active_idx].history.len();
        if self.selected + 1 < hist_len {
            self.selected += 1;
            self.body_scroll = 0;
        }
    }

    fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.body_scroll = 0;
        }
    }

    fn current_permalink(&self) -> Option<&str> {
        self.stations
            .get(self.active_idx)?
            .history
            .get(self.selected)
            .map(|e| e.permalink.as_str())
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Broadcast { callsign, id, title, body, source, permalink, published, tags } => {
                if let Some(st) = self.stations.iter_mut().find(|s| s.callsign == callsign) {
                    st.status = StationStatus::OnAir;
                    st.history.insert(0, BroadcastEntry { id, title, body, source, permalink, published, tags });
                    st.history.sort_by_key(|b| std::cmp::Reverse(b.published));
                    st.history.truncate(50);
                }
            }
            Event::Quiet(callsign) => {
                if let Some(st) = self.stations.iter_mut().find(|s| s.callsign == callsign) {
                    if st.status != StationStatus::OnAir {
                        st.status = StationStatus::Static;
                    }
                }
            }
            Event::OffAir { callsign, .. } | Event::Error { callsign, .. } => {
                if let Some(st) = self.stations.iter_mut().find(|s| s.callsign == callsign) {
                    st.status = StationStatus::OffAir;
                }
            }
        }
    }
}

pub async fn run(mut rx: mpsc::Receiver<Event>, meta: Vec<StationMeta>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(meta);
    let mut events = EventStream::new();

    // ── Warmup animation ─────────────────────────────────────────────────────
    {
        let dial_stations: Vec<DialStation> = app.stations.iter().map(|s| DialStation {
            callsign:  s.callsign.clone(),
            frequency: s.frequency.clone(),
        }).collect();
        let list_entries: Vec<StationEntry> = app.stations.iter().map(|s| StationEntry {
            callsign: s.callsign.clone(),
            status:   s.status,
        }).collect();
        warmup::play(&mut terminal, &dial_stations, &list_entries, app.active_idx).await?;
    }

    // First full render after warmup.
    draw(&mut terminal, &app)?;

    // ── Main event loop ───────────────────────────────────────────────────────
    // flash_until: when set, we're in the one-frame copper_bright flash window.
    let mut flash_until: Option<Instant> = None;

    loop {
        // Schedule a wake-up to clear the flash, if one is pending.
        let flash_deadline = flash_until.map(|t| tokio::time::sleep_until(t.into()));

        tokio::select! {
            Some(Ok(event)) = events.next() => {
                match event {
                    CEvent::Key(k) => match k.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                        KeyCode::Enter | KeyCode::Char('o') => {
                            if let Some(url) = app.current_permalink() {
                                open_url(url);
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => app.tune_next(),
                        KeyCode::Left  | KeyCode::Char('h') => app.tune_prev(),
                        KeyCode::Down  | KeyCode::Char('j') => app.select_next(),
                        KeyCode::Up    | KeyCode::Char('k') => app.select_prev(),
                        KeyCode::Char(' ') | KeyCode::PageDown => {
                            app.body_scroll = app.body_scroll.saturating_add(3);
                        }
                        KeyCode::Char('b') | KeyCode::PageUp => {
                            app.body_scroll = app.body_scroll.saturating_sub(3);
                        }
                        _ => continue,
                    }
                    CEvent::Resize(_, _) => {} // fall through to redraw
                    _ => continue,
                }
                draw(&mut terminal, &app)?;
            }

            Some(event) = rx.recv() => {
                // Check if this broadcast is for the currently tuned station.
                let for_active = matches!(&event, Event::Broadcast { callsign, .. }
                    if *callsign == app.stations[app.active_idx].callsign);

                app.apply_event(event);

                if for_active {
                    // One-frame copper_bright flash.
                    app.flash = true;
                    draw(&mut terminal, &app)?;
                    flash_until = Some(Instant::now() + Duration::from_millis(80));
                } else {
                    draw(&mut terminal, &app)?;
                }
            }

            // Flash timeout: clear the copper_bright and redraw normally.
            Some(_) = async { match flash_deadline { Some(f) => { f.await; Some(()) } None => None } } => {
                app.flash = false;
                flash_until = None;
                draw(&mut terminal, &app)?;
            }
        }
    }

    // ── Quit animation ────────────────────────────────────────────────────────
    warmup::sign_off(&mut terminal).await?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &App) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        f.render_widget(
            ratatui::widgets::Block::default().style(THEME.bg()),
            size,
        );

        // Vertical: [content | dial(3) | status(1)]
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(size);

        // Horizontal: [station_list(14) | now_playing(rest)]
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(14), Constraint::Min(0)])
            .split(v[0]);

        // Station list
        let entries: Vec<StationEntry> = app.stations.iter().map(|s| StationEntry {
            callsign: s.callsign.clone(),
            status:   s.status,
        }).collect();
        f.render_widget(
            StationList { stations: &entries, active_idx: app.active_idx },
            h[0],
        );

        // Broadcast list
        let active = &app.stations[app.active_idx];
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let time_strs: Vec<String> = active.history.iter()
            .map(|b| relative_time(b.published, now))
            .collect();
        let rows: Vec<BroadcastRow> = active.history.iter().zip(&time_strs).map(|(b, t)| BroadcastRow {
            title:  b.title.as_str(),
            source: b.source.as_str(),
            time:   t.as_str(),
            body:   b.body.as_str(),
        }).collect();
        f.render_widget(
            BroadcastList {
                callsign:    &active.callsign,
                frequency:   &active.frequency,
                items:       &rows,
                selected:    app.selected,
                flash:       app.flash,
                body_scroll: app.body_scroll,
            },
            h[1],
        );

        // Dial
        let dial_stations: Vec<DialStation> = app.stations.iter().map(|s| DialStation {
            callsign:  s.callsign.clone(),
            frequency: s.frequency.clone(),
        }).collect();
        f.render_widget(
            Dial { stations: &dial_stations, active_idx: app.active_idx, scroll_offset: 0 },
            v[1],
        );

        // Status bar
        f.render_widget(StatusBar, v[2]);
    })?;
    Ok(())
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let _ = std::process::Command::new("start").arg(url).spawn();
}

fn relative_time(published: u64, now: u64) -> String {
    let diff = now.saturating_sub(published);
    if diff < 60      { "just now".to_string() }
    else if diff < 3600  { format!("{} min ago",  diff / 60) }
    else if diff < 86400 { format!("{} hr ago",   diff / 3600) }
    else                 { format!("{} days ago", diff / 86400) }
}

