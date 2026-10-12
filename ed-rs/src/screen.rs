//! Full-screen terminal UI for the Ed reimplementation (crossterm).
//!
//! Renders a fixed-size window like the original Ed's own console
//! window ("Ed 1.14"): a title bar, 10 text lines, and a status line
//! at the bottom of the window.
//!
//! Faithful to the behavior spec (docs/ed-spec.md):
//! - bottom line is the status/message line; messages persist until the
//!   next command
//! - ESC enters extended mode; RETURN executes and returns to immediate
//!   mode; ESC executes and stays in extended mode
//! - CTRL-G repeats the last extended command line
//! - X exits with save; Q exits without save (with confirmation if
//!   there are unsaved edits)

use crate::buffer::Buffer;
use crate::cmdline::{self, Editor};
use crossterm::{
    cursor::{MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Result as CResult, Stdout, Write};

/// One extra row for the status line.
const STATUS_ROW: u16 = 1;

/// Window title, like the original's "Ed 1.14" title bar.
const TITLE: &str = " Neo-Ed";

/// Text lines visible inside the window. The original Ed opened a
/// 640x200 window (~12 text lines at the 8x16 font); we use 10.
const TEXT_ROWS: u16 = 10;

/// Full window height: title bar + text rows + status line.
const WINDOW_ROWS: u16 = TEXT_ROWS + STATUS_ROW + 1;

/// Prompt shown when quitting would discard unsaved edits (spec 5.4).
const CONFIRM_PROMPT: &str = "Edits will be lost - type Y to confirm: ";

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Immediate,
    Extended(String),
    QuitConfirm,
    ExchangeConfirm,
    ShDisplay,
}

/// What the main loop should do after a key has been processed.
#[derive(Debug, PartialEq)]
pub enum Decision {
    Continue,
    Quit { save: bool },
}

pub struct Screen {
    stdout: Stdout,
    pub rows: u16,
    pub cols: u16,
}

impl Screen {
    pub fn new() -> CResult<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        let (cols, rows) = terminal::size()?;
        Ok(Screen { stdout, rows, cols })
    }

    pub fn cleanup(&mut self) -> CResult<()> {
        terminal::disable_raw_mode()?;
        execute!(self.stdout, Show, LeaveAlternateScreen)
    }

    pub fn resize(&mut self) {
        if let Ok((c, r)) = terminal::size() {
            self.cols = c;
            self.rows = r;
        }
    }

    /// Total window height (title + text + status), clamped to the
    /// terminal size on very small terminals.
    pub fn window_rows(&self) -> u16 {
        self.rows.min(WINDOW_ROWS)
    }

    /// Text rows visible inside the window (between title and status).
    pub fn text_rows(&self) -> usize {
        self.window_rows().saturating_sub(STATUS_ROW + 1) as usize
    }

    pub fn render(
        &mut self,
        ed: &Editor,
        mode: &Mode,
        message: &Option<String>,
    ) -> io::Result<()> {
        queue!(self.stdout, Clear(ClearType::All))?;
        let b = &ed.buffer;
        let text_rows = self.text_rows();
        let status_row = self.window_rows() - STATUS_ROW;
        let top = b.top;

        // title bar (row 0): white on black, like an Amiga window title
        queue!(
            self.stdout,
            MoveTo(0, 0),
            SetBackgroundColor(crossterm::style::Color::White),
            SetForegroundColor(crossterm::style::Color::Black),
            Print(format!("{:<width$}", TITLE, width = self.cols as usize)),
            ResetColor,
        )?;

        // draw visible lines
        for (row, i) in (top..b.line_count()).enumerate() {
            if row >= text_rows {
                break;
            }
            let line = &b.lines[i];
            let chars: Vec<char> = line.chars().collect();
            let visible: String = chars.iter().skip(b.left).take(self.cols as usize).collect();
            queue!(self.stdout, MoveTo(0, row as u16 + 1), Print(&visible))?;
        }

        // status line (bottom)
        let status: String = match mode {
            Mode::Extended(s) => format!("*{}", s),
            Mode::QuitConfirm => "Edits will be lost - type Y to confirm: ".to_string(),
            Mode::ExchangeConfirm => "Exchange? ".to_string(),
            Mode::ShDisplay => "Type any character to continue ".to_string(),
            Mode::Immediate => message
                .as_ref()
                .map(|m| m.clone())
                .or_else(|| ed.last_message.clone())
                .unwrap_or_default(),
        };
        // status line (bottom of the window, reverse video like the
        // title bar so the window edge is clearly visible)
        queue!(
            self.stdout,
            MoveTo(0, status_row),
            SetBackgroundColor(crossterm::style::Color::White),
            SetForegroundColor(crossterm::style::Color::Black),
            Print(format!("{:<width$}", status, width = self.cols as usize)),
            ResetColor,
        )?;

        // SH display replaces the screen (spec section 7)
        if matches!(mode, Mode::ShDisplay) {
            let sh = ed.sh_text();
            queue!(self.stdout, Clear(ClearType::All))?;
            for (row, line) in sh.lines().enumerate() {
                if row >= text_rows {
                    break;
                }
                queue!(self.stdout, MoveTo(0, row as u16 + 1), Print(line))?;
            }
            queue!(
                self.stdout,
                MoveTo(0, status_row),
                SetBackgroundColor(crossterm::style::Color::White),
                SetForegroundColor(crossterm::style::Color::Black),
                Print(format!(
                    "{:<width$}",
                    "Type any character to continue ",
                    width = self.cols as usize
                )),
                ResetColor,
            )?;
            // rest the visible cursor just after the continue prompt
            queue!(self.stdout, MoveTo(33, status_row))?;
            self.stdout.flush()?;
            return Ok(());
        }

        // cursor position
        let screen_row = b.cursor.line.saturating_sub(top) as u16 + 1;
        let screen_col = b.cursor.col.saturating_sub(b.left) as u16;
        queue!(self.stdout, MoveTo(screen_col, screen_row))?;
        self.stdout.flush()
    }

    pub fn scroll_follow(&mut self, b: &mut Buffer) {
        let text_rows = self.text_rows();
        if b.cursor.line < b.top {
            b.top = b.cursor.line;
        } else if b.cursor.line >= b.top + text_rows {
            b.top = b.cursor.line + 1 - text_rows;
        }
        let cols = self.cols as usize;
        if b.cursor.col < b.left {
            b.left = (b.cursor.col / 10) * 10;
        } else if b.cursor.col >= b.left + cols {
            b.left = ((b.cursor.col + 1 - cols) / 10) * 10 + 10;
        }
    }
}

/// Handle a key press in immediate mode. Returns (new_mode, quit).
fn handle_immediate(
    ed: &mut Editor,
    code: KeyCode,
    mods: KeyModifiers,
    rows: usize,
) -> (Mode, bool, Option<String>) {
    let mut msg = None;
    let mut quit = false;
    let mut mode = Mode::Immediate;

    // Ctrl-key combinations
    if mods.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = code {
            match c {
                'c' => {
                    quit = true;
                }
                'a' => {
                    ed.buffer.insert_after("");
                    ed.dirty = true;
                }
                'b' => {
                    if let Some(m) = ed.buffer.delete_line() {
                        msg = Some(m.to_string());
                    }
                    ed.dirty = true;
                }
                'd' => {
                    ed.buffer.cursor.line = ed.buffer.cursor.line.saturating_sub(rows);
                    ed.buffer.cursor.col = 0;
                    ed.buffer.follow_cursor_line();
                }
                'e' => {
                    let b = &mut ed.buffer;
                    if b.cursor.line == b.top {
                        // already at top of screen → move to bottom
                        let last = (b.top + rows.saturating_sub(1)).min(b.line_count() - 1);
                        b.cursor.line = last;
                    } else {
                        b.cursor.line = b.top;
                    }
                    b.cursor.col = 0;
                    b.follow_cursor_line();
                }
                'f' => {
                    let b = &mut ed.buffer;
                    let line = b.current_line().clone();
                    if let Some(c) = line.chars().nth(b.cursor.col) {
                        if c.is_ascii_alphabetic() {
                            let flipped = if c.is_uppercase() {
                                c.to_lowercase().next().unwrap()
                            } else {
                                c.to_uppercase().next().unwrap()
                            };
                            b.lines[b.cursor.line].remove(b.cursor.col);
                            b.lines[b.cursor.line].insert(b.cursor.col, flipped);
                            b.cursor.col += 1;
                            ed.dirty = true;
                        } else {
                            b.cursor.col += 1;
                        }
                    }
                }
                'g' => {
                    // repeat last extended command (handled by caller)
                }
                'h' => {
                    ed.buffer.delete_left();
                    ed.dirty = true;
                }
                'i' => {
                    // tab: move to next tab stop
                    let b = &mut ed.buffer;
                    let col = b.cursor.col;
                    let next = (col / ed.tab_distance + 1) * ed.tab_distance;
                    b.cursor.col = next.min(b.current_line().chars().count().max(1));
                }
                'o' => {
                    let b = &mut ed.buffer;
                    let line = b.current_line().clone();
                    let col = b.cursor.col.min(line.chars().count());
                    let is_space = line.chars().nth(col).map(|c| c == ' ').unwrap_or(true);
                    if is_space {
                        // delete all spaces up to next nonblank
                        let start = col;
                        let end = line[start..]
                            .find(|c: char| c != ' ')
                            .map(|i| start + i)
                            .unwrap_or(line.chars().count());
                        let sb = char_to_byte(&line, start);
                        let eb = char_to_byte(&line, end);
                        b.lines[b.cursor.line].replace_range(sb..eb, "");
                    } else {
                        // delete from cursor to first space
                        let end = line[col..]
                            .find(' ')
                            .map(|i| col + i)
                            .unwrap_or(line.chars().count());
                        let sb = char_to_byte(&line, col);
                        let eb = char_to_byte(&line, end);
                        b.lines[b.cursor.line].replace_range(sb..eb, "");
                    }
                    ed.dirty = true;
                }
                'r' => {
                    let b = &mut ed.buffer;
                    let line = b.current_line().clone();
                    let col = b.cursor.col;
                    let start = line[..char_to_byte(&line, col)]
                        .rfind(' ')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    b.cursor.col = byte_to_char(&line, start);
                }
                't' => {
                    let b = &mut ed.buffer;
                    let line = b.current_line().clone();
                    let col = b.cursor.col;
                    if let Some(off) = line[col..].find(' ') {
                        b.cursor.col = col + off + 1;
                    } else {
                        b.cursor.col = line.chars().count();
                    }
                }
                'u' => {
                    ed.buffer.cursor.line =
                        (ed.buffer.cursor.line + rows).min(ed.buffer.line_count() - 1);
                    ed.buffer.cursor.col = 0;
                    ed.buffer.follow_cursor_line();
                }
                'v' => {
                    // full redraw happens on next render anyway
                }
                'y' => {
                    let b = &mut ed.buffer;
                    let col = b.cursor.col;
                    let line = &mut b.lines[b.cursor.line];
                    let cb = char_to_byte(line, col);
                    line.truncate(cb);
                    ed.dirty = true;
                }
                _ => {}
            }
            return (mode, quit, msg);
        }
    }

    match code {
        KeyCode::Esc => {
            mode = Mode::Extended(String::new());
        }
        KeyCode::Backspace => {
            ed.buffer.delete_left();
            ed.dirty = true;
        }
        KeyCode::Delete => {
            ed.buffer.delete_at();
            ed.dirty = true;
        }
        KeyCode::Enter => {
            ed.buffer.split_line(ed.left_margin);
            ed.dirty = true;
        }
        KeyCode::Tab => {
            let b = &mut ed.buffer;
            let col = b.cursor.col;
            let next = (col / ed.tab_distance + 1) * ed.tab_distance;
            b.cursor.col = next.min(b.current_line().chars().count().max(1));
        }
        KeyCode::Up => {
            let b = &mut ed.buffer;
            if b.cursor.line == 0 {
                msg = Some("Top of file".to_string());
            } else {
                b.cursor.line -= 1;
                b.follow_cursor_line();
            }
        }
        KeyCode::Down => {
            let b = &mut ed.buffer;
            if b.cursor.line + 1 >= b.line_count() {
                msg = Some("End of file".to_string());
            } else {
                b.cursor.line += 1;
                b.follow_cursor_line();
            }
        }
        KeyCode::Left => {
            ed.buffer.cursor.col = ed.buffer.cursor.col.saturating_sub(1);
        }
        KeyCode::Right => {
            ed.buffer.cursor.col += 1;
        }
        KeyCode::Home => {
            ed.buffer.cursor.col = 0;
        }
        KeyCode::End => {
            ed.buffer.cursor.col = ed.buffer.current_line().chars().count();
        }
        KeyCode::Char(c) => {
            match ed.buffer.insert_char(c, ed.right_margin) {
                Ok(_) => {
                    ed.dirty = true;
                }
                Err(e) => {
                    msg = Some(e.to_string());
                }
            }
        }
        _ => {}
    }
    (mode, quit, msg)
}

/// Execute an extended command line and return (quit, save_before_quit).
fn exec_command_line(ed: &mut Editor, line: &str) -> (bool, bool) {
    match cmdline::parse_line(line) {
        Ok(items) => {
            let r = ed.run_line(&items);
            if let Some(m) = &r.message {
                ed.last_message = Some(m.clone());
            }
            if r.quit {
                if r.quit_save {
                    return (true, true);
                }
                // Q with unsaved edits: do not quit yet. The command set
                // the confirm prompt in last_message; the prompt check in
                // process_key enters QuitConfirm mode and waits for Y.
                if r.message.as_deref() == Some(CONFIRM_PROMPT) {
                    return (false, false);
                }
                return (true, false);
            }
            (false, false)
        }
        Err(e) => {
            ed.last_message = Some(e.to_string());
            (false, false)
        }
    }
}

/// Save the FROM file and the backup (spec section 6, corrected).
fn save_files(ed: &Editor) {
    let text = ed.buffer.to_text();
    let _ = std::fs::write(&ed.from_file, &text);
    let backup = std::path::Path::new("t").join("ed-backup");
    if let Some(parent) = backup.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&backup, &text);
}

/// Process one key press against the editor state machine.
///
/// This is the testable core of the TUI: no terminal I/O happens here,
/// all rendering and saving stays in run_loop. Returns the new mode,
/// what the loop should do next, and a message for the status line.
fn process_key(
    ed: &mut Editor,
    mode: Mode,
    code: KeyCode,
    modifiers: KeyModifiers,
    rows: usize,
    last_command: &mut String,
) -> (Mode, Decision, Option<String>) {
    let mut message: Option<String> = None;

    let mut mode = match mode {
        Mode::Immediate => {
            let (new_mode, quit, msg) = handle_immediate(ed, code, modifiers, rows);
            message = msg;
            if quit {
                return (new_mode, Decision::Quit { save: false }, message);
            }
            // CTRL-G: repeat the last extended command
            if modifiers.contains(KeyModifiers::CONTROL) {
                if let KeyCode::Char('g') = code {
                    if !last_command.is_empty() {
                        let (q, save) = exec_command_line(ed, last_command);
                        if q {
                            return (new_mode, Decision::Quit { save }, message);
                        }
                    }
                }
            }
            let mut m = new_mode;
            if ed.sh_show {
                m = Mode::ShDisplay;
                ed.sh_show = false;
            } else if ed.pending_exchange.is_some() {
                m = Mode::ExchangeConfirm;
            }
            m
        }
        Mode::Extended(linebuf) => {
            match (code, modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Enter, _) => {
                    // execute; RETURN returns to immediate mode, ESC stays
                    let (q, save) = exec_command_line(ed, &linebuf);
                    *last_command = linebuf;
                    if q {
                        return (Mode::Immediate, Decision::Quit { save }, message);
                    }
                    let mut m = if code == KeyCode::Enter {
                        Mode::Immediate
                    } else {
                        Mode::Extended(String::new())
                    };
                    if ed.sh_show {
                        m = Mode::ShDisplay;
                        ed.sh_show = false;
                    } else if ed.pending_exchange.is_some() {
                        m = Mode::ExchangeConfirm;
                    }
                    m
                }
                (KeyCode::Backspace, _) => {
                    let mut lb = linebuf;
                    lb.pop();
                    Mode::Extended(lb)
                }
                (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                    let mut lb = linebuf;
                    lb.push(c);
                    Mode::Extended(lb)
                }
                _ => Mode::Extended(linebuf),
            }
        }
        Mode::QuitConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                return (Mode::Immediate, Decision::Quit { save: false }, message);
            }
            _ => Mode::Immediate,
        },
        Mode::ExchangeConfirm => {
            let pair = ed.pending_exchange.take();
            match (code, pair) {
                (KeyCode::Char('y') | KeyCode::Char('Y'), Some((needle, repl))) => {
                    let b = &mut ed.buffer;
                    let line = &mut b.lines[b.cursor.line];
                    let cb = char_to_byte(line, b.cursor.col);
                    let nb = needle.len();
                    line.replace_range(cb..cb + nb, &repl);
                    b.cursor.col += repl.chars().count();
                    ed.dirty = true;
                }
                (KeyCode::Char('n') | KeyCode::Char('N'), _) => {
                    ed.buffer.cursor.col += 1;
                }
                _ => {}
            }
            Mode::Immediate
        }
        Mode::ShDisplay => Mode::Immediate,
    };

    // Q with unsaved edits → wait for Y before quitting
    if ed.last_message.as_deref() == Some(CONFIRM_PROMPT) {
        mode = Mode::QuitConfirm;
        ed.last_message = None;
    }

    (mode, Decision::Continue, message)
}

/// Run the interactive editor loop.
pub fn run_loop(ed: &mut Editor) -> CResult<()> {
    let mut screen = Screen::new()?;
    let mut mode = Mode::Immediate;
    let mut message: Option<String> = None;
    let mut last_command: String = String::new();

    loop {
        screen.resize();
        screen.scroll_follow(&mut ed.buffer);
        screen.render(ed, &mode, &message)?;
        message = None;
        ed.last_message = None;

        let ev = event::read()?;
        if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = ev {
            if kind == KeyEventKind::Release {
                continue;
            }
            let window_text_rows = screen.text_rows();
            let (new_mode, decision, msg) = process_key(
                ed,
                mode,
                code,
                modifiers,
                window_text_rows,
                &mut last_command,
            );
            mode = new_mode;
            message = msg;
            match decision {
                Decision::Quit { save } => {
                    if save {
                        save_files(ed);
                    }
                    screen.cleanup()?;
                    return Ok(());
                }
                Decision::Continue => {}
            }
        }
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn byte_to_char(s: &str, bi: usize) -> usize {
    s.char_indices().take_while(|(b, _)| *b <= bi).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: usize = 10; // text rows inside the window

    fn ed_with(text: &str) -> Editor {
        Editor::new("test.txt", Some(text.to_string()))
    }

    fn press(
        ed: &mut Editor,
        mode: Mode,
        code: KeyCode,
        modifiers: KeyModifiers,
        last: &mut String,
    ) -> (Mode, Decision, Option<String>) {
        process_key(ed, mode, code, modifiers, ROWS, last)
    }

    /// Type an extended command line (after ESC) and press RETURN.
    fn run_extended(ed: &mut Editor, line: &str, last: &mut String) -> Mode {
        let mut mode = press(ed, Mode::Immediate, KeyCode::Esc, KeyModifiers::NONE, last).0;
        for c in line.chars() {
            mode = press(ed, mode, KeyCode::Char(c), KeyModifiers::NONE, last).0;
        }
        press(ed, mode, KeyCode::Enter, KeyModifiers::NONE, last)
            .0
    }

    #[test]
    fn immediate_typing_inserts_and_dirties() {
        let mut ed = ed_with("one\ntwo\n");
        let mut last = String::new();
        let (mode, decision, msg) = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('X'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);
        assert!(msg.is_none());
        assert_eq!(ed.buffer.lines[0], "Xone");
        assert!(ed.dirty);
    }

    #[test]
    fn immediate_up_at_top_reports_top_of_file() {
        let mut ed = ed_with("one\n");
        let mut last = String::new();
        let (_, _, msg) = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Up,
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(msg.as_deref(), Some("Top of file"));
        assert_eq!(ed.buffer.cursor.line, 0);
    }

    #[test]
    fn esc_enters_extended_and_return_executes() {
        let mut ed = ed_with("a\nb\nc\n");
        let mut last = String::new();

        let mode = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Esc,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        assert_eq!(mode, Mode::Extended(String::new()));

        let mode = press(
            &mut ed,
            mode,
            KeyCode::Char('N'),
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        assert_eq!(mode, Mode::Extended("N".to_string()));

        let (mode, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Enter,
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);
        assert_eq!(ed.buffer.cursor.line, 1);
        assert_eq!(last, "N");
    }

    #[test]
    fn esc_executes_command_and_stays_extended() {
        let mut ed = ed_with("a\nb\nc\n");
        let mut last = String::new();
        let mut mode = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Esc,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        for c in "N".chars() {
            mode = press(&mut ed, mode, KeyCode::Char(c), KeyModifiers::NONE, &mut last).0;
        }
        let (mode, _, _) = press(&mut ed, mode, KeyCode::Esc, KeyModifiers::NONE, &mut last);
        assert_eq!(mode, Mode::Extended(String::new()));
        assert_eq!(ed.buffer.cursor.line, 1);
    }

    #[test]
    fn extended_backspace_edits_command_line() {
        let mut ed = ed_with("a\n");
        let mut last = String::new();
        let mut mode = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Esc,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        for c in "SH".chars() {
            mode = press(&mut ed, mode, KeyCode::Char(c), KeyModifiers::NONE, &mut last).0;
        }
        let mode = press(
            &mut ed,
            mode,
            KeyCode::Backspace,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        assert_eq!(mode, Mode::Extended("S".to_string()));
    }

    #[test]
    fn ctrl_g_repeats_last_extended_command() {
        let mut ed = ed_with("a\nb\nc\nd\n");
        let mut last = String::new();
        run_extended(&mut ed, "N", &mut last);
        assert_eq!(ed.buffer.cursor.line, 1);

        let (mode, decision, _) = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('g'),
            KeyModifiers::CONTROL,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);
        assert_eq!(ed.buffer.cursor.line, 2);
    }

    #[test]
    fn q_with_unsaved_edits_asks_confirmation() {
        let mut ed = ed_with("one\n");
        let mut last = String::new();
        // dirty the buffer by typing
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert!(ed.dirty);

        // Q must NOT quit; it must enter QuitConfirm
        let mode = run_extended(&mut ed, "Q", &mut last);
        assert_eq!(mode, Mode::QuitConfirm);

        // any other key than Y returns to editing
        let (mode, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Char('n'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);

        // Y confirms the quit without saving
        let mode = run_extended(&mut ed, "Q", &mut last);
        assert_eq!(mode, Mode::QuitConfirm);
        let (_, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Char('y'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(decision, Decision::Quit { save: false });
    }

    #[test]
    fn q_with_clean_buffer_quits_immediately() {
        let mut ed = ed_with("one\n");
        assert!(!ed.dirty);
        let mut last = String::new();
        let mut mode = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Esc,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        mode = press(
            &mut ed,
            mode,
            KeyCode::Char('Q'),
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        let (_, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Enter,
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(decision, Decision::Quit { save: false });
    }

    #[test]
    fn x_quits_with_save() {
        let mut ed = ed_with("one\n");
        let mut last = String::new();
        let mut mode = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Esc,
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        mode = press(
            &mut ed,
            mode,
            KeyCode::Char('X'),
            KeyModifiers::NONE,
            &mut last,
        )
        .0;
        let (_, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Enter,
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(decision, Decision::Quit { save: true });
    }

    #[test]
    fn eq_query_exchange_confirms_before_replacing() {
        let mut ed = ed_with("alpha beta\n");
        let mut last = String::new();
        let mode = run_extended(&mut ed, "EQ/al/XY/", &mut last);
        assert_eq!(mode, Mode::ExchangeConfirm);
        assert!(ed.pending_exchange.is_some());

        let (mode, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Char('y'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);
        assert_eq!(ed.buffer.lines[0], "XYpha beta");
        assert_eq!(ed.buffer.cursor.col, 2);
        assert!(ed.dirty);
        assert!(ed.pending_exchange.is_none());
    }

    #[test]
    fn eq_query_n_skips_the_match() {
        let mut ed = ed_with("alpha beta\n");
        let mut last = String::new();
        let mode = run_extended(&mut ed, "EQ/al/XY/", &mut last);
        let (mode, _, _) = press(
            &mut ed,
            mode,
            KeyCode::Char('n'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(ed.buffer.lines[0], "alpha beta");
        assert_eq!(ed.buffer.cursor.col, 1);
    }

    #[test]
    fn e_exchange_replaces_without_asking() {
        let mut ed = ed_with("alpha beta\n");
        let mut last = String::new();
        let mode = run_extended(&mut ed, "E/al/XY/", &mut last);
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(ed.buffer.lines[0], "XYpha beta");
        assert!(ed.pending_exchange.is_none());
    }

    #[test]
    fn sh_command_shows_display_then_any_key_returns() {
        let mut ed = ed_with("one\n");
        let mut last = String::new();
        let mode = run_extended(&mut ed, "SH", &mut last);
        assert_eq!(mode, Mode::ShDisplay);
        assert!(!ed.sh_show); // consumed by the mode switch

        let (mode, decision, _) = press(
            &mut ed,
            mode,
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(mode, Mode::Immediate);
        assert_eq!(decision, Decision::Continue);
    }

    #[test]
    fn ctrl_c_quits_from_immediate_mode() {
        let mut ed = ed_with("one\n");
        let mut last = String::new();
        let (_, decision, _) = press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            &mut last,
        );
        assert_eq!(decision, Decision::Quit { save: false });
    }

    #[test]
    fn u_restores_edited_current_line() {
        let mut ed = ed_with("hello\nworld\n");
        let mut last = String::new();
        // type X at the front of line 0
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('X'),
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(ed.buffer.lines[0], "Xhello");

        run_extended(&mut ed, "U", &mut last);
        assert_eq!(ed.buffer.lines[0], "hello");
        assert_eq!(ed.buffer.cursor.line, 0);
    }

    #[test]
    fn u_commits_when_cursor_leaves_the_line() {
        let mut ed = ed_with("hello\nworld\n");
        let mut last = String::new();
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Char('X'),
            KeyModifiers::NONE,
            &mut last,
        );
        // move away and come back: the edit is committed (spec 5.3)
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Down,
            KeyModifiers::NONE,
            &mut last,
        );
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Up,
            KeyModifiers::NONE,
            &mut last,
        );
        run_extended(&mut ed, "U", &mut last);
        assert_eq!(ed.buffer.lines[0], "Xhello");
    }

    #[test]
    fn u_does_not_resurrect_deleted_line() {
        let mut ed = ed_with("one\ntwo");
        let mut last = String::new();
        run_extended(&mut ed, "D", &mut last);
        assert_eq!(ed.buffer.line_count(), 1);
        run_extended(&mut ed, "U", &mut last);
        assert_eq!(ed.buffer.line_count(), 1);
        assert_eq!(ed.buffer.lines[0], "two");
    }

    #[test]
    fn u_after_split_keeps_the_split() {
        let mut ed = ed_with("hello world");
        let mut last = String::new();
        // cursor to end of line, then RETURN splits it
        run_extended(&mut ed, "CE", &mut last);
        press(
            &mut ed,
            Mode::Immediate,
            KeyCode::Enter,
            KeyModifiers::NONE,
            &mut last,
        );
        assert_eq!(ed.buffer.line_count(), 2);
        run_extended(&mut ed, "U", &mut last);
        assert_eq!(ed.buffer.line_count(), 2);
        assert_eq!(ed.buffer.lines[0], "hello world");
        assert_eq!(ed.buffer.lines[1], "");
    }

    /// Verified live against the original (2026-10-13):
    /// `3 A /xx/` inserts three lines in reading order; UC makes
    /// F case-insensitive and LC restores the distinction.
    #[test]
    fn repeat_count_and_case_sensitivity() {
        // 3 A /xx/ from the end of LINE2: three lines, reading order
        let mut ed = ed_with("LINE1\nLINE2");
        ed.buffer.cursor.line = 1;
        let mut last = String::new();
        run_extended(&mut ed, "3 A /xx/", &mut last);
        assert_eq!(ed.buffer.lines, vec!["LINE1", "LINE2", "xx", "xx", "xx"]);

        // UC then wrong-case search finds it
        let mut ed2 = ed_with("LINE1\nLINE2");
        let mut last2 = String::new();
        run_extended(&mut ed2, "UC; F /line2/", &mut last2);
        assert_eq!(ed2.buffer.cursor.line, 1);

        // LC restores distinction: same search now fails
        run_extended(&mut ed2, "LC; F /line2/", &mut last2);
        assert_eq!(ed2.last_message.as_deref(), Some("search failed"));
    }
}
