use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// Prover statistics for TUI display
#[derive(Debug, Clone, Default)]
pub struct ProverStats {
    pub jobs_pending: u32,
    pub jobs_claimed: u32,
    pub jobs_completed: u32,
    pub jobs_failed: u32,
    pub total_earnings_lamports: u64,
    pub reputation_score: u16,
    pub avg_proof_time_secs: f64,
    pub uptime_secs: u64,
    pub last_job_id: Option<u64>,
    pub last_job_status: Option<String>,
    pub rpc_latency_ms: u64,
    pub current_block: u64,
}

/// Recent job entry for display
#[derive(Debug, Clone)]
pub struct RecentJob {
    pub id: u64,
    pub job_type: String,
    pub status: String,
    pub duration_secs: f64,
    pub earnings_lamports: u64,
}

/// Shared state between prover and TUI
pub struct TUIState {
    pub stats: Arc<Mutex<ProverStats>>,
    pub recent_jobs: Arc<Mutex<Vec<RecentJob>>>,
    pub should_quit: Arc<AtomicBool>,
}

impl TUIState {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(ProverStats::default())),
            recent_jobs: Arc::new(Mutex::new(Vec::new())),
            should_quit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut ProverStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            updater(&mut *stats);
        }
    }

    pub fn add_recent_job(&self, job: RecentJob) {
        if let Ok(mut jobs) = self.recent_jobs.lock() {
            jobs.insert(0, job);
            if jobs.len() > 10 {
                jobs.truncate(10);
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    pub fn set_quit(&self) {
        self.should_quit.store(true, Ordering::Relaxed);
    }
}

/// TUI application
pub struct TUIApp {
    state: Arc<TUIState>,
    start_time: Instant,
}

impl TUIApp {
    pub fn new(state: Arc<TUIState>) -> Self {
        Self {
            state,
            start_time: Instant::now(),
        }
    }

    /// Run the TUI event loop
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Update uptime
            let uptime_secs = self.start_time.elapsed().as_secs();
            self.state.update_stats(|stats| {
                stats.uptime_secs = uptime_secs;
            });

            // Render
            terminal.draw(|f| self.render(f))?;

            // Handle input with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.state.set_quit();
                            break;
                        }
                        _ => {}
                    }
                }
            }

            // Check if should quit (from external signal)
            if self.state.should_quit() {
                break;
            }
        }
        Ok(())
    }

    /// Render the TUI
    fn render(&self, f: &mut Frame) {
        let size = f.size();

        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(9),  // Stats panel
                Constraint::Min(0),     // Job list
                Constraint::Length(3),  // Footer
            ])
            .split(size);

        // Header
        self.render_header(f, chunks[0]);

        // Stats panel
        self.render_stats(f, chunks[1]);

        // Recent jobs list
        self.render_jobs(f, chunks[2]);

        // Footer
        self.render_footer(f, chunks[3]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new(" 🔐 Zyberlink Prover Node - LIVE ")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::White)));

        f.render_widget(title, area);
    }

    fn render_stats(&self, f: &mut Frame, area: Rect) {
        let stats = match self.state.stats.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                // Mutex poisoned (indicates a bug), but recover gracefully for demo
                eprintln!("⚠️  Stats mutex poisoned, recovering...");
                poisoned.into_inner().clone()
            }
        };

        // Split stats area into columns
        let stat_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status line
                Constraint::Length(3), // Jobs line
                Constraint::Length(3), // Performance line
            ])
            .split(area);

        // Status line
        let status_line = format!(
            " Status: {} | RPC: {}ms | Block: {} | Uptime: {}",
            if stats.jobs_claimed > 0 { "🟢 ACTIVE" } else { "🟡 IDLE" },
            stats.rpc_latency_ms,
            stats.current_block,
            format_duration(stats.uptime_secs)
        );

        let status = Paragraph::new(status_line)
            .style(Style::default().fg(Color::Green))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" System ")
                    .style(Style::default().fg(Color::White)),
            );
        f.render_widget(status, stat_chunks[0]);

        // Jobs line
        let jobs_text = vec![
            Line::from(vec![
                Span::raw(" Jobs:  "),
                Span::styled(
                    format!("{} ", stats.jobs_pending),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("pending | "),
                Span::styled(
                    format!("{} ", stats.jobs_claimed),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("claimed | "),
                Span::styled(
                    format!("{} ", stats.jobs_completed),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("done | "),
                Span::styled(
                    format!("{} ", stats.jobs_failed),
                    Style::default().fg(Color::Red),
                ),
                Span::raw("failed"),
            ]),
        ];

        let jobs = Paragraph::new(jobs_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Jobs ")
                .style(Style::default().fg(Color::White)),
        );
        f.render_widget(jobs, stat_chunks[1]);

        // Performance line
        let earnings_sol = stats.total_earnings_lamports as f64 / 1_000_000_000.0;
        let perf_text = vec![
            Line::from(vec![
                Span::raw(" Earnings: "),
                Span::styled(
                    format!("{:.4} SOL", earnings_sol),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | Reputation: "),
                Span::styled(
                    format!("{}/1000", stats.reputation_score),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" | Avg Time: "),
                Span::styled(
                    format!("{:.1}s", stats.avg_proof_time_secs),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
        ];

        let perf = Paragraph::new(perf_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Performance ")
                .style(Style::default().fg(Color::White)),
        );
        f.render_widget(perf, stat_chunks[2]);
    }

    fn render_jobs(&self, f: &mut Frame, area: Rect) {
        let jobs = match self.state.recent_jobs.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                // Mutex poisoned (indicates a bug), but recover gracefully for demo
                eprintln!("⚠️  Jobs mutex poisoned, recovering...");
                poisoned.into_inner().clone()
            }
        };

        let items: Vec<ListItem> = jobs
            .iter()
            .map(|job| {
                let status_icon = match job.status.as_str() {
                    "completed" => "✅",
                    "failed" => "❌",
                    "claimed" => "🔄",
                    "pending" => "⏸",
                    _ => "•",
                };

                let earnings_sol = job.earnings_lamports as f64 / 1_000_000_000.0;

                let line = format!(
                    " {} #{:<5} {:<12} {:.1}s  +{:.4} SOL",
                    status_icon, job.id, job.job_type, job.duration_secs, earnings_sol
                );

                let style = match job.status.as_str() {
                    "completed" => Style::default().fg(Color::Green),
                    "failed" => Style::default().fg(Color::Red),
                    "claimed" => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::Gray),
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let jobs_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Recent Jobs (last 10) ")
                .style(Style::default().fg(Color::White)),
        );

        f.render_widget(jobs_list, area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(" Press 'q' or ESC to quit ")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::White)));

        f.render_widget(footer, area);
    }
}

/// Format duration in human-readable format
fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Setup terminal for TUI
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal mode
pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
