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

/// Prepares the terminal: raw mode, alternate screen, mouse capture, bracketed paste.
/// Installs a panic hook that restores the terminal.
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

/// Undoes `setup`.
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

/// Measures the terminal's cell size in pixels, or `None` if the terminal doesn't report it.
pub fn cell_pixel_size() -> Option<[u16; 2]> {
    let ws = ratatui::crossterm::terminal::window_size().ok()?;
    let cell = [
        ws.width.checked_div(ws.columns)?,
        ws.height.checked_div(ws.rows)?,
    ];
    (!cell.contains(&0)).then_some(cell)
}
