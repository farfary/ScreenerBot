/// Test script to demonstrate the new modern panel borders
use crossterm::{
    execute,
    style::{Color, Print, SetForegroundColor, ResetColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, stdout};

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

    /// Double-line border style
    pub const DOUBLE: BorderChars = BorderChars {
        top_left: '╔',
        top_right: '╗',
        bottom_left: '╚',
        bottom_right: '╝',
        horizontal: '═',
        vertical: '║',
        top_tee: '╦',
        bottom_tee: '╩',
        left_tee: '╠',
        right_tee: '╣',
        cross: '╬',
    };

    /// Heavy border style
    pub const HEAVY: BorderChars = BorderChars {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '━',
        vertical: '┃',
        top_tee: '┯',
        bottom_tee: '┷',
        left_tee: '┝',
        right_tee: '┥',
        cross: '╋',
    };

    /// Generate a top border line
    pub fn top_border(&self, width: u16) -> String {
        if width < 2 { return String::new(); }
        format!("{}{}{}", 
            self.top_left,
            self.horizontal.to_string().repeat(width as usize - 2),
            self.top_right
        )
    }

    /// Generate a bottom border line
    pub fn bottom_border(&self, width: u16) -> String {
        if width < 2 { return String::new(); }
        format!("{}{}{}", 
            self.bottom_left,
            self.horizontal.to_string().repeat(width as usize - 2),
            self.bottom_right
        )
    }

    /// Generate a separator line (horizontal divider)
    pub fn separator(&self, width: u16) -> String {
        if width < 2 { return String::new(); }
        format!("{}{}{}", 
            self.left_tee,
            self.horizontal.to_string().repeat(width as usize - 2),
            self.right_tee
        )
    }

    /// Wrap text content with vertical borders
    pub fn wrap_content(&self, content: &str, width: u16) -> String {
        if width < 2 { return String::new(); }
        let max_content = width as usize - 2;
        let padded_content = if content.len() > max_content {
            content[..max_content].to_string()
        } else {
            format!("{:<width$}", content, width = max_content)
        };
        format!("{}{}{}", self.vertical, padded_content, self.vertical)
    }
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All))?;
    
    let width = 60u16;
    
    println!("Modern Dashboard Panel Borders Preview\n");
    
    // Rounded style example (Header)
    execute!(stdout, SetForegroundColor(Color::DarkCyan))?;
    println!("{}", BorderChars::ROUNDED.top_border(width));
    execute!(stdout, SetForegroundColor(Color::White))?;
    println!("{}", BorderChars::ROUNDED.wrap_content("SCREENERBOT DASHBOARD", width));
    execute!(stdout, SetForegroundColor(Color::DarkCyan))?;
    println!("{}", BorderChars::ROUNDED.bottom_border(width));
    execute!(stdout, ResetColor)?;
    
    println!();
    
    // Rounded style example (Positions)
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("{}", BorderChars::ROUNDED.top_border(width));
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("{}", BorderChars::ROUNDED.wrap_content("POSITIONS", width));
    println!("{}", BorderChars::ROUNDED.separator(width));
    execute!(stdout, SetForegroundColor(Color::Green))?;
    println!("{}", BorderChars::ROUNDED.wrap_content("Open Positions (3)", width));
    execute!(stdout, SetForegroundColor(Color::Yellow))?;
    println!("{}", BorderChars::ROUNDED.wrap_content("Symbol       Entry        Current      P&L SOL   P&L %", width));
    execute!(stdout, SetForegroundColor(Color::Green))?;
    println!("{}", BorderChars::ROUNDED.wrap_content("PUMP     0.005000000  0.006200000  +0.001200000  +24.0%", width));
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("{}", BorderChars::ROUNDED.bottom_border(width));
    execute!(stdout, ResetColor)?;
    
    println!();
    
    // Heavy style example (Statistics)
    execute!(stdout, SetForegroundColor(Color::Magenta))?;
    println!("{}", BorderChars::HEAVY.top_border(width));
    println!("{}", BorderChars::HEAVY.wrap_content("STATISTICS", width));
    println!("{}", BorderChars::HEAVY.separator(width));
    execute!(stdout, SetForegroundColor(Color::White))?;
    println!("{}", BorderChars::HEAVY.wrap_content("Wallet: 1.234567890 SOL   Open: 3   Win Rate: 67.5%", width));
    execute!(stdout, SetForegroundColor(Color::Green))?;
    println!("{}", BorderChars::HEAVY.wrap_content("P&L: +0.123456789 SOL    RPC: 1,234   Success: 95.2%", width));
    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    println!("{}", BorderChars::HEAVY.wrap_content("Tx Total: 456   New: 12   Priority: 3", width));
    execute!(stdout, SetForegroundColor(Color::Magenta))?;
    println!("{}", BorderChars::HEAVY.bottom_border(width));
    execute!(stdout, ResetColor)?;
    
    println!();
    
    // Double style example (Activity Log)
    execute!(stdout, SetForegroundColor(Color::White))?;
    println!("{}", BorderChars::DOUBLE.top_border(width));
    println!("{}", BorderChars::DOUBLE.wrap_content("ACTIVITY LOG", width));
    println!("{}", BorderChars::DOUBLE.separator(width));
    execute!(stdout, SetForegroundColor(Color::Green))?;
    println!("{}", BorderChars::DOUBLE.wrap_content("[21:30:15] [TRADER] BUY order executed successfully", width));
    execute!(stdout, SetForegroundColor(Color::Blue))?;
    println!("{}", BorderChars::DOUBLE.wrap_content("[21:30:12] [RPC] Price update: PUMP = 0.006200000 SOL", width));
    execute!(stdout, SetForegroundColor(Color::Yellow))?;
    println!("{}", BorderChars::DOUBLE.wrap_content("[21:30:10] [ENTRY] New token discovered: PUMP", width));
    execute!(stdout, SetForegroundColor(Color::White))?;
    println!("{}", BorderChars::DOUBLE.bottom_border(width));
    execute!(stdout, ResetColor)?;
    
    println!("\nBorder styles implemented:");
    println!("• Header: Rounded borders with cyan accent");
    println!("• Positions: Rounded borders with section separators");
    println!("• Statistics: Heavy borders for importance");
    println!("• Activity Log: Double borders for log entries");
    println!("• Footer: Rounded borders with status info");
    
    Ok(())
}
