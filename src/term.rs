//! Terminal setup and teardown.

use std::io::{self, Stdout, stdout};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor, execute};

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Raw mode, alternate screen, mouse capture, bracketed paste, hidden cursor.
/// Installs a panic hook that restores the terminal first.
pub fn setup() -> io::Result<Term> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        previous(info);
    }));
    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        cursor::Hide
    )?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Undoes [`setup`]. Safe to call more than once.
pub fn restore() -> io::Result<()> {
    let raw = disable_raw_mode();
    execute!(
        stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    )?;
    raw
}

/// The terminal's cell size in pixels, if it reports one.
pub fn cell_pixel_size() -> Option<[u16; 2]> {
    let ws = ratatui::crossterm::terminal::window_size().ok()?;
    if ws.width == 0 || ws.height == 0 || ws.columns == 0 || ws.rows == 0 {
        return None;
    }
    Some([ws.width / ws.columns, ws.height / ws.rows])
}
