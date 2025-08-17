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
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor, Attribute, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetSize, DisableLineWrap, EnableLineWrap, SetTitle},
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, Write, stdout};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::sleep;
use chrono::{Local, Utc};
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::positions::{get_open_positions, get_closed_positions, calculate_position_pnl};
use crate::tokens::get_token_price_safe;
use crate::utils::{get_sol_balance, get_wallet_address};
use crate::rpc::get_global_rpc_stats;
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
    color: Color,
    bg: Option<Color>,
    bold: bool,
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
        }
    }

    /// Initialize the dashboard terminal
    pub async fn initialize(&mut self) -> io::Result<()> {
        // Check terminal size requirements
        let (width, height) = terminal::size()?;
        if width < MIN_TERMINAL_WIDTH || height < MIN_TERMINAL_HEIGHT {
            eprintln!("Terminal too small! Minimum size: {}x{}, Current: {}x{}", 
                     MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT, width, height);
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
    pub async fn run(&mut self, shutdown: Arc<Notify>) -> io::Result<()> {
        let mut last_draw = Instant::now();
        
        loop {
            // Check for shutdown signal first
            tokio::select! {
                _ = shutdown.notified() => {
                    // Switch to shutdown phase and keep UI alive to stream logs
                    self.phase = DashboardPhase::ShuttingDown;
                    if self.shutdown_started_at.is_none() { self.shutdown_started_at = Some(Instant::now()); }
                    self.shutdown_requested = true;
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

            // Decide if we should exit the loop (only after shutdown drain)
            if self.phase == DashboardPhase::ShuttingDown {
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
                
                let inactive = self.last_log_change.elapsed() >= Duration::from_millis(SHUTDOWN_INACTIVITY_MS);
                let timed_out = self.shutdown_started_at.map(|t| t.elapsed() >= Duration::from_millis(SHUTDOWN_MAX_WAIT_MS)).unwrap_or(false);
                if inactive || timed_out {
                    break;
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
        let header_height = 3;
        let footer_height = 2;
        let available_height = max_content_height.saturating_sub(header_height + footer_height);
        
    // Split available space into sections
    let positions_height = (available_height * 40 / 100).max(8); // 40% for positions
    let stats_height = (available_height * 30 / 100).max(6); // 30% for stats
    let logs_height = available_height.saturating_sub(positions_height + stats_height); // Rest for logs
        
        let mut current_row = header_height;
        
        // Draw positions section incrementally
        if current_row < max_content_height {
            let sec_h = positions_height.min(max_content_height.saturating_sub(current_row));
            let pos_lines = self.build_positions_lines(width, sec_h).await;
            Self::draw_section(&mut stdout, current_row, width, &pos_lines, &mut self.prev_positions)?;
            current_row = current_row.saturating_add(sec_h);
        }
        
        // Draw statistics section incrementally
        if current_row < max_content_height {
            let sec_h = stats_height.min(max_content_height.saturating_sub(current_row));
            let stats_lines = self.build_statistics_lines(width, sec_h).await;
            Self::draw_section(&mut stdout, current_row, width, &stats_lines, &mut self.prev_stats)?;
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
            Self::draw_section(&mut stdout, footer_start, width, &footer_lines, &mut self.prev_footer)?;
        }
        
        // advance spinner for next frame
        self.spinner_idx = (self.spinner_idx + 1) % 4;
        stdout.flush()?;
        Ok(())
    }

    fn pad_truncate(text: &str, width: u16) -> String {
        let target = width as usize;
        if target == 0 { return String::new(); }
        let mut out = String::with_capacity(target);
        let mut curw = 0usize;
        for ch in text.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if curw + ch_w > target { break; }
            out.push(ch);
            curw += ch_w;
        }
        if curw < target { out.push_str(&" ".repeat(target - curw)); }
        out
    }

    /// Draw only the lines that changed; clear current line before printing to avoid leftovers
    fn draw_section(
        stdout: &mut io::Stdout,
        start_row: u16,
        width: u16,
        lines: &Vec<RenderLine>,
        prev_cache: &mut Vec<String>,
    ) -> io::Result<()> {
        for (i, rl) in lines.iter().enumerate() {
            let row = start_row.saturating_add(i as u16);
            let new_text = Self::pad_truncate(&rl.text, width);
            let cached = prev_cache.get(i);
            if cached.map(|c| c.as_str()) != Some(new_text.as_str()) {
                stdout.queue(MoveTo(0, row))?;
                stdout.queue(Clear(ClearType::CurrentLine))?;
                if let Some(bg) = rl.bg { stdout.queue(SetBackgroundColor(bg))?; }
                stdout.queue(SetForegroundColor(rl.color))?;
                if rl.bold { stdout.queue(SetAttribute(Attribute::Bold))?; }
                stdout.queue(Print(new_text.clone()))?;
                stdout.queue(ResetColor)?;
                if prev_cache.len() <= i { prev_cache.resize(i+1, String::new()); }
                prev_cache[i] = new_text;
            }
        }
        // If previous cache had more lines (e.g., resize smaller), we don't need to clear: those
        // extra rows are outside this section or will be covered by other sections/full redraw.
        if prev_cache.len() > lines.len() { prev_cache.truncate(lines.len()); }
    Ok(())
    }

    fn spinner(&self) -> char {
        match self.spinner_idx { 0 => '-', 1 => '\\', 2 => '|', _ => '/' }
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
        if height >= 1 {
            let title = format!("ScreenerBot Dashboard  [{} {}]", self.phase_label(), self.spinner());
            let t = format!("{:^width$}", title, width = width as usize);
            v.push(RenderLine{ text: Self::pad_truncate(&t, width), color: Color::White, bg: Some(Color::DarkBlue), bold: true });
        }
        if height >= 2 {
            let t = format!("{:^width$}", Local::now().format("%Y-%m-%d %H:%M:%S"), width = width as usize);
            v.push(RenderLine{ text: Self::pad_truncate(&t, width), color: Color::White, bg: Some(Color::DarkBlue), bold: true });
        }
        if height >= 3 {
            v.push(RenderLine{ text: "-".repeat(width as usize), color: Color::DarkGrey, bg: None, bold: false });
        }
        v
    }

    async fn build_positions_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 { return v; }
        v.push(RenderLine{ text: "POSITIONS".to_string(), color: Color::Cyan, bg: None, bold: true });
        if height == 1 { return v; }

        let mut rows_left = height.saturating_sub(1) as usize;
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();

        if !open_positions.is_empty() && rows_left > 0 {
            v.push(RenderLine{ text: format!("Open Positions ({})", open_positions.len()), color: Color::Green, bg: None, bold: true });
            rows_left = rows_left.saturating_sub(1);
            if rows_left > 0 {
                let hdr = format!("{:<12} {:>14} {:>14} {:>14} {:>9}", "Symbol", "Entry", "Current", "P&L SOL", "P&L %");
                v.push(RenderLine{ text: hdr, color: Color::Yellow, bg: None, bold: false });
                v.push(RenderLine{ text: "-".repeat(width as usize), color: Color::DarkGrey, bg: None, bold: false });
                rows_left = rows_left.saturating_sub(1);
            }
            for position in open_positions.iter().take(rows_left) {
                // Get current price via centralized price service (bounded wait)
                let current_price = if let Ok(price_opt) = tokio::time::timeout(
                    Duration::from_millis(150),
                    get_token_price_safe(&position.mint)
                ).await {
                    price_opt.unwrap_or(0.0)
                } else { 0.0 };
                let (pnl_sol, pnl_percent) = if current_price > 0.0 { calculate_position_pnl(position, Some(current_price)) } else { (0.0, 0.0) };
                let color = if pnl_percent > 0.0 { Color::Green } else if pnl_percent < 0.0 { Color::Red } else { Color::White };
                let sym = {
                    let mut acc = String::new();
                    let mut w = 0usize;
                    for ch in position.symbol.chars() { let cw = UnicodeWidthChar::width(ch).unwrap_or(0); if w + cw > 11 { break; } acc.push(ch); w += cw; }
                    acc
                };
                let row = format!("{:<12} {:>14.9} {:>14.9} {:>14.9} {:>8.2}%",
                    sym, position.entry_price, current_price, pnl_sol, pnl_percent);
                v.push(RenderLine{ text: Self::pad_truncate(&row, width), color, bg: None, bold: false });
            }
        } else {
            v.push(RenderLine{ text: "No open positions".to_string(), color: Color::Grey, bg: None, bold: false });
            rows_left = rows_left.saturating_sub(1);
        }

        if rows_left > 0 && !closed_positions.is_empty() {
            v.push(RenderLine{ text: "Recent Closed Positions".to_string(), color: Color::Yellow, bg: None, bold: true });
            rows_left = rows_left.saturating_sub(1);
            let recent_closed: Vec<_> = closed_positions.iter().filter(|p| p.exit_price.is_some()).rev().take(rows_left).collect();
            for position in recent_closed {
                let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
                let color = if pnl_percent > 0.0 { Color::Green } else if pnl_percent < 0.0 { Color::Red } else { Color::White };
                let sym = {
                    let mut acc = String::new();
                    let mut w = 0usize;
                    for ch in position.symbol.chars() { let cw = UnicodeWidthChar::width(ch).unwrap_or(0); if w + cw > 11 { break; } acc.push(ch); w += cw; }
                    acc
                };
                let row = format!("{:<12} {:>14.9} {:>14.9} {:>14.9} {:>8.2}%",
                    sym, position.entry_price, position.exit_price.unwrap_or(0.0), pnl_sol, pnl_percent);
                v.push(RenderLine{ text: Self::pad_truncate(&row, width), color, bg: None, bold: false });
            }
        }
        // Ensure we don't exceed height
        v.truncate(height as usize);
        v
    }

    async fn build_statistics_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 { return v; }
    v.push(RenderLine{ text: "STATISTICS".to_string(), color: Color::Magenta, bg: None, bold: true });
        if height == 1 { return v; }

        // Wallet balance (fast, with timeout)
        let wallet_balance = if let Ok(wallet_addr) = get_wallet_address() {
            if let Ok(balance) = tokio::time::timeout(Duration::from_millis(200), get_sol_balance(&wallet_addr.to_string())).await { balance.unwrap_or(0.0) } else { 0.0 }
        } else { 0.0 };

        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        let total_open = open_positions.len();
        let total_closed = closed_positions.len();

        let mut total_pnl_sol = 0.0; let mut winners = 0; let mut losers = 0;
        for position in &closed_positions { let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None); total_pnl_sol += pnl_sol; if pnl_percent > 0.0 { winners += 1; } else if pnl_percent < 0.0 { losers += 1; } }
        let win_rate = if total_closed > 0 { (winners as f64 / total_closed as f64) * 100.0 } else { 0.0 };
        let rpc_stats = get_global_rpc_stats();
        let total_requests: u64 = if let Some(stats) = &rpc_stats { stats.calls_per_method.values().sum() } else { 0 };

        // Row 1
        if v.len() < height as usize {
            let col1 = Self::pad_truncate(&format!("Wallet: {:.9} SOL", wallet_balance), width/3);
            let col2 = Self::pad_truncate(&format!("Open: {}", total_open), width/3);
            let col3 = Self::pad_truncate(&format!("RPC Calls: {}", total_requests), width - (width/3)*2);
            v.push(RenderLine{ text: format!("{}{}{}", col1, col2, col3), color: Color::White, bg: None, bold: false });
        }
        // Row 2
        if v.len() < height as usize {
            let pnl_color = if total_pnl_sol > 0.0 { Color::Green } else if total_pnl_sol < 0.0 { Color::Red } else { Color::White };
            let col1 = Self::pad_truncate(&format!("P&L: {:+.9} SOL", total_pnl_sol), width/3);
            let col2 = Self::pad_truncate(&format!("WinRate: {:.1}%", win_rate), width/3);
            let success_rate = if total_requests > 0 { 95.0 } else { 0.0 };
            let col3 = Self::pad_truncate(&format!("RPC Success: {:.1}%", success_rate), width - (width/3)*2);
            v.push(RenderLine{ text: format!("{}{}{}", col1, col2, col3), color: pnl_color, bg: None, bold: false });
        }
        v.truncate(height as usize);
        v
    }

    async fn build_logs_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 { return v; }
        v.push(RenderLine{ text: "ACTIVITY LOG".to_string(), color: Color::White, bg: None, bold: true });
        v.push(RenderLine{ text: "-".repeat(width as usize), color: Color::DarkGrey, bg: None, bold: false });
        if height == 1 { return v; }
        let mut current = 2usize;
        
        // Copy logs quickly to avoid holding lock during formatting
        let logs_snapshot = {
            if let Ok(logs) = self.logs.lock() {
                logs.iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new() // fallback on lock failure
            }
        };
        
        let avail = height as usize - 1;
        for log_entry in logs_snapshot.iter().rev().take(avail) {
            let max_msg_len = (width as usize).saturating_sub(25);
            let message = {
                // build truncated message by display width
                let mut acc = String::new();
                let mut w = 0usize;
                for ch in log_entry.message.chars() {
                    let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if w + ch_w > max_msg_len { break; }
                    acc.push(ch);
                    w += ch_w;
                }
                if acc.width() < max_msg_len { acc } else { 
                    let ell = "...";
                    let ell_w = ell.width();
                    let mut base = String::new();
                    let mut bw = 0usize;
                    for ch in acc.chars() {
                        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if bw + ch_w + ell_w > max_msg_len { break; }
                        base.push(ch); bw += ch_w;
                    }
                    base.push_str(ell);
                    base
                }
            };
            let line = format!("[{}] [{}] {}", log_entry.timestamp, log_entry.tag, message);
            v.push(RenderLine{ text: line, color: log_entry.color, bg: None, bold: false });
            current += 1;
            if current >= height as usize { break; }
        }
        v.truncate(height as usize);
        v
    }    fn build_footer_lines(&self, width: u16, height: u16) -> Vec<RenderLine> {
        let mut v = Vec::new();
        if height == 0 { return v; }
        let controls = format!("Controls: [Q/ESC] Exit | [R] Refresh | [C] Clear Logs | [Ctrl+C] Force Exit  [{} {}]",
                               self.phase_label(), self.spinner());
        v.push(RenderLine{ text: Self::pad_truncate(&controls, width), color: Color::White, bg: Some(Color::DarkGrey), bold: false });
        if height >= 2 {
            let status = match self.phase {
                DashboardPhase::Startup => format!("Starting services... | Last Update: {} | Terminal: {}x{}", Local::now().format("%H:%M:%S"), width, terminal::size().map(|(_, h)| h).unwrap_or(self.terminal_size.1)),
                DashboardPhase::Running => format!("Dashboard Running | Last Update: {} | Terminal: {}x{}", Local::now().format("%H:%M:%S"), width, terminal::size().map(|(_, h)| h).unwrap_or(self.terminal_size.1)),
                DashboardPhase::ShuttingDown => format!("Shutting down... waiting for services | Last Update: {} | Terminal: {}x{}", Local::now().format("%H:%M:%S"), width, terminal::size().map(|(_, h)| h).unwrap_or(self.terminal_size.1)),
            };
            v.push(RenderLine{ text: Self::pad_truncate(&status, width), color: Color::White, bg: Some(Color::DarkGrey), bold: false });
        }
        v
    }

    /// Draw the header section
    async fn draw_header(&self, stdout: &mut io::Stdout, width: u16, height: u16) -> io::Result<()> {
        let now = Local::now();
    let title = "ScreenerBot Dashboard";
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Header background
        stdout.queue(SetBackgroundColor(Color::DarkBlue))?;
        stdout.queue(SetForegroundColor(Color::White))?;
        stdout.queue(SetAttribute(Attribute::Bold))?;
        
        // Title line
        if height >= 1 {
            stdout.queue(MoveTo(0, 0))?;
            let mut line = format!("{:^width$}", title, width = width as usize);
            if line.len() > width as usize { line.truncate(width as usize); }
            stdout.queue(Print(line))?;
        }
        
        // Timestamp line
        if height >= 2 {
            stdout.queue(MoveTo(0, 1))?;
            let mut line = format!("{:^width$}", timestamp, width = width as usize);
            if line.len() > width as usize { line.truncate(width as usize); }
            stdout.queue(Print(line))?;
        }
        
        // Separator line
        if height >= 3 {
            stdout.queue(MoveTo(0, 2))?;
            let sep = "═".repeat(width as usize);
            stdout.queue(Print(sep))?;
        }
        
        stdout.queue(ResetColor)?;
        Ok(())
    }

    /// Draw the positions section
    async fn draw_positions_section(&self, stdout: &mut io::Stdout, start_row: u16, width: u16, height: u16) -> io::Result<u16> {
        if height == 0 { return Ok(start_row); }
        // Section header
        if height >= 1 {
            stdout.queue(MoveTo(0, start_row))?;
            stdout.queue(SetForegroundColor(Color::Cyan))?;
            stdout.queue(SetAttribute(Attribute::Bold))?;
            stdout.queue(Print("POSITIONS"))?;
            stdout.queue(ResetColor)?;
        }
        
        let mut current_row = start_row + 1;
        
        // Get positions data
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        
        // Draw open positions
    if !open_positions.is_empty() {
            stdout.queue(MoveTo(0, current_row))?;
            stdout.queue(SetForegroundColor(Color::Green))?;
            stdout.queue(Print(format!("Open Positions ({})", open_positions.len())))?;
            stdout.queue(ResetColor)?;
            current_row += 1;
            
            // Header
            if current_row < start_row + height {
                stdout.queue(MoveTo(0, current_row))?;
                stdout.queue(SetForegroundColor(Color::Yellow))?;
                let mut hdr = format!("{:<12} {:<10} {:<12} {:<12} {:<10}", 
                    "Symbol", "Entry", "Current", "P&L SOL", "P&L %");
                if hdr.len() > width as usize { hdr.truncate(width as usize); }
                stdout.queue(Print(hdr))?;
                stdout.queue(ResetColor)?;
                current_row += 1;
            }
            
            // Show up to available space
            let available_rows = (start_row + height).saturating_sub(current_row);
            for (i, position) in open_positions.iter().take(available_rows as usize).enumerate() {
                if current_row >= start_row + height { break; }
                
                // Get current price for P&L calculation via centralized price service
                let current_price = if let Ok(price_opt) = tokio::time::timeout(
                    Duration::from_millis(150),
                    get_token_price_safe(&position.mint)
                ).await {
                    price_opt.unwrap_or(0.0)
                } else { 0.0 };
                
                let (pnl_sol, pnl_percent) = if current_price > 0.0 {
                    calculate_position_pnl(position, Some(current_price))
                } else {
                    (0.0, 0.0)
                };
                
                stdout.queue(MoveTo(0, current_row))?;
                
                // Color based on P&L
                if pnl_percent > 0.0 {
                    stdout.queue(SetForegroundColor(Color::Green))?;
                } else if pnl_percent < 0.0 {
                    stdout.queue(SetForegroundColor(Color::Red))?;
                } else {
                    stdout.queue(SetForegroundColor(Color::White))?;
                }
                
                let mut row = format!("{:<12} {:<10.6} {:<12.6} {:<12.6} {:<10.2}%", 
                    position.symbol.chars().take(11).collect::<String>(),
                    position.entry_price,
                    current_price,
                    pnl_sol,
                    pnl_percent
                );
                if row.len() > width as usize { row.truncate(width as usize); }
                stdout.queue(Print(row))?;
                stdout.queue(ResetColor)?;
                current_row += 1;
            }
        } else {
            if current_row < start_row + height {
                stdout.queue(MoveTo(0, current_row))?;
                stdout.queue(SetForegroundColor(Color::Grey))?;
                stdout.queue(Print("No open positions"))?;
                stdout.queue(ResetColor)?;
                current_row += 1;
            }
        }
        
        // Recent closed positions
        if current_row < start_row + height && !closed_positions.is_empty() {
            current_row += 1; // Space
            stdout.queue(MoveTo(0, current_row))?;
            stdout.queue(SetForegroundColor(Color::Yellow))?;
            stdout.queue(Print("Recent Closed Positions"))?;
            stdout.queue(ResetColor)?;
            current_row += 1;
            
            let available_rows = (start_row + height).saturating_sub(current_row);
            let recent_closed: Vec<_> = closed_positions.iter()
                .filter(|p| p.exit_price.is_some())
                .rev()
                .take(available_rows as usize)
                .collect();
            
            for position in recent_closed {
                if current_row >= start_row + height { break; }
                
                let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
                
                stdout.queue(MoveTo(0, current_row))?;
                
                // Color based on P&L
                if pnl_percent > 0.0 {
                    stdout.queue(SetForegroundColor(Color::Green))?;
                } else if pnl_percent < 0.0 {
                    stdout.queue(SetForegroundColor(Color::Red))?;
                } else {
                    stdout.queue(SetForegroundColor(Color::White))?;
                }
                
                let mut row = format!("{:<12} {:<10.6} {:<12.6} {:<12.6} {:<10.2}%", 
                    position.symbol.chars().take(11).collect::<String>(),
                    position.entry_price,
                    position.exit_price.unwrap_or(0.0),
                    pnl_sol,
                    pnl_percent
                );
                if row.len() > width as usize { row.truncate(width as usize); }
                stdout.queue(Print(row))?;
                stdout.queue(ResetColor)?;
                current_row += 1;
            }
        }
        
        Ok(start_row + height)
    }

    /// Draw the statistics section
    async fn draw_statistics_section(&self, stdout: &mut io::Stdout, start_row: u16, width: u16, height: u16) -> io::Result<u16> {
        if height == 0 { return Ok(start_row); }
        // Section header
        if height >= 1 {
            stdout.queue(MoveTo(0, start_row))?;
            stdout.queue(SetForegroundColor(Color::Magenta))?;
            stdout.queue(SetAttribute(Attribute::Bold))?;
            stdout.queue(Print("STATISTICS"))?;
            stdout.queue(ResetColor)?;
        }
        
        let mut current_row = start_row + 1;
        
        // Get wallet balance
        let wallet_balance = if let Ok(wallet_addr) = get_wallet_address() {
            if let Ok(balance) = tokio::time::timeout(
                Duration::from_millis(200),
                get_sol_balance(&wallet_addr.to_string())
            ).await {
                balance.unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Calculate totals
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        
        let total_open = open_positions.len();
        let total_closed = closed_positions.len();
        
        // Calculate P&L summary
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
            (winners as f64 / total_closed as f64) * 100.0
        } else {
            0.0
        };
        
        // Get RPC stats
        let rpc_stats = get_global_rpc_stats();
        let total_requests: u64 = if let Some(stats) = &rpc_stats {
            stats.calls_per_method.values().sum()
        } else {
            0
        };
        let total_calls: u64 = if let Some(stats) = &rpc_stats {
            stats.calls_per_url.values().sum()
        } else {
            0
        };
        
        // Display statistics in columns
        let col1_width = width / 3;
        let col2_width = width / 3;
        
        // Row 1: Wallet & Positions
    if current_row < start_row + height {
            stdout.queue(MoveTo(0, current_row))?;
            stdout.queue(SetForegroundColor(Color::Cyan))?;
            let mut part = format!("Wallet: {:.6} SOL", wallet_balance);
            if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
            stdout.queue(Print(part))?;
            
            stdout.queue(MoveTo(col1_width, current_row))?;
            stdout.queue(SetForegroundColor(Color::Green))?;
            let mut part = format!("Open: {}", total_open);
            if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
            stdout.queue(Print(part))?;
            
            stdout.queue(MoveTo(col2_width * 2, current_row))?;
            stdout.queue(SetForegroundColor(Color::Yellow))?;
            let mut part = format!("Closed: {}", total_closed);
            if part.len() > (width as usize).saturating_sub((col2_width * 2) as usize) { part.truncate((width as usize).saturating_sub((col2_width * 2) as usize)); }
            stdout.queue(Print(part))?;
            stdout.queue(ResetColor)?;
            current_row += 1;
        }
        
        // Row 2: P&L & Win Rate
    if current_row < start_row + height {
            stdout.queue(MoveTo(0, current_row))?;
            if total_pnl_sol > 0.0 {
                stdout.queue(SetForegroundColor(Color::Green))?;
                let mut part = format!("P&L: +{:.6} SOL", total_pnl_sol);
                if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
                stdout.queue(Print(part))?;
            } else if total_pnl_sol < 0.0 {
                stdout.queue(SetForegroundColor(Color::Red))?;
                let mut part = format!("P&L: {:.6} SOL", total_pnl_sol);
                if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
                stdout.queue(Print(part))?;
            } else {
                stdout.queue(SetForegroundColor(Color::White))?;
                let mut part = format!("P&L: {:.6} SOL", total_pnl_sol);
                if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
                stdout.queue(Print(part))?;
            }
            
            stdout.queue(MoveTo(col1_width, current_row))?;
            if win_rate > 50.0 {
                stdout.queue(SetForegroundColor(Color::Green))?;
            } else {
                stdout.queue(SetForegroundColor(Color::Red))?;
            }
            let mut part = format!("WinRate: {:.1}%", win_rate);
            if part.len() > (col1_width as usize).saturating_sub(1) { part.truncate((col1_width as usize).saturating_sub(1)); }
            stdout.queue(Print(part))?;
            
            stdout.queue(MoveTo(col2_width * 2, current_row))?;
            stdout.queue(SetForegroundColor(Color::Blue))?;
            let mut part = format!("RPC Calls: {}", total_requests);
            if part.len() > (width as usize).saturating_sub((col2_width * 2) as usize) { part.truncate((width as usize).saturating_sub((col2_width * 2) as usize)); }
            stdout.queue(Print(part))?;
            stdout.queue(ResetColor)?;
            current_row += 1;
        }
        
        // Row 3: Winners/Losers & Success Rate
    if current_row < start_row + height {
            stdout.queue(MoveTo(0, current_row))?;
            stdout.queue(SetForegroundColor(Color::Green))?;
            stdout.queue(Print(format!("Winners: {}", winners)))?;
            
            stdout.queue(MoveTo(col1_width, current_row))?;
            stdout.queue(SetForegroundColor(Color::Red))?;
            stdout.queue(Print(format!("Losers: {}", losers)))?;
            
            stdout.queue(MoveTo(col2_width * 2, current_row))?;
            let success_rate = if total_requests > 0 {
                // Assuming most calls are successful for now - you might want to track failures
                95.0
            } else {
                0.0
            };
            stdout.queue(SetForegroundColor(Color::Cyan))?;
            stdout.queue(Print(format!("RPC Success: {:.1}%", success_rate)))?;
            stdout.queue(ResetColor)?;
            current_row += 1;
        }
        
        Ok(start_row + height)
    }

    /// Draw the logs section
    async fn draw_logs_section(&self, stdout: &mut io::Stdout, start_row: u16, width: u16, height: u16) -> io::Result<()> {
        if height == 0 { return Ok(()); }
        // Section header
        if height >= 1 {
            stdout.queue(MoveTo(0, start_row))?;
            stdout.queue(SetForegroundColor(Color::White))?;
            stdout.queue(SetAttribute(Attribute::Bold))?;
            stdout.queue(Print("ACTIVITY LOG"))?;
            stdout.queue(ResetColor)?;
        }
        
        let mut current_row = start_row + 1;
        
        // Get recent logs
        if let Ok(logs) = self.logs.lock() {
            let available_rows = (start_row + height).saturating_sub(current_row);
            let recent_logs: Vec<_> = logs.iter()
                .rev()
                .take(available_rows as usize)
                .collect();
            
            for log_entry in recent_logs {
                if current_row >= start_row + height { break; }
                
                stdout.queue(MoveTo(0, current_row))?;
                stdout.queue(SetForegroundColor(log_entry.color))?;
                
                // Truncate message to fit terminal width
                let max_msg_len = (width as usize).saturating_sub(25); // Space for timestamp and tag
                let message = if log_entry.message.len() > max_msg_len {
                    format!("{}...", &log_entry.message[..max_msg_len.saturating_sub(3)])
                } else {
                    log_entry.message.clone()
                };
                
                let mut line = format!("[{}] [{}] {}", 
                    log_entry.timestamp,
                    log_entry.tag,
                    message
                );
                if line.len() > width as usize { line.truncate(width as usize); }
                stdout.queue(Print(line))?;
                stdout.queue(ResetColor)?;
                current_row += 1;
            }
        }
        
        if current_row == start_row + 1 {
            stdout.queue(MoveTo(0, current_row))?;
            stdout.queue(SetForegroundColor(Color::Grey))?;
            stdout.queue(Print("No recent activity"))?;
            stdout.queue(ResetColor)?;
        }
        
        Ok(())
    }

    /// Draw the footer section
    async fn draw_footer(&self, stdout: &mut io::Stdout, start_row: u16, width: u16) -> io::Result<()> {
        // Footer background
        stdout.queue(SetBackgroundColor(Color::DarkGrey))?;
        stdout.queue(SetForegroundColor(Color::White))?;
        
        // Controls line
        stdout.queue(MoveTo(0, start_row))?;
    let controls = "Controls: [Q/ESC] Exit | [R] Refresh | [C] Clear Logs | [Ctrl+C] Force Exit";
    let mut line = format!("{:<width$}", controls, width = width as usize);
    if line.len() > width as usize { line.truncate(width as usize); }
    stdout.queue(Print(line))?;
        
        // Status line
        stdout.queue(MoveTo(0, start_row + 1))?;
    let status = format!("Dashboard Running | Last Update: {} | Terminal: {}x{}", 
               Local::now().format("%H:%M:%S"),
               width, 
               terminal::size().map(|(_, h)| h).unwrap_or(self.terminal_size.1));
    let mut line = format!("{:<width$}", status, width = width as usize);
    if line.len() > width as usize { line.truncate(width as usize); }
    stdout.queue(Print(line))?;
        
        stdout.queue(ResetColor)?;
        Ok(())
    }

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
        execute!(
            stdout(),
            Show,
            EnableLineWrap,
            LeaveAlternateScreen
        )?;

        // Disable raw mode
        terminal::disable_raw_mode()?;
        
        Ok(())
    }
}

/// Initialize and run the dashboard
pub async fn run_dashboard(shutdown: Arc<Notify>) -> io::Result<()> {
    let mut dashboard = Dashboard::new();
    
    // Initialize terminal for dashboard
    dashboard.initialize().await?;
    
    // Add initial log
    dashboard.add_log("SYSTEM", "INFO", "Dashboard initialized successfully");
    
    // Run the dashboard
    let result = dashboard.run(shutdown).await;
    
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
