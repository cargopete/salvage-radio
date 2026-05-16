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
use dial::{Dial, DialStation};
use now_playing::NowPlaying;
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
    stations:   Vec<AppStation>,
    active_idx: usize,
    scroll:     u16,
    buf_idx:    usize, // 0 = latest, 1 = one before, etc.
    flash:      bool,
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
        Self { stations, active_idx: 0, scroll: 0, buf_idx: 0, flash: false }
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
        self.scroll = 0;
        self.buf_idx = 0;
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Broadcast { callsign, id, title, body, source, permalink, published, tags } => {
                if let Some(st) = self.stations.iter_mut().find(|s| s.callsign == callsign) {
                    st.status = StationStatus::OnAir;
                    st.history.insert(0, BroadcastEntry { id, title, body, source, permalink, published, tags });
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
                        KeyCode::Right | KeyCode::Char('l') => app.tune_next(),
                        KeyCode::Left  | KeyCode::Char('h') => app.tune_prev(),
                        KeyCode::Down  | KeyCode::Char('j') => {
                            app.scroll = app.scroll.saturating_add(3);
                        }
                        KeyCode::Up    | KeyCode::Char('k') => {
                            app.scroll = app.scroll.saturating_sub(3);
                        }
                        KeyCode::Char('[') => {
                            let hist_len = app.stations[app.active_idx].history.len();
                            if app.buf_idx + 1 < hist_len {
                                app.buf_idx += 1;
                                app.scroll = 0;
                            }
                        }
                        KeyCode::Char(']') => {
                            if app.buf_idx > 0 {
                                app.buf_idx -= 1;
                                app.scroll = 0;
                            }
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

        // Now playing
        let active = &app.stations[app.active_idx];
        let broadcast = active.history.get(app.buf_idx);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let rel_time = broadcast.map(|b| relative_time(b.published, now));
        let read_time = broadcast.map(|b| estimate_read_time(&b.body));
        f.render_widget(
            NowPlaying {
                callsign:      &active.callsign,
                frequency:     &active.frequency,
                title:         broadcast.map(|b| b.title.as_str()),
                body:          broadcast.map(|b| b.body.as_str()),
                source:        broadcast.map(|b| b.source.as_str()),
                relative_time: rel_time.as_deref(),
                read_time:     read_time.as_deref(),
                tags:          broadcast.map(|b| b.tags.as_slice()).unwrap_or(&[]),
                scroll_offset: app.scroll,
                flash:         app.flash,
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

fn relative_time(published: u64, now: u64) -> String {
    let diff = now.saturating_sub(published);
    if diff < 60      { "just now".to_string() }
    else if diff < 3600  { format!("{} min ago",  diff / 60) }
    else if diff < 86400 { format!("{} hr ago",   diff / 3600) }
    else                 { format!("{} days ago", diff / 86400) }
}

fn estimate_read_time(body: &str) -> String {
    let words = body.split_whitespace().count();
    let mins = ((words as f32 / 200.0).ceil() as u32).max(1);
    if mins == 1 { "1 min read".to_string() } else { format!("{mins} min read") }
}
