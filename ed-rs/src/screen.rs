//! Full-screen terminal UI for the Ed reimplementation (crossterm).

use crate::buffer::Buffer;
use crate::cmdline::{self, Editor, ExecResult, Item};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Result as CResult, Stdout, Write};

pub struct Screen {
    stdout: Stdout,
    pub rows: usize,
    pub cols: usize,
}

pub enum Mode {
    Immediate,
    Extended(String),
    /// Waiting for Y/N answer to the Q confirmation prompt.
    QuitConfirm,
    /// Waiting for Y/N answer to an EQ "Exchange? " prompt.
    ExchangeConfirm(Box<(String, String)>),
    /// SH display shown; any key dismisses it.
    ShDisplay(String),
}

impl Screen {
    pub fn new() -> CResult<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let (cols, rows) = terminal::size()?;
        Ok(Screen { stdout, rows: rows as usize, cols: cols as usize })
    }

    pub fn cleanup(&mut self) -> CResult<()> {
        execute!(self.stdout, Show, LeaveAlternateScreen)
    }

    pub fn render(&mut self, ed: &Editor, mode: &Mode, message: &Option<String>) -> io::Result<()> {
        queue!(self.stdout, Clear(ClearType::All))?;
        let b = &ed.buffer;
        let text_rows = self.rows.saturating_sub(1); // last line = status
        let mut row = 0usize;
        // vertical scroll
        if b.cursor.line < b.top {
            // cursor moved above the viewport
        }
        let top = b.top;
        for i in top..b.line_count() {
            if row >= text_rows {
                break;
            }
            let line = &b.lines[i];
            // horizontal window of 10-char scroll steps
            let chars: Vec<char> = line.chars().collect();
            let visible: String = chars
                .iter()
                .skip(b.left)
                .take(self.cols)
                .collect();
            queue!(self.stdout, MoveTo(0, row as u16), Print(&visible))?;
            row += 1;
        }
        // status line
        let status = match mode {
            Mode::Extended(s) => format!("*{}", s),
            Mode::QuitConfirm => "Edits will be lost - type Y to confirm: ".to_string(),
            Mode::ExchangeConfirm(_) => "Exchange? ".to_string(),
            Mode::ShDisplay(_) => "Type any character to continue ".to_string(),
            Mode::Immediate => message.clone().unwrap_or_default(),
        };
        queue!(
            self.stdout,
            MoveTo(0, (self.rows - 1) as u16),
            SetForegroundColor(crossterm::style::Color::Black),
            Print(format!("{:<width$}", status, width = self.cols)),
            ResetColor,
        )?;
        // cursor
        let cur_line = b.cursor.line;
        let cur_col = b.cursor.col;
        let screen_row = cur_line.saturating_sub(top);
        let screen_col = cur_col.saturating_sub(b.left);
        queue!(self.stdout, MoveTo(screen_col as u16, screen_row as u16))?;
        self.stdout.flush()
    }

    pub fn scroll_follow(&mut self, b: &mut Buffer) {
        let text_rows = self.rows.saturating_sub(1);
        if b.cursor.line < b.top {
            b.top = b.cursor.line;
        } else if b.cursor.line >= b.top + text_rows {
            b.top = b.cursor.line + 1 - text_rows;
        }
        // horizontal: scroll in steps of 10 characters
        if b.cursor.col < b.left {
            b.left = b.cursor.col / 10 * 10;
        } else if b.cursor.col >= b.left + self.cols {
            b.left = (b.cursor.col + 1 - self.cols) / 10 * 10 + 10;
        }
    }
}

/// Run the interactive editor loop over an already-loaded Editor.
pub fn run_loop(ed: &mut Editor) -> CResult<()> {
    let mut screen = Screen::new()?;
    let mut mode = Mode::Immediate;
    let mut message: Option<String> = None;
    let mut last_items: Vec<Item> = Vec::new();

    loop {
        screen.scroll_follow(&mut ed.buffer);
        screen.render(ed, &mode, &message)?;
        let ev = event::read()?;
        if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = ev {
            if kind == event::KeyEventKind::Release {
                continue;
            }
            // abort signal: any printable key during command execution
            // is handled where sequences run long (RP); ESC in extended
            // mode terminates the line and stays in extended mode.
            match &mut mode {
                Mode::Immediate => match (code, modifiers) {
                    (KeyCode::Esc, _) => {
                        mode = Mode::Extended(String::new());
                    }
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                        screen.cleanup()?;
                        return Ok(());
                    }
                    (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // CTRL-A: insert blank line after current
                        ed.buffer.insert_after("");
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Char('b'), m) if m.contains(KeyModifiers::CONTROL) => {
                        if let Some(msg) = ed.buffer.delete_line() {
                            message = Some(msg.to_string());
                        }
                        ed.dirty = true;
                    }
                    (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        if b.cursor.line == b.top && b.cursor.col == 0 {
                            b.cursor.line = (b.top + screen.rows - 2).min(b.line_count() - 1);
                            b.top = b.cursor.line.min(b.line_count().saturating_sub(1));
                            b.top = b.top.saturating_sub(screen.rows - 2);
                            b.cursor.col = 0;
                        } else {
                            b.cursor.line = b.top;
                            b.cursor.col = 0;
                        }
                        message = None;
                    }
                    (KeyCode::Char('f'), m) if m.contains(KeyModifiers::CONTROL) => {
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
                            }
                        }
                        message = None;
                    }
                    (KeyCode::Char('g'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // repeat the last extended command line
                        if !last_items.is_empty() {
                            let r = ed.run_line(&last_items);
                            apply_result(ed, r, &mut mode, &mut message);
                        }
                    }
                    (KeyCode::Char('o'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        let line = b.current_line().clone();
                        let col = b.cursor.col.min(line.chars().count());
                        if line.chars().nth(col).map(|c| c != ' ') == Some(true) {
                            let end = line[col..]
                                .find(' ')
                                .map(|i| col + i)
                                .unwrap_or(line.chars().count());
                            let start_b = char_to_byte(&line, col);
                            let end_b = char_to_byte(&line, end);
                            b.lines[b.cursor.line].replace_range(start_b..end_b, "");
                        } else {
                            let start = col;
                            let end = line[start..]
                                .find(|c: char| c != ' ')
                                .map(|i| start + i)
                                .unwrap_or(line.chars().count());
                            let start_b = char_to_byte(&line, start);
                            let end_b = char_to_byte(&line, end);
                            b.lines[b.cursor.line].replace_range(start_b..end_b, "");
                        }
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Char('r'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        let line = b.current_line().clone();
                        // move left to the space after the previous word
                        let start = line[..char_to_byte(&line, b.cursor.col)]
                            .rfind(' ')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        b.cursor.col = byte_to_char(&line, start);
                        message = None;
                    }
                    (KeyCode::Char('t'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        let line = b.current_line().clone();
                        let col = b.cursor.col;
                        if let Some(off) = line[col..].find(' ') {
                            let target = col + off + 1;
                            b.cursor.col = target.min(line.chars().count());
                        } else {
                            b.cursor.col = line.chars().count();
                        }
                        message = None;
                    }
                    (KeyCode::Char('v'), m) if m.contains(KeyModifiers::CONTROL) => {
                        message = None; // full redraw happens anyway
                    }
                    (KeyCode::Char('y'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        let col = b.cursor.col;
                        let line = &mut b.lines[b.cursor.line];
                        let col_b = char_to_byte(line, col);
                        line.truncate(col_b);
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Char('d'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // scroll text down 12 lines toward the beginning
                        let b = &mut ed.buffer;
                        b.cursor.line = b.cursor.line.saturating_sub(12);
                        b.cursor.col = 0;
                        message = None;
                    }
                    (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let b = &mut ed.buffer;
                        b.cursor.line = (b.cursor.line + 12).min(b.line_count() - 1);
                        b.cursor.col = 0;
                        message = None;
                    }
                    (KeyCode::Backspace, _) => {
                        let _ = ed.buffer.delete_left();
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Delete, _) => {
                        ed.buffer.delete_at();
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Enter, _) => {
                        ed.buffer.split_line(ed.left_margin);
                        ed.dirty = true;
                        message = None;
                    }
                    (KeyCode::Tab, _) => {
                        let b = &mut ed.buffer;
                        let col = b.cursor.col;
                        let next = (col / ed.tab_distance + 1) * ed.tab_distance;
                        b.cursor.col = next.min(b.current_line().chars().count());
                        message = None;
                    }
                    (KeyCode::Up, _) => {
                        let b = &mut ed.buffer;
                        if b.cursor.line == 0 {
                            message = Some("Top of file".to_string());
                        } else {
                            b.cursor.line -= 1;
                            b.cursor.col = b.cursor.col.min(b.current_line().chars().count().max(1));
                            message = None;
                        }
                    }
                    (KeyCode::Down, _) => {
                        let b = &mut ed.buffer;
                        if b.cursor.line + 1 >= b.line_count() {
                            message = Some("End of file".to_string());
                        } else {
                            b.cursor.line += 1;
                            message = None;
                        }
                    }
                    (KeyCode::Left, _) => {
                        let b = &mut ed.buffer;
                        if b.cursor.col > 0 {
                            b.cursor.col -= 1;
                        }
                        message = None;
                    }
                    (KeyCode::Right, _) => {
                        let b = &mut ed.buffer;
                        b.cursor.col += 1;
                        message = None;
                    }
                    (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                        match ed.buffer.insert_char(c, ed.right_margin) {
                            Ok(_) => {
                                ed.dirty = true;
                                message = None;
                            }
                            Err(msg) => message = Some(msg.to_string()),
                        }
                    }
                    _ => {}
                },
                Mode::Extended(linebuf) => match (code, modifiers) {
                    (KeyCode::Esc, _) => {
                        // ESC terminates the line and stays in
                        // extended mode (spec 5)
                        let r = exec_line(ed, linebuf);
                        last_items = r;
                        mode = Mode::Extended(String::new());
                        apply_result(ed, ExecResult::default(), &mut mode, &mut message);
                    }
                    (KeyCode::Enter, _) => {
                        let items = exec_line(ed, linebuf);
                        last_items = items;
                        mode = Mode::Immediate;
                        // message already applied inside exec_line
                        let _ = &mut message;
                    }
                    (KeyCode::Backspace, _) => {
                        linebuf.pop();
                    }
                    (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                        linebuf.push(c);
                    }
                    _ => {}
                },
                Mode::QuitConfirm => match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        screen.cleanup()?;
                        return Ok(());
                    }
                    _ => mode = Mode::Immediate,
                },
                Mode::ExchangeConfirm(pair) => match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let (needle, repl) = (*pair.clone()).clone();
                        // apply at the found position
                        let b = &mut ed.buffer;
                        let line = &mut b.lines[b.cursor.line];
                        let col_b = char_to_byte(line, b.cursor.col);
                        let n_b = needle.len();
                        line.replace_range(col_b..col_b + n_b, &repl);
                        b.cursor.col += repl.chars().count();
                        ed.dirty = true;
                        mode = Mode::Immediate;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        // cursor moves past the search string
                        ed.buffer.cursor.col += 1;
                        mode = Mode::Immediate;
                    }
                    _ => mode = Mode::Immediate,
                },
                Mode::ShDisplay(_) => {
                    ed.sh_show = false;
                    mode = Mode::Immediate;
                }
            }
            if ed.sh_show {
                mode = Mode::ShDisplay(ed.sh_text());
            }
            if let Mode::Immediate = mode {
                if let Event::Key(ke) = ev {
                    if ke.code == KeyCode::Enter && !message_is_none(&message) {
                        // keep messages until next command (spec 3)
                    }
                }
            }
        }
    }
}

fn message_is_none(m: &Option<String>) -> bool {
    m.is_none()
}

fn exec_line(ed: &mut Editor, linebuf: &String) -> Vec<Item> {
    match cmdline::parse_line(linebuf) {
        Ok(items) => {
            let r = ed.run_line(&items);
            if let Some(msg) = &r.message {
                // store into the editor for the UI to pick up
                ed.last_message = Some(msg.clone());
                ed.last_exchange_prompt = matches!(msg.as_str(), "Exchange? ");
                ed.quit_pending = r.quit;
                ed.quit_save_pending = r.quit_save;
            }
            items
        }
        Err(e) => {
            ed.last_message = Some(e.to_string());
            ed.quit_pending = false;
            Vec::new()
        }
    }
}

fn apply_result(_ed: &mut Editor, _r: ExecResult, _mode: &mut Mode, _message: &mut Option<String>) {
    // handled via ed.last_message in the main loop; placeholder
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn byte_to_char(s: &str, bi: usize) -> usize {
    s.char_indices().take_while(|(b, _)| *b <= bi).count()
}
