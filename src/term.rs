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
///
/// # Returns
/// The ratatui terminal; a panic hook restores the terminal if the program panics.
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
///
/// # Returns
/// An error if the terminal couldn't be written to.
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

/// Measures the terminal's cell size.
///
/// # Returns
/// Width and height in pixels, or `None` if the terminal doesn't report them.
pub fn cell_pixel_size() -> Option<[u16; 2]> {
    let ws = ratatui::crossterm::terminal::window_size().ok()?;
    if ws.width == 0 || ws.height == 0 || ws.columns == 0 || ws.rows == 0 {
        return None;
    }
    Some([ws.width / ws.columns, ws.height / ws.rows])
}
