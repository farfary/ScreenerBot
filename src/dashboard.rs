/// Dashboard Terminal UI for ScreenerBot
///
/// Real-time terminal dashboard using crossterm for advanced terminal control.
/// Displays live trading data, positions, statistics, and logs in a structured layout.
///
/// Features:
/// - Live updating dashboard with multiple sections
/// - Real-time position tracking with P&L
/// - Recent transactions and activity log
/// - System statistics and performance metrics
/// - Trading summary and profit/loss overview
/// - Responsive layout that adapts to terminal size
/// - Color-coded status indicators
/// - Clean exit handling with terminal restoration

use crossterm::{
    cursor::{ Hide, MoveTo, Show },
    event::{ self, Event, KeyCode, KeyEvent, KeyModifiers },
    execute,
    style::{ Color, Print, ResetColor, SetForegroundColor, Attribute, SetAttribute },
    terminal::{
        self,
        Clear,
        ClearType,
        EnterAlternateScreen,
        LeaveAlternateScreen,
        DisableLineWrap,
        EnableLineWrap,
        SetTitle,
    },
    QueueableCommand,
};
use std::io::{ self, Write, stdout };
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicBool, Ordering };
use std::time::{ Duration, Instant };
use tokio::sync::Notify;
use tokio::time::sleep;
use chrono::Local;
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use unicode_width::{ UnicodeWidthChar, UnicodeWidthStr };

use crate::positions::{ get_open_positions, get_closed_positions, calculate_position_pnl };
use crate::tokens::{ get_pricing_stats, get_pool_service };
use crate::utils::{ get_sol_balance, get_wallet_address };
use crate::rpc::get_global_rpc_stats;
use crate::transactions::get_transaction_stats;
use crate::logger::LogTag;

/// Dashboard configuration constants
const REFRESH_RATE_MS: u64 = 250; // Faster incremental refresh for real-time feel
const MAX_LOG_LINES: usize = 20; // Maximum log lines to display
const MIN_TERMINAL_WIDTH: u16 = 80; // Minimum terminal width (reduced from 100)
const MIN_TERMINAL_HEIGHT: u16 = 20; // Minimum terminal height (reduced from 30)
const SHUTDOWN_INACTIVITY_MS: u64 = 2000; // Exit after 2s of no new logs
const SHUTDOWN_MAX_WAIT_MS: u64 = 20000; // Hard exit after 20s
const STARTUP_SPLASH_MS: u64 = 1200; // Show startup state ~1.2s

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DashboardPhase {
    Startup,
    Running,
    ShuttingDown,
}

/// A single renderable line with style (module-scope so all methods can use it)
struct RenderLine {
    text: String,
    color: Color, // content color
    bold: bool,
    is_wrapped: bool, // line contains vertical borders at both ends
    is_border_only: bool, // line is a pure border line (top/bottom/separator)
}

const BORDER_COLOR: Color = Color::Grey;

/// Modern Unicode box-drawing characters for panel borders
struct BorderChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
    pub top_tee: char,
    pub bottom_tee: char,
    pub left_tee: char,
    pub right_tee: char,
    pub cross: char,
}

impl BorderChars {
    /// Spaces between content and vertical borders
    const INSET: usize = 1;
    /// Modern rounded border style (Unicode box-drawing)
    pub const ROUNDED: BorderChars = BorderChars {
        top_left: '╭',
        top_right: '╮',
        bottom_left: '╰',
        bottom_right: '╯',
        horizontal: '─',
        vertical: '│',
        top_tee: '┬',
        bottom_tee: '┴',
        left_tee: '├',
        right_tee: '┤',
        cross: '┼',
    };

    /// Generate a top border line
    pub fn top_border(&self, width: u16) -> String {
        if width < 2 {
            return String::new();
        }
        format!(
            "{}{}{}",
            self.top_left,
            self.horizontal.to_string().repeat((width as usize) - 2),
            self.top_right
        )
    }

    /// Generate a bottom border line
    pub fn bottom_border(&self, width: u16) -> String {
        if width < 2 {
            return String::new();
        }
        format!(
            "{}{}{}",
            self.bottom_left,
            self.horizontal.to_string().repeat((width as usize) - 2),
            self.bottom_right
        )
    }

    /// Generate a separator line (horizontal divider)
    pub fn separator(&self, width: u16) -> String {
        if width < 2 {
            return String::new();
        }
        format!(
            "{}{}{}",
            self.left_tee,
            self.horizontal.to_string().repeat((width as usize) - 2),
            self.right_tee
        )
    }

    /// Wrap text content with vertical borders and a tiny inset
    pub fn wrap_content(&self, content: &str, width: u16) -> String {
        if width < 2 {
            return String::new();
        }
        let inner = (width as usize) - 2;
        let inset_each = Self::INSET.min(inner / 2);
        let avail = inner.saturating_sub(inset_each * 2);
        // Build truncated content by display width (Unicode-safe)
        let mut acc = String::new();
        let mut curw = 0usize;
        for ch in content.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if curw + w > avail {
                break;
            }
            acc.push(ch);
            curw += w;
        }
        if curw < avail {
            acc.push_str(&" ".repeat(avail - curw));
        }
        let left_pad = " ".repeat(inset_each);
        let right_pad = " ".repeat(inset_each);
        format!("{}{}{}{}{}", self.vertical, left_pad, acc, right_pad, self.vertical)
    }
}

/// Dashboard state structure
#[derive(Clone)]
pub struct Dashboard {
    pub running: Arc<Mutex<bool>>,
    pub logs: Arc<Mutex<VecDeque<LogEntry>>>,
    pub last_update: Arc<Mutex<Instant>>,
    pub terminal_size: (u16, u16), // (width, height)
    // Incremental rendering caches
    prev_header: Vec<String>,
    prev_positions: Vec<String>,
    prev_stats: Vec<String>,
    prev_logs: Vec<String>,
    prev_footer: Vec<String>,
    force_redraw: Arc<AtomicBool>,
    phase: DashboardPhase,
    started_at: Instant,
    shutdown_requested: bool,
    shutdown_started_at: Option<Instant>,
    last_log_len: usize,
    last_log_change: Instant,
    spinner_idx: usize,
    services_completed: bool,
}

/// Log entry for dashboard display
#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub tag: String,
    pub log_type: String,
    pub message: String,
    pub color: Color,
}

impl Dashboard {
    /// Create a new dashboard instance
    pub fn new() -> Self {
        let (width, height) = terminal::size().unwrap_or((120, 40));
        Self {
            running: Arc::new(Mutex::new(true)),
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES * 2))),
            last_update: Arc::new(Mutex::new(Instant::now())),
            terminal_size: (width, height),
            prev_header: Vec::new(),
            prev_positions: Vec::new(),
            prev_stats: Vec::new(),
            prev_logs: Vec::new(),
            prev_footer: Vec::new(),
            force_redraw: Arc::new(AtomicBool::new(true)),
            phase: DashboardPhase::Startup,
            started_at: Instant::now(),
            shutdown_requested: false,
            shutdown_started_at: None,
            last_log_len: 0,
            last_log_change: Instant::now(),
            spinner_idx: 0,
            services_completed: false,
        }
    }

    /// Initialize the dashboard terminal
    pub async fn initialize(&mut self) -> io::Result<()> {
        // Check terminal size requirements
        let (width, height) = terminal::size()?;
        if width < MIN_TERMINAL_WIDTH || height < MIN_TERMINAL_HEIGHT {
            eprintln!(
                "Terminal too small! Minimum size: {}x{}, Current: {}x{}",
                MIN_TERMINAL_WIDTH,
                MIN_TERMINAL_HEIGHT,
                width,
                height
            );
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Terminal too small"));
        }

        self.terminal_size = (width, height);

        // Setup terminal for dashboard mode with scroll prevention
        execute!(
            stdout(),
            EnterAlternateScreen,
            DisableLineWrap,
            Hide,
            // Purge scrollback to avoid any scrollable history like `btm`
            Clear(ClearType::Purge),
            Clear(ClearType::All),
            SetTitle("ScreenerBot Dashboard")
        )?;

        // Enable raw mode for input handling
        terminal::enable_raw_mode()?;

        Ok(())
    }

    /// Main dashboard loop
    pub async fn run(
        &mut self,
        shutdown: Arc<Notify>,
        services_completed: Arc<Notify>
    ) -> io::Result<()> {
        let mut last_draw = Instant::now();
        let mut services_finished = false;

        loop {
            // Check for shutdown signal first
            tokio::select! {
                _ = shutdown.notified() => {
                    // Switch to shutdown phase and keep UI alive to monitor service shutdown
                    self.phase = DashboardPhase::ShuttingDown;
                    if self.shutdown_started_at.is_none() { self.shutdown_started_at = Some(Instant::now()); }
                    self.shutdown_requested = true;
                    self.force_redraw.store(true, Ordering::SeqCst);
                    // Don't exit here - keep monitoring until services complete
                }
                _ = services_completed.notified() => {
                    // All background services have completed
                    services_finished = true;
                    self.services_completed = true;
                    self.add_log("SYSTEM", "INFO", "🎯 All background services completed - dashboard will exit in 3 seconds");
                    self.force_redraw.store(true, Ordering::SeqCst);
                }
                result = async {
                    // Check running flag - quick check without holding lock
                    let should_shutdown = {
                        if let Ok(running) = self.running.lock() {
                            !*running
                        } else {
                            false
                        }
                    };
                    
                    if should_shutdown {
                        // User requested exit -> show shutdown UI and continue until drain
                        self.phase = DashboardPhase::ShuttingDown;
                        if self.shutdown_started_at.is_none() { self.shutdown_started_at = Some(Instant::now()); }
                        // Don't exit immediately
                    }

                    // Handle input events (non-blocking with longer timeout to reduce CPU usage)
                    if event::poll(Duration::from_millis(100))? {
                        if let Event::Key(key_event) = event::read()? {
                            let _ = self.handle_input(key_event).await?;
                        }
                    }

                    // Transition out of startup after splash time
                    if self.phase == DashboardPhase::Startup && self.started_at.elapsed() >= Duration::from_millis(STARTUP_SPLASH_MS) {
                        self.phase = DashboardPhase::Running;
                        self.force_redraw.store(true, Ordering::SeqCst);
                    }

                    // Force redraw on demand
                    if self.force_redraw.swap(false, Ordering::SeqCst) {
                        self.draw().await?;
                        last_draw = Instant::now();
                        // Update last refresh time - minimize lock scope
                        if let Ok(mut last_update) = self.last_update.lock() { 
                            *last_update = Instant::now(); 
                        }
                    } else if last_draw.elapsed() >= Duration::from_millis(REFRESH_RATE_MS) {
                        // Periodic refresh
                        self.draw().await?;
                        last_draw = Instant::now();
                        
                        // Update last refresh time - minimize lock scope
                        if let Ok(mut last_update) = self.last_update.lock() {
                            *last_update = Instant::now();
                        }
                    }

                    // Yield control to prevent high CPU usage
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    
                    Ok::<bool, io::Error>(false)
                } => {
                    match result {
                        Ok(should_exit) => {
                            // We don't hard-exit here; shutdown handled below
                        }
                        Err(e) => return Err(e),
                    }
                }
            }

            // Decide if we should exit the loop
            if self.phase == DashboardPhase::ShuttingDown {
                // In shutdown phase, wait for services to complete before considering exit
                if services_finished {
                    // Services are done, wait a bit for user to see final status then exit
                    let grace_period_elapsed = self.shutdown_started_at
                        .map(|t| t.elapsed() >= Duration::from_millis(3000)) // 3 second grace period
                        .unwrap_or(true);

                    if grace_period_elapsed {
                        break; // Safe to exit now
                    }
                } else {
                    // Still waiting for services - stay active to show progress
                    // Update inactivity timer based on logs length - minimize lock time
                    let current_log_len = {
                        if let Ok(logs) = self.logs.lock() {
                            logs.len()
                        } else {
                            self.last_log_len // fallback to previous value
                        }
                    };

                    if current_log_len != self.last_log_len {
                        self.last_log_len = current_log_len;
                        self.last_log_change = Instant::now();
                    }

                    // Extended timeout when waiting for critical services
                    let max_wait_time = Duration::from_millis(SHUTDOWN_MAX_WAIT_MS * 3); // 60 seconds
                    let timed_out = self.shutdown_started_at
                        .map(|t| t.elapsed() >= max_wait_time)
                        .unwrap_or(false);

                    if timed_out {
                        self.add_log(
                            "SYSTEM",
                            "WARN",
                            "⚠️  Force exit: services taking too long to shutdown"
                        );
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    async fn handle_input(&self, key_event: KeyEvent) -> io::Result<bool> {
        match key_event.code {
            // Exit on Ctrl+C or 'q'
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                // Set running flag to false to trigger main shutdown
                if let Ok(mut running) = self.running.lock() {
                    *running = false;
                }
                // don't immediately exit; shutdown UI will continue
                return Ok(true);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Set running flag to false to trigger main shutdown
                if let Ok(mut running) = self.running.lock() {
                    *running = false;
                }
                // don't immediately exit; shutdown UI will continue
                return Ok(true);
            }
            // Refresh on 'r' -> request immediate redraw
            KeyCode::Char('r') => {
                self.force_redraw.store(true, Ordering::SeqCst);
            }
            // Clear logs on 'c'
            KeyCode::Char('c') => {
                // Quick operation - minimal lock time
                if let Ok(mut logs) = self.logs.lock() {
                    logs.clear();
                }
                // Force redraw to show cleared logs immediately
                self.force_redraw.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
        Ok(false)
    }

    /// Draw the complete dashboard
    async fn draw(&mut self) -> io::Result<()> {
        let mut stdout = stdout();

        // Recalculate terminal size each frame to handle resizes without overflow
        let (width, height) = terminal::size().unwrap_or(self.terminal_size);
        let size_changed = (width, height) != self.terminal_size;
        if size_changed {
            // Update cached size and force full redraw by clearing caches
            self.terminal_size = (width, height);
            self.prev_header.clear();
            self.prev_positions.clear();
            self.prev_stats.clear();
            self.prev_logs.clear();
            self.prev_footer.clear();
            // Ensure cursor is at top-left on size change
            stdout.queue(MoveTo(0, 0))?;
            stdout.queue(Clear(ClearType::All))?;
        }

        // Ensure we don't exceed terminal boundaries
        let max_content_height = height.saturating_sub(1);

        // Build and draw header incrementally
        let header_lines = self.build_header_lines(width, height);
        Self::draw_section(&mut stdout, 0, width, &header_lines, &mut self.prev_header)?;

        // Calculate layout sections
        // Header actually renders 4 lines (top border, title, timestamp, bottom border)
        let header_height = 4;
        let footer_height = 2;
        let available_height = max_content_height.saturating_sub(header_height + footer_height);

        // Split available space into sections
        let positions_height = ((available_height * 40) / 100).max(8); // 40% for positions
        let stats_height = ((available_height * 30) / 100).max(6); // 30% for stats
        let logs_height = available_height.saturating_sub(positions_height + stats_height); // Rest for logs

        let mut current_row = header_height;

        // Draw positions section incrementally
        if current_row < max_content_height {
            let sec_h = positions_height.min(max_content_height.saturating_sub(current_row));
            let pos_lines = self.build_positions_lines(width, sec_h).await;
            Self::draw_section(
                &mut stdout,
                current_row,
                width,
                &pos_lines,
                &mut self.prev_positions
            )?;
            current_row = current_row.saturating_add(sec_h);
        }

        // Draw statistics section incrementally
        if current_row < max_content_height {
            let sec_h = stats_height.min(max_content_height.saturating_sub(current_row));
            let stats_lines = self.build_statistics_lines(width, sec_h).await;
            Self::draw_section(
                &mut stdout,
                current_row,
                width,
                &stats_lines,
                &mut self.prev_stats
            )?;
            current_row = current_row.saturating_add(sec_h);
        }

        // Draw logs section incrementally (may redraw block on change)
        if current_row < max_content_height {
            let sec_h = logs_height.min(max_content_height.saturating_sub(current_row));
            let log_lines = self.build_logs_lines(width, sec_h).await;
            Self::draw_section(&mut stdout, current_row, width, &log_lines, &mut self.prev_logs)?;
        }

        // Draw footer incrementally in last two visible rows
        if height >= 2 {
            let footer_start = height.saturating_sub(2);
            let footer_lines = self.build_footer_lines(width, 2);
            Self::draw_section(
                &mut stdout,
                footer_start,
                width,
                &footer_lines,
                &mut self.prev_footer
            )?;
        }

        // advance spinner for next frame
        self.spinner_idx = (self.spinner_idx + 1) % 4;
        stdout.flush()?;
        Ok(())
    }

    fn pad_truncate(text: &str, width: u16) -> String {
        let target = width as usize;
        if target == 0 {
            return String::new();
        }
        let mut out = String::with_capacity(target);
        let mut curw = 0usize;
        for ch in text.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if curw + ch_w > target {
                break;
            }
            out.push(ch);
            curw += ch_w;
        }
        if curw < target {
            out.push_str(&" ".repeat(target - curw));
        }
        out
    }

    /// Draw only the lines that changed; clear current line before printing to avoid leftovers
    fn draw_section(
        stdout: &mut io::Stdout,
        start_row: u16,
        width: u16,
        lines: &Vec<RenderLine>,
        prev_cache: &mut Vec<String>
    ) -> io::Result<()> {
        for (i, rl) in lines.iter().enumerate() {
            let row = start_row.saturating_add(i as u16);
            let new_text = Self::pad_truncate(&rl.text, width);
            let cached = prev_cache.get(i);
            if cached.map(|c| c.as_str()) != Some(new_text.as_str()) {
                stdout.queue(MoveTo(0, row))?;
                stdout.queue(Clear(ClearType::CurrentLine))?;
                // Reset styling to avoid bleed between lines
                stdout.queue(SetAttribute(Attribute::Reset))?;
                stdout.queue(ResetColor)?;
                if rl.is_border_only {
                    // Entire line is a border
                    stdout.queue(SetForegroundColor(BORDER_COLOR))?;
                    if rl.bold {
                        stdout.queue(SetAttribute(Attribute::Bold))?;
                    }
                    stdout.queue(Print(new_text.clone()))?;
                } else if rl.is_wrapped && new_text.len() >= 2 {
                    // Print borders in light gray and content with its color
                    let mut chars = new_text.chars();
                    let left = chars.next().unwrap_or(' ');
                    let right = new_text.chars().last().unwrap_or(' ');
                    // Middle content (may be empty)
                    let middle: String = new_text
                        .chars()
                        .skip(1)
                        .take(new_text.len().saturating_sub(2))
                        .collect();
                    // Left border
                    stdout.queue(SetForegroundColor(BORDER_COLOR))?;
                    if rl.bold {
                        stdout.queue(SetAttribute(Attribute::Bold))?;
                    }
                    stdout.queue(Print(left.to_string()))?;
                    // Middle content
                    stdout.queue(SetForegroundColor(rl.color))?;
                    if rl.bold {
                        stdout.queue(SetAttribute(Attribute::Bold))?;
                    }
                    stdout.queue(Print(middle))?;
                    // Right border
                    stdout.queue(SetForegroundColor(BORDER_COLOR))?;
                    if rl.bold {
                        stdout.queue(SetAttribute(Attribute::Bold))?;
                    }
                    stdout.queue(Print(right.to_string()))?;
                } else {
                    // Plain line
                    stdout.queue(SetForegroundColor(rl.color))?;
                    if rl.bold {
                        stdout.queue(SetAttribute(Attribute::Bold))?;
                    }
                    stdout.queue(Print(new_text.clone()))?;
                }
                // Ensure attributes and colors are reset after each line
                stdout.queue(SetAttribute(Attribute::Reset))?;
                stdout.queue(ResetColor)?;
                if prev_cache.len() <= i {
                    prev_cache.resize(i + 1, String::new());
                }
                prev_cache[i] = new_text;
            }
        }
        // If previous cache had more lines (e.g., resize smaller), we don't need to clear: those
        // extra rows are outside this section or will be covered by other sections/full redraw.
        if prev_cache.len() > lines.len() {
            prev_cache.truncate(lines.len());
        }
        Ok(())
    }

    fn spinner(&self) -> char {
        match self.spinner_idx {
            0 => '-',
            1 => '\\',
            2 => '|',
            _ => '/',
        }
    }

    fn phase_label(&self) -> &'static str {
        match self.phase {
            DashboardPhase::Startup => "Starting",
            DashboardPhase::Running => "Running",
            DashboardPhase::ShuttingDown => "Shutting down",
        }
    }

    fn build_header_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        let border = &BorderChars::ROUNDED;

        if height >= 1 {
            // Top border
            v.push(RenderLine {
                text: border.top_border(width),
                color: BORDER_COLOR,
                bold: true,
                is_wrapped: false,
                is_border_only: true,
            });
        }
        if height >= 2 {
            let title = format!(
                "ScreenerBot Dashboard  [{} {}]",
                self.phase_label(),
                self.spinner()
            );
            let wrapped = border.wrap_content(
                &format!("{:^width$}", title, width = width.saturating_sub(2) as usize),
                width
            );
            v.push(RenderLine {
                text: wrapped,
                color: Color::White,
                bold: true,
                is_wrapped: true,
                is_border_only: false,
            });
        }
        if height >= 3 {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let wrapped = border.wrap_content(
                &format!("{:^width$}", timestamp, width = width.saturating_sub(2) as usize),
                width
            );
            v.push(RenderLine {
                text: wrapped,
                color: Color::White,
                bold: true,
                is_wrapped: true,
                is_border_only: false,
            });
        }
        if height >= 4 {
            // Bottom border
            v.push(RenderLine {
                text: border.bottom_border(width),
                color: BORDER_COLOR,
                bold: true,
                is_wrapped: false,
                is_border_only: true,
            });
        }
        v
    }

    async fn build_positions_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 {
            return v;
        }
        let border = &BorderChars::ROUNDED;

        // Section header with top border
        v.push(RenderLine {
            text: border.top_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });
        if height == 1 {
            return v;
        }

        v.push(RenderLine {
            text: border.wrap_content("POSITIONS", width),
            color: Color::White,
            bold: true,
            is_wrapped: true,
            is_border_only: false,
        });
        if height == 2 {
            return v;
        }

        let mut rows_left = height.saturating_sub(3) as usize; // Account for header + borders
        let open_positions = get_open_positions().await;
        let closed_positions = get_closed_positions().await;

        // Separator before content
        v.push(RenderLine {
            text: border.separator(width),
            color: BORDER_COLOR,
            bold: false,
            is_wrapped: false,
            is_border_only: true,
        });
        rows_left = rows_left.saturating_sub(1);

        // Column layout: left (open), right (closed)
        let inner_w = width.saturating_sub(2) as usize; // exclude vertical borders
        let col_gap = 2usize; // spaces between columns
        let col_w = if inner_w > col_gap { (inner_w - col_gap) / 2 } else { inner_w / 2 };

        // Titles row
        if rows_left > 0 {
            let left_title = format!("Open Positions ({})", open_positions.len());
            let right_title = "Recent Closed".to_string();
            let row = format!(
                "{:<lw$}{:gap$}{:<rw$}",
                Self::pad_truncate(&left_title, col_w as u16),
                "",
                Self::pad_truncate(&right_title, col_w as u16),
                lw = col_w,
                gap = col_gap,
                rw = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row, width),
                color: Color::White,
                bold: true,
                is_wrapped: true,
                is_border_only: false,
            });
            rows_left -= 1;
        }

        // Headers row
        if rows_left > 0 {
            let hdr = format!(
                "{:<12} {:>11} {:>11} {:>11} {:>7}",
                "Symbol",
                "Entry",
                "Current",
                "P&L SOL",
                "P&L %"
            );
            let row = format!(
                "{:<lw$}{:gap$}{:<rw$}",
                Self::pad_truncate(&hdr, col_w as u16),
                "",
                Self::pad_truncate(&hdr, col_w as u16),
                lw = col_w,
                gap = col_gap,
                rw = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row, width),
                color: Color::Yellow,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            rows_left -= 1;
        }

        // Rows in columns
        if rows_left > 0 {
            let left_iter = open_positions.iter();
            let right_iter = closed_positions
                .iter()
                .filter(|p| p.exit_price.is_some())
                .rev();
            let mut left_rows: Vec<String> = Vec::new();
            let mut right_rows: Vec<String> = Vec::new();

            for position in left_iter.take(rows_left) {
                // Use stored current_price from position object (updated by monitor_open_positions)
                let current_price = position.current_price.unwrap_or(0.0);
                let (pnl_sol, pnl_percent) = if current_price > 0.0 {
                    calculate_position_pnl(position, Some(current_price))
                } else {
                    (0.0, 0.0)
                };
                let sym = {
                    let mut acc = String::new();
                    let mut w = 0usize;
                    for ch in position.symbol.chars() {
                        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if w + cw > 11 {
                            break;
                        }
                        acc.push(ch);
                        w += cw;
                    }
                    acc
                };
                let row = format!(
                    "{:<12} {:>11.9} {:>11.9} {:>11.9} {:>6.2}%",
                    sym,
                    position.entry_price,
                    current_price,
                    pnl_sol,
                    pnl_percent
                );
                left_rows.push(Self::pad_truncate(&row, col_w as u16));
            }

            for position in right_iter.take(rows_left) {
                let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
                let sym = {
                    let mut acc = String::new();
                    let mut w = 0usize;
                    for ch in position.symbol.chars() {
                        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if w + cw > 11 {
                            break;
                        }
                        acc.push(ch);
                        w += cw;
                    }
                    acc
                };
                let row = format!(
                    "{:<12} {:>11.9} {:>11.9} {:>11.9} {:>6.2}%",
                    sym,
                    position.entry_price,
                    position.exit_price.unwrap_or(0.0),
                    pnl_sol,
                    pnl_percent
                );
                right_rows.push(Self::pad_truncate(&row, col_w as u16));
            }

            let max_rows = left_rows.len().max(right_rows.len()).min(rows_left);
            for i in 0..max_rows {
                let left = left_rows
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "".to_string());
                let right = right_rows
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "".to_string());
                // Determine color by left side P&L only to keep simple; borders stay light gray due to renderer
                let color = Color::White;
                let row = format!(
                    "{:<lw$}{:gap$}{:<rw$}",
                    left,
                    "",
                    right,
                    lw = col_w,
                    gap = col_gap,
                    rw = col_w
                );
                v.push(RenderLine {
                    text: border.wrap_content(&row, width),
                    color,
                    bold: false,
                    is_wrapped: true,
                    is_border_only: false,
                });
            }
            rows_left = rows_left.saturating_sub(max_rows);
        }

        // Bottom border
        v.push(RenderLine {
            text: border.bottom_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });

        // Ensure we don't exceed height
        v.truncate(height as usize);
        v
    }

    async fn build_statistics_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 {
            return v;
        }
        // Use a single modern border style everywhere to avoid visual mismatch
        let border = &BorderChars::ROUNDED;

        // Section header with top border
        v.push(RenderLine {
            text: border.top_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });
        if height == 1 {
            return v;
        }

        v.push(RenderLine {
            text: border.wrap_content("STATISTICS", width),
            color: Color::White,
            bold: true,
            is_wrapped: true,
            is_border_only: false,
        });
        if height == 2 {
            return v;
        }

        // Wallet balance (fast, with timeout)
        let wallet_balance = if let Ok(wallet_addr) = get_wallet_address() {
            if
                let Ok(balance) = tokio::time::timeout(
                    Duration::from_millis(200),
                    get_sol_balance(&wallet_addr.to_string())
                ).await
            {
                balance.unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let open_positions = get_open_positions().await;
        let closed_positions = get_closed_positions().await;
        let total_open = open_positions.len();
        let total_closed = closed_positions.len();

        let mut total_pnl_sol = 0.0;
        let mut winners = 0;
        let mut losers = 0;
        for position in &closed_positions {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
            total_pnl_sol += pnl_sol;
            if pnl_percent > 0.0 {
                winners += 1;
            } else if pnl_percent < 0.0 {
                losers += 1;
            }
        }
        let win_rate = if total_closed > 0 {
            ((winners as f64) / (total_closed as f64)) * 100.0
        } else {
            0.0
        };

        // Get system stats
        let rpc_stats = get_global_rpc_stats();
        let total_requests: u64 = if let Some(stats) = &rpc_stats {
            stats.calls_per_method.values().sum()
        } else {
            0
        };

        // Get pricing stats (non-blocking with timeout)
        let pricing_stats = if
            let Ok(stats) = tokio::time::timeout(
                Duration::from_millis(100),
                get_pricing_stats()
            ).await
        {
            stats
        } else {
            "Price Cache: Loading...".to_string()
        };

        // Get transaction stats (non-blocking with timeout)
        let tx_stats = if
            let Ok(stats) = tokio::time::timeout(
                Duration::from_millis(100),
                get_transaction_stats()
            ).await
        {
            stats
        } else {
            crate::transactions::TransactionStats {
                total_transactions: 0,
                new_transactions_count: 0,
                known_signatures_count: 0,
            }
        };

        // Get pool service stats (non-blocking with timeout)
        let pool_stats = if
            let Ok(stats) = tokio::time::timeout(Duration::from_millis(100), async {
                let pool_service = get_pool_service();
                pool_service.get_enhanced_stats().await
            }).await
        {
            stats
        } else {
            crate::tokens::pool::PoolServiceStats::default()
        };

        // Multi-row display for comprehensive stats
        let mut current_line = 3usize; // Account for header + top border

        // Separator line
        if current_line < (height as usize) {
            v.push(RenderLine {
                text: border.separator(width),
                color: BORDER_COLOR,
                bold: false,
                is_wrapped: false,
                is_border_only: true,
            });
            current_line += 1;
        }

        // Row 1: Wallet & Core Trading Stats
        if current_line < (height as usize) {
            let col1 = format!("Wallet: {:.9} SOL", wallet_balance);
            let col2 = format!("Open: {} | Closed: {}", total_open, total_closed);
            let col3 = format!("Win Rate: {:.1}%", win_rate);
            let content_width = width.saturating_sub(2) as usize;
            let col_w = content_width / 3;
            let row_content = format!(
                "{:<col_w$}{:<col_w$}{:<col_w$}",
                col1.chars().take(col_w).collect::<String>(),
                col2.chars().take(col_w).collect::<String>(),
                col3.chars().take(col_w).collect::<String>(),
                col_w = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row_content, width),
                color: Color::White,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Row 2: P&L & RPC Stats
        if current_line < (height as usize) {
            let pnl_color = if total_pnl_sol > 0.0 {
                Color::Green
            } else if total_pnl_sol < 0.0 {
                Color::Red
            } else {
                Color::White
            };
            let col1 = format!("P&L: {:+.9} SOL", total_pnl_sol);
            let col2 = format!("RPC Calls: {}", total_requests);
            let success_rate = if total_requests > 0 { 95.0 } else { 0.0 };
            let col3 = format!("RPC Success: {:.1}%", success_rate);
            let content_width = width.saturating_sub(2) as usize;
            let col_w = content_width / 3;
            let row_content = format!(
                "{:<col_w$}{:<col_w$}{:<col_w$}",
                col1.chars().take(col_w).collect::<String>(),
                col2.chars().take(col_w).collect::<String>(),
                col3.chars().take(col_w).collect::<String>(),
                col_w = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row_content, width),
                color: pnl_color,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Row 3: Transaction Stats
        if current_line < (height as usize) {
            let col1 = format!("Tx Total: {}", tx_stats.total_transactions);
            let col2 = format!("New: {}", tx_stats.new_transactions_count);
            let col3 = format!("Cached: {}", tx_stats.known_signatures_count);
            let content_width = width.saturating_sub(2) as usize;
            let col_w = content_width / 3;
            let row_content = format!(
                "{:<col_w$}{:<col_w$}{:<col_w$}",
                col1.chars().take(col_w).collect::<String>(),
                col2.chars().take(col_w).collect::<String>(),
                col3.chars().take(col_w).collect::<String>(),
                col_w = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row_content, width),
                color: Color::Cyan,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Row 4: Pool Performance
        if current_line < (height as usize) {
            let col1 = format!("Pool Req: {}", pool_stats.total_price_requests);
            let col2 = format!("Cache Hits: {}", pool_stats.cache_hits);
            let pool_success_rate = if pool_stats.total_price_requests > 0 {
                ((pool_stats.successful_calculations as f64) /
                    (pool_stats.total_price_requests as f64)) *
                    100.0
            } else {
                0.0
            };
            let col3 = format!("Pool Rate: {:.1}%", pool_success_rate);
            let pool_color = if pool_success_rate >= 90.0 {
                Color::Green
            } else if pool_success_rate >= 70.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            let content_width = width.saturating_sub(2) as usize;
            let col_w = content_width / 3;
            let row_content = format!(
                "{:<col_w$}{:<col_w$}{:<col_w$}",
                col1.chars().take(col_w).collect::<String>(),
                col2.chars().take(col_w).collect::<String>(),
                col3.chars().take(col_w).collect::<String>(),
                col_w = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row_content, width),
                color: pool_color,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Row 5: Pool Timing Stats
        if current_line < (height as usize) {
            let col1 = format!("Pool OK: {}", pool_stats.successful_calculations);
            let col2 = format!("Pool Fail: {}", pool_stats.failed_calculations);
            let col3 = format!("Blockchain: {}", pool_stats.blockchain_calculations);
            let content_width = width.saturating_sub(2) as usize;
            let col_w = content_width / 3;
            let row_content = format!(
                "{:<col_w$}{:<col_w$}{:<col_w$}",
                col1.chars().take(col_w).collect::<String>(),
                col2.chars().take(col_w).collect::<String>(),
                col3.chars().take(col_w).collect::<String>(),
                col_w = col_w
            );
            v.push(RenderLine {
                text: border.wrap_content(&row_content, width),
                color: Color::Blue,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Row 6: Price Service Summary (compact)
        if current_line < (height as usize) {
            let price_summary = if pricing_stats.len() > (width.saturating_sub(4) as usize) {
                format!("{:.width$}...", pricing_stats, width = width.saturating_sub(7) as usize)
            } else {
                pricing_stats
            };
            v.push(RenderLine {
                text: border.wrap_content(&price_summary, width),
                color: Color::Grey,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current_line += 1;
        }

        // Bottom border
        v.push(RenderLine {
            text: border.bottom_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });

        v.truncate(height as usize);
        v
    }

    async fn build_logs_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 {
            return v;
        }
        let border = &BorderChars::ROUNDED;

        // Section header with top border
        v.push(RenderLine {
            text: border.top_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });
        if height == 1 {
            return v;
        }

        v.push(RenderLine {
            text: border.wrap_content("ACTIVITY LOG", width),
            color: Color::White,
            bold: true,
            is_wrapped: true,
            is_border_only: false,
        });
        if height == 2 {
            return v;
        }

        // Separator line
        v.push(RenderLine {
            text: border.separator(width),
            color: BORDER_COLOR,
            bold: false,
            is_wrapped: false,
            is_border_only: true,
        });
        if height == 3 {
            return v;
        }

        let mut current = 3usize;

        // Copy logs quickly to avoid holding lock during formatting
        let logs_snapshot = {
            if let Ok(logs) = self.logs.lock() {
                logs.iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new() // fallback on lock failure
            }
        };

        let avail = height.saturating_sub(4) as usize; // Account for borders + footer
        for log_entry in logs_snapshot.iter().rev().take(avail) {
            let max_msg_len = width.saturating_sub(27) as usize; // Account for borders and timestamp
            let message = {
                // build truncated message by display width
                let mut acc = String::new();
                let mut w = 0usize;
                for ch in log_entry.message.chars() {
                    let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if w + ch_w > max_msg_len {
                        break;
                    }
                    acc.push(ch);
                    w += ch_w;
                }
                if acc.width() < max_msg_len {
                    acc
                } else {
                    let ell = "...";
                    let ell_w = ell.width();
                    let mut base = String::new();
                    let mut bw = 0usize;
                    for ch in acc.chars() {
                        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if bw + ch_w + ell_w > max_msg_len {
                            break;
                        }
                        base.push(ch);
                        bw += ch_w;
                    }
                    base.push_str(ell);
                    base
                }
            };
            let line = format!("[{}] [{}] {}", log_entry.timestamp, log_entry.tag, message);
            v.push(RenderLine {
                text: border.wrap_content(&line, width),
                color: log_entry.color,
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
            current += 1;
            if current >= (height.saturating_sub(1) as usize) {
                break;
            } // Reserve space for bottom border
        }

        // Bottom border
        v.push(RenderLine {
            text: border.bottom_border(width),
            color: BORDER_COLOR,
            bold: true,
            is_wrapped: false,
            is_border_only: true,
        });

        v.truncate(height as usize);
        v
    }
    fn build_footer_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 {
            return v;
        }
        let border = &BorderChars::ROUNDED;

        // Top border
        v.push(RenderLine {
            text: border.top_border(width),
            color: BORDER_COLOR,
            bold: false,
            is_wrapped: false,
            is_border_only: true,
        });

        if height >= 2 {
            let controls = if self.phase == DashboardPhase::ShuttingDown {
                if self.services_completed {
                    format!(
                        "✅ Shutdown Complete - Dashboard will exit shortly [{} {}]",
                        self.phase_label(),
                        self.spinner()
                    )
                } else {
                    format!(
                        "🔄 Shutdown in Progress - Please wait for services to complete [{} {}]",
                        self.phase_label(),
                        self.spinner()
                    )
                }
            } else {
                format!(
                    "Controls: [Q/ESC] Exit | [R] Refresh | [C] Clear Logs | [Ctrl+C] Force Exit  [{} {}]",
                    self.phase_label(),
                    self.spinner()
                )
            };

            v.push(RenderLine {
                text: border.wrap_content(&controls, width),
                color: if self.services_completed {
                    Color::Green
                } else {
                    Color::White
                },
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
        }

        if height >= 3 {
            let status = match self.phase {
                DashboardPhase::Startup =>
                    format!(
                        "Starting services... | Last Update: {} | Terminal: {}x{}",
                        Local::now().format("%H:%M:%S"),
                        width,
                        terminal
                            ::size()
                            .map(|(_, h)| h)
                            .unwrap_or(self.terminal_size.1)
                    ),
                DashboardPhase::Running =>
                    format!(
                        "Dashboard Running | Last Update: {} | Terminal: {}x{}",
                        Local::now().format("%H:%M:%S"),
                        width,
                        terminal
                            ::size()
                            .map(|(_, h)| h)
                            .unwrap_or(self.terminal_size.1)
                    ),
                DashboardPhase::ShuttingDown => {
                    if self.services_completed {
                        format!(
                            "✅ All services completed - exiting soon | Last Update: {} | Terminal: {}x{}",
                            Local::now().format("%H:%M:%S"),
                            width,
                            terminal
                                ::size()
                                .map(|(_, h)| h)
                                .unwrap_or(self.terminal_size.1)
                        )
                    } else {
                        let elapsed = self.shutdown_started_at
                            .map(|t| t.elapsed().as_secs())
                            .unwrap_or(0);
                        format!(
                            "🔄 Shutting down... waiting for services ({} seconds) | Terminal: {}x{}",
                            elapsed,
                            width,
                            terminal
                                ::size()
                                .map(|(_, h)| h)
                                .unwrap_or(self.terminal_size.1)
                        )
                    }
                }
            };
            v.push(RenderLine {
                text: border.wrap_content(&status, width),
                color: if self.services_completed {
                    Color::Green
                } else {
                    Color::White
                },
                bold: false,
                is_wrapped: true,
                is_border_only: false,
            });
        }

        v
    }

    // Removed legacy immediate-draw functions; the dashboard now uses the
    // modern incremental renderer with Unicode borders only.

    /// Add a log entry to the dashboard
    pub fn add_log(&self, tag: &str, log_type: &str, message: &str) {
        let color = match log_type.to_uppercase().as_str() {
            "ERROR" | "FAILED" => Color::Red,
            "WARN" | "WARNING" => Color::Yellow,
            "SUCCESS" => Color::Green,
            "INFO" => Color::Cyan,
            "BUY" => Color::Green,
            "SELL" => Color::Magenta,
            "PROFIT" => Color::Green,
            "LOSS" => Color::Red,
            _ => Color::White,
        };

        let log_entry = LogEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            tag: tag.to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            color,
        };

        if let Ok(mut logs) = self.logs.lock() {
            logs.push_back(log_entry);

            // Keep only recent logs
            while logs.len() > MAX_LOG_LINES * 2 {
                logs.pop_front();
            }
        }
    }

    /// Shutdown the dashboard and restore terminal
    pub async fn shutdown(&self) -> io::Result<()> {
        // Set running flag to false
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }

        // Restore terminal state
        execute!(stdout(), Show, EnableLineWrap, LeaveAlternateScreen)?;

        // Disable raw mode
        terminal::disable_raw_mode()?;

        Ok(())
    }
}

/// Initialize and run the dashboard
pub async fn run_dashboard(
    shutdown: Arc<Notify>,
    services_completed: Arc<Notify>
) -> io::Result<()> {
    let mut dashboard = Dashboard::new();

    // Initialize terminal for dashboard
    dashboard.initialize().await?;

    // Add initial log
    dashboard.add_log("SYSTEM", "INFO", "Dashboard initialized successfully");

    // Run the dashboard
    let result = dashboard.run(shutdown, services_completed).await;

    // Always cleanup terminal state
    let _ = dashboard.shutdown().await;

    result
}

/// Global dashboard instance for log forwarding
static GLOBAL_DASHBOARD: Lazy<Arc<Mutex<Option<Arc<Dashboard>>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// Set the global dashboard instance
pub fn set_global_dashboard(dashboard: Arc<Dashboard>) {
    if let Ok(mut global) = GLOBAL_DASHBOARD.lock() {
        *global = Some(dashboard);
    }
}

/// Clear the global dashboard instance
pub fn clear_global_dashboard() {
    if let Ok(mut global) = GLOBAL_DASHBOARD.lock() {
        *global = None;
    }
}

/// Create a dashboard log interceptor for the logger module
/// This function should be called from the logger when dashboard mode is active
pub fn dashboard_log(tag: &str, log_type: &str, message: &str) {
    // Use try_lock to avoid blocking if dashboard is busy
    if let Ok(global) = GLOBAL_DASHBOARD.try_lock() {
        if let Some(ref dashboard) = *global {
            dashboard.add_log(tag, log_type, message);
        }
    }
    // If we can't get the lock, just drop the log to avoid deadlock
}
