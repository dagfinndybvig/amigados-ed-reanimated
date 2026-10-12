//! Full-screen terminal UI for the Ed reimplementation (crossterm).
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
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Result as CResult, Stdout, Write};

/// One extra row for the status line.
const STATUS_ROW: u16 = 1;

pub enum Mode {
    Immediate,
    Extended(String),
    QuitConfirm,
    ExchangeConfirm,
    ShDisplay,
}

pub struct Screen {
    stdout: Stdout,
    pub rows: u16,
    pub cols: u16,
}

impl Screen {
    pub fn new() -> CResult<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
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

    pub fn render(
        &mut self,
        ed: &Editor,
        mode: &Mode,
        message: &Option<String>,
    ) -> io::Result<()> {
        queue!(self.stdout, Clear(ClearType::All))?;
        let b = &ed.buffer;
        let text_rows = (self.rows - STATUS_ROW) as usize;
        let top = b.top;

        // draw visible lines
        for (row, i) in (top..b.line_count()).enumerate() {
            if row >= text_rows {
                break;
            }
            let line = &b.lines[i];
            let chars: Vec<char> = line.chars().collect();
            let visible: String = chars.iter().skip(b.left).take(self.cols as usize).collect();
            queue!(self.stdout, MoveTo(0, row as u16), Print(&visible))?;
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
        // if we just executed a command that set last_message, show it
        let status = if matches!(mode, Mode::Immediate) {
            status
        } else {
            status
        };
        queue!(
            self.stdout,
            MoveTo(0, self.rows - STATUS_ROW),
            SetForegroundColor(crossterm::style::Color::White),
            Print(format!("{:<width$}", status, width = self.cols as usize)),
            ResetColor,
        )?;

        // SH display replaces the screen (spec section 7)
        if matches!(mode, Mode::ShDisplay) {
            let sh = ed.sh_text();
            queue!(self.stdout, Clear(ClearType::All))?;
            for (row, line) in sh.lines().enumerate() {
                if (row as u16) >= self.rows - STATUS_ROW {
                    break;
                }
                queue!(self.stdout, MoveTo(0, row as u16), Print(line))?;
            }
            queue!(
                self.stdout,
                MoveTo(0, self.rows - STATUS_ROW),
                Print("Type any character to continue "),
            )?;
            queue!(self.stdout, MoveTo(0, 0))?;
            self.stdout.flush()?;
            return Ok(());
        }

        // cursor position
        let screen_row = b.cursor.line.saturating_sub(top) as u16;
        let screen_col = b.cursor.col.saturating_sub(b.left) as u16;
        queue!(self.stdout, MoveTo(screen_col, screen_row))?;
        self.stdout.flush()
    }

    pub fn scroll_follow(&mut self, b: &mut Buffer) {
        let text_rows = (self.rows - STATUS_ROW) as usize;
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
    cols: usize,
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
                    ed.buffer.cursor.line = ed.buffer.cursor.line.saturating_sub(12);
                    ed.buffer.cursor.col = 0;
                }
                'e' => {
                    let b = &mut ed.buffer;
                    if b.cursor.line == b.top {
                        // already at top of screen → move to bottom
                        let last = (b.top + rows.saturating_sub(2)).min(b.line_count() - 1);
                        b.cursor.line = last;
                    } else {
                        b.cursor.line = b.top;
                    }
                    b.cursor.col = 0;
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
                        (ed.buffer.cursor.line + 12).min(ed.buffer.line_count() - 1);
                    ed.buffer.cursor.col = 0;
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
            }
        }
        KeyCode::Down => {
            let b = &mut ed.buffer;
            if b.cursor.line + 1 >= b.line_count() {
                msg = Some("End of file".to_string());
            } else {
                b.cursor.line += 1;
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
                } else {
                    return (true, false);
                }
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

/// Run the interactive editor loop.
pub fn run_loop(ed: &mut Editor) -> CResult<()> {
    let mut screen = Screen::new()?;
    let mut mode = Mode::Immediate;
    let mut message: Option<String> = None;
    let mut last_command: String = String::new();
    let mut should_quit = false;
    let mut save_on_quit = false;

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
            match mode {
                Mode::Immediate => {
                    let (new_mode, quit, msg) = handle_immediate(
                        ed,
                        code,
                        modifiers,
                        screen.cols as usize,
                        screen.rows as usize,
                    );
                    mode = new_mode;
                    message = msg;
                    if quit {
                        screen.cleanup()?;
                        return Ok(());
                    }
                    // CTRL-G: repeat last extended command
                    if modifiers.contains(KeyModifiers::CONTROL) {
                        if let KeyCode::Char('g') = code {
                            if !last_command.is_empty() {
                                let (q, save) = exec_command_line(ed, &last_command);
                                if q {
                                    if save {
                                        save_files(ed);
                                    }
                                    screen.cleanup()?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    if ed.sh_show {
                        mode = Mode::ShDisplay;
                        ed.sh_show = false;
                    }
                    if ed.pending_exchange.is_some() {
                        mode = Mode::ExchangeConfirm;
                    }
                }
                Mode::Extended(ref linebuf) => {
                    let mut lb = linebuf.clone();
                    match (code, modifiers) {
                        (KeyCode::Esc, _) => {
                            // execute and stay in extended mode
                            let (q, save) = exec_command_line(ed, &lb);
                            last_command = lb;
                            if q {
                                if save {
                                    save_files(ed);
                                }
                                screen.cleanup()?;
                                return Ok(());
                            }
                            if ed.sh_show {
                                mode = Mode::ShDisplay;
                                ed.sh_show = false;
                            } else if ed.pending_exchange.is_some() {
                                mode = Mode::ExchangeConfirm;
                            } else {
                                mode = Mode::Extended(String::new());
                            }
                        }
                        (KeyCode::Enter, _) => {
                            // execute and return to immediate mode
                            let (q, save) = exec_command_line(ed, &lb);
                            last_command = lb;
                            if q {
                                if save {
                                    save_files(ed);
                                }
                                screen.cleanup()?;
                                return Ok(());
                            }
                            if ed.sh_show {
                                mode = Mode::ShDisplay;
                                ed.sh_show = false;
                            } else if ed.pending_exchange.is_some() {
                                mode = Mode::ExchangeConfirm;
                            } else {
                                mode = Mode::Immediate;
                            }
                        }
                        (KeyCode::Backspace, _) => {
                            lb.pop();
                            mode = Mode::Extended(lb);
                        }
                        (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                            lb.push(c);
                            mode = Mode::Extended(lb);
                        }
                        _ => {}
                    }
                }
                Mode::QuitConfirm => match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        screen.cleanup()?;
                        return Ok(());
                    }
                    _ => {
                        mode = Mode::Immediate;
                    }
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
                    mode = Mode::Immediate;
                }
                Mode::ShDisplay => {
                    mode = Mode::Immediate;
                }
            }

            // Q with unsaved edits → confirm before quitting
            if ed.last_message.is_some() {
                let msg = ed.last_message.clone().unwrap();
                if msg == "Edits will be lost - type Y to confirm: " {
                    mode = Mode::QuitConfirm;
                    ed.last_message = None;
                }
            }
            let _ = should_quit;
            let _ = save_on_quit;
        }
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn byte_to_char(s: &str, bi: usize) -> usize {
    s.char_indices().take_while(|(b, _)| *b <= bi).count()
}
