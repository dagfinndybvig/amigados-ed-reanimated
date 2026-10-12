//! Native window backend for Neo-Ed (eframe/egui).
//!
//! Opens a separate window: 640 pixels wide like the original Ed
//! screen (`RAW:0/0/639/199`), and tall enough for TEXT_ROWS text
//! rows (a deliberate readability deviation from the original's
//! 200-pixel ten-row window). Renders the editor and closes the
//! window when the editor quits. The testable state machine lives in
//! `screen.rs`; this module only maps egui input to it and draws
//! the result.

use crate::buffer::Buffer;
use crate::cmdline::Editor;
use crate::screen::{self, Decision, Mode};
use crossterm::event::{KeyCode, KeyModifiers};
use eframe::egui;

/// Text lines visible inside the window (between title and status).
const TEXT_ROWS: usize = 20;

/// Target row height in points, like the original's 8x16 font.
const ROW_H: f32 = 16.0;

/// Window width, like the original Ed screen (RAW:0/0/639/199).
const SCREEN_W: f32 = 640.0;

/// Window height: title bar + TEXT_ROWS rows + status line + padding.
const SCREEN_H: f32 = (TEXT_ROWS as f32 + 2.0) * ROW_H + 8.0;

/// Title-bar text: a key hint, drawn highlighted like the original's
/// "Ed 1.14" title bar.
const TITLE: &str = " ESC for control";

/// Mild blue background for the reverse-video fields (title bar and
/// status line); text on them stays black.
const REV_VIDEO_BG: egui::Color32 = egui::Color32::from_rgb(0x6F, 0xA8, 0xDC);

/// Prompt shown when the window close button is clicked with unsaved
/// edits. Y saves and closes; N closes without saving; any other key
/// cancels the close.
const CLOSE_PROMPT: &str = "Save changes before closing? (Y/N): ";

pub struct EdApp {
    pub ed: Editor,
    pub mode: Mode,
    pub message: Option<String>,
    pub last_command: String,
    pub rows: usize,
    pub quit: bool,
    pub save_on_quit: bool,
    /// True while the window close button has been clicked and we are
    /// waiting for the user to confirm (Y/N) before closing.
    pub close_requested: bool,
    /// Monospace font size, chosen so the layout fits the window.
    pub font_size: f32,
    /// Whether `font_size` has been computed yet.
    pub font_ready: bool,
}

impl EdApp {
    pub fn new(ed: Editor) -> Self {
        EdApp {
            ed,
            mode: Mode::Immediate,
            message: None,
            last_command: String::new(),
            rows: TEXT_ROWS,
            quit: false,
            save_on_quit: false,
            close_requested: false,
            font_size: 14.0,
            font_ready: false,
        }
    }

    fn process(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let (new_mode, decision, msg) = screen::process_key(
            &mut self.ed,
            self.mode.clone(),
            code,
            modifiers,
            self.rows,
            &mut self.last_command,
        );
        self.mode = new_mode;
        self.message = msg;
        if let Decision::Quit { save } = decision {
            self.quit = true;
            self.save_on_quit = save;
        }
    }

    /// Handle a key press while the close confirmation prompt is shown.
    fn handle_close_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.quit = true;
                self.save_on_quit = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.quit = true;
                self.save_on_quit = false;
            }
            _ => {
                // cancel the close and return to editing
                self.close_requested = false;
            }
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    if pressed {
                        // A new key press clears the previous status
                        // message; it persists until the next command
                        // (spec section 3).
                        self.message = None;
                        self.ed.last_message = None;
                        if let Some(code) = map_key(key, modifiers) {
                            if self.close_requested {
                                self.handle_close_key(code);
                            } else {
                                self.process(code, map_mods(modifiers));
                            }
                        }
                    }
                }
                egui::Event::Text(text) => {
                    for c in text.chars() {
                        // Control characters (e.g. Ctrl-C) arrive as Key
                        // events, not text; never insert them.
                        if c.is_control() {
                            continue;
                        }
                        self.message = None;
                        self.ed.last_message = None;
                        if self.close_requested {
                            self.handle_close_key(KeyCode::Char(c));
                        } else {
                            self.process(KeyCode::Char(c), KeyModifiers::NONE);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let font = egui::FontId::monospace(self.font_size);
        // Use the font's true row height (baseline-to-baseline) for line
        // spacing, not the glyph height, so rows are not cramped.
        let cell_h = ui.ctx().fonts_mut(|f| f.row_height(&font));
        let blink_on = (ui.ctx().input(|i| i.time) * 2.0) as i64 % 2 == 0;
        let painter = ui.painter();
        let rect = ui.max_rect();
        let galley = painter.layout_no_wrap("M".to_string(), font.clone(), egui::Color32::WHITE);
        let cell_w = galley.size().x;
        let title_h = cell_h + 4.0;
        let status_h = cell_h + 4.0;
        let text_top = rect.min.y + title_h;

        // title bar (mild blue on black, like an Amiga window title)
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), title_h)),
            0.0,
            REV_VIDEO_BG,
        );
        painter.text(
            rect.min + egui::vec2(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            TITLE,
            font.clone(),
            egui::Color32::BLACK,
        );

        let cols = (rect.width() / cell_w) as usize;
        // Scroll the view so the cursor stays visible: vertically one
        // line at a time, horizontally ten characters at a time (spec
        // section 3).
        scroll_follow(&mut self.ed.buffer, self.rows, cols);
        let b = &self.ed.buffer;
        let top = b.top;

        if matches!(self.mode, Mode::ShDisplay) {
            // SH display replaces the screen (spec section 7)
            for (row, line) in self.ed.sh_text().lines().enumerate() {
                if row >= self.rows {
                    break;
                }
                let y = text_top + row as f32 * cell_h;
                painter.text(
                    egui::pos2(rect.min.x, y),
                    egui::Align2::LEFT_TOP,
                    line,
                    font.clone(),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // text lines
            for (row, i) in (top..b.line_count()).enumerate() {
                if row >= self.rows {
                    break;
                }
                let line = &b.lines[i];
                let visible: String = line.chars().skip(b.left).take(cols).collect();
                let y = text_top + row as f32 * cell_h;
                painter.text(
                    egui::pos2(rect.min.x, y),
                    egui::Align2::LEFT_TOP,
                    visible,
                    font.clone(),
                    egui::Color32::WHITE,
                );
            }
        }

        // status line (bottom of the window, reverse video)
        let status: String = if self.close_requested {
            CLOSE_PROMPT.to_string()
        } else {
            match &self.mode {
                Mode::Extended(s) => format!("*{}", s),
                Mode::QuitConfirm => screen::CONFIRM_PROMPT.to_string(),
                Mode::ExchangeConfirm => "Exchange? ".to_string(),
                Mode::ShDisplay => "Type any character to continue ".to_string(),
                Mode::Immediate => self
                    .message
                    .clone()
                    .or_else(|| self.ed.last_message.clone())
                    .unwrap_or_default(),
            }
        };
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.max.y - status_h),
            egui::vec2(rect.width(), status_h),
        );
        painter.rect_filled(status_rect, 0.0, REV_VIDEO_BG);
        painter.text(
            status_rect.min + egui::vec2(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            status,
            font.clone(),
            egui::Color32::BLACK,
        );

        // cursor (only in the text area, not in SH display; blinks)
        if !matches!(self.mode, Mode::ShDisplay) && blink_on {
            let cursor_x = rect.min.x + b.cursor.col.saturating_sub(b.left) as f32 * cell_w;
            let cursor_y = text_top + b.cursor.line.saturating_sub(top) as f32 * cell_h;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(cursor_x, cursor_y),
                    egui::vec2(cell_w, cell_h),
                ),
                0.0,
                egui::Color32::from_rgb(0, 128, 255),
            );
        }
    }
}

impl eframe::App for EdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Pick a font size that fits the window on the first frame.
        if !self.font_ready {
            self.font_size = fit_font_size(ui.ctx(), SCREEN_H);
            self.font_ready = true;
        }

        self.handle_input(ui.ctx());

        // Window close button: if there are unsaved edits, prompt before
        // closing; otherwise close immediately.
        let close_requested = ui.ctx().input(|i| i.viewport().close_requested());
        if close_requested && !self.quit && !self.close_requested {
            if self.ed.dirty {
                self.close_requested = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else {
                self.quit = true;
                self.save_on_quit = false;
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| {
                self.render(ui);
            });

        if self.quit {
            if self.save_on_quit {
                screen::save_files(&self.ed);
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            // Keep the cursor blinking.
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

/// Open the editor in its own window. Returns when the window closes.
pub fn run(ed: Editor) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([SCREEN_W, SCREEN_H])
            .with_resizable(false)
            .with_title("Neo-Ed"),
        ..Default::default()
    };
    eframe::run_native(
        "Neo-Ed",
        options,
        Box::new(|_cc| Ok(Box::new(EdApp::new(ed)))),
    )
}

/// Largest monospace font size (from 14.0 down) whose layout — title bar,
/// `TEXT_ROWS` text lines, and status line — fits within `height`.
fn fit_font_size(ctx: &egui::Context, height: f32) -> f32 {
    let mut size = 14.0;
    while size > 8.0 {
        let font = egui::FontId::monospace(size);
        let row_h = ctx.fonts_mut(|f| f.row_height(&font));
        if (TEXT_ROWS as f32 + 2.0) * row_h + 8.0 <= height {
            return size;
        }
        size -= 0.5;
    }
    size
}

/// Scroll the view so the cursor stays visible: vertically one line
/// at a time, horizontally ten characters at a time (spec section 3).
/// Ported from the crossterm TUI's `Screen::scroll_follow`.
fn scroll_follow(b: &mut Buffer, rows: usize, cols: usize) {
    if b.cursor.line < b.top {
        b.top = b.cursor.line;
    } else if b.cursor.line >= b.top + rows {
        b.top = b.cursor.line + 1 - rows;
    }
    if b.cursor.col < b.left {
        b.left = b.cursor.col / 10 * 10;
    } else if b.cursor.col >= b.left + cols {
        b.left = (b.cursor.col + 1 - cols) / 10 * 10 + 10;
    }
}

/// Map an egui key to a crossterm key code. Letter keys are only mapped
/// when a control/alt modifier is held (e.g. Ctrl-C); plain letters are
/// delivered through `Event::Text` instead, so they are not mapped here
/// to avoid double-processing.
fn map_key(key: egui::Key, modifiers: egui::Modifiers) -> Option<KeyCode> {
    use egui::Key;
    let code = match key {
        Key::Enter => KeyCode::Enter,
        Key::Backspace => KeyCode::Backspace,
        Key::Delete => KeyCode::Delete,
        Key::ArrowUp => KeyCode::Up,
        Key::ArrowDown => KeyCode::Down,
        Key::ArrowLeft => KeyCode::Left,
        Key::ArrowRight => KeyCode::Right,
        Key::Tab => KeyCode::Tab,
        Key::Escape => KeyCode::Esc,
        Key::Home => KeyCode::Home,
        Key::End => KeyCode::End,
        _ => {
            if modifiers.ctrl || modifiers.alt {
                if let Some(c) = letter_key_to_char(key) {
                    KeyCode::Char(c)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    };
    Some(code)
}

fn letter_key_to_char(key: egui::Key) -> Option<char> {
    use egui::Key::*;
    Some(match key {
        A => 'a',
        B => 'b',
        C => 'c',
        D => 'd',
        E => 'e',
        F => 'f',
        G => 'g',
        H => 'h',
        I => 'i',
        J => 'j',
        K => 'k',
        L => 'l',
        M => 'm',
        N => 'n',
        O => 'o',
        P => 'p',
        Q => 'q',
        R => 'r',
        S => 's',
        T => 't',
        U => 'u',
        V => 'v',
        W => 'w',
        X => 'x',
        Y => 'y',
        Z => 'z',
        _ => return None,
    })
}

fn map_mods(m: egui::Modifiers) -> KeyModifiers {
    let mut km = KeyModifiers::NONE;
    if m.ctrl {
        km |= KeyModifiers::CONTROL;
    }
    if m.shift {
        km |= KeyModifiers::SHIFT;
    }
    if m.alt {
        km |= KeyModifiers::ALT;
    }
    km
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf() -> Buffer {
        Buffer::from_text(&"x\n".repeat(30))
    }

    #[test]
    fn vertical_scroll_puts_cursor_on_last_visible_row() {
        let mut b = buf();
        b.cursor.line = 14; // past the 10-row window
        scroll_follow(&mut b, 10, 80);
        assert_eq!(b.top, 5);
        assert_eq!(b.cursor.line - b.top, 9);
    }

    #[test]
    fn vertical_scroll_follows_cursor_up() {
        let mut b = buf();
        b.top = 5;
        b.cursor.line = 2;
        scroll_follow(&mut b, 10, 80);
        assert_eq!(b.top, 2);
    }

    #[test]
    fn no_scroll_when_cursor_inside_window() {
        let mut b = buf();
        b.top = 5;
        b.cursor.line = 7;
        scroll_follow(&mut b, 10, 80);
        assert_eq!(b.top, 5);
    }

    #[test]
    fn horizontal_scroll_moves_ten_chars_at_a_time() {
        let mut b = buf();
        b.cursor.col = 95; // past the 80-col window
        scroll_follow(&mut b, 10, 80);
        assert_eq!(b.left, 20); // cursor at screen col 75
        b.cursor.col = 15; // back left of the window
        scroll_follow(&mut b, 10, 80);
        assert_eq!(b.left, 10); // snapped to a 10-char boundary
    }
}
