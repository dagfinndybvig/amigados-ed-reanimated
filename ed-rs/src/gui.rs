//! Native window backend for Neo-Ed (eframe/egui).
//!
//! Opens a separate window sized to the original Ed screen (640x200,
//! the `RAW:0/0/639/199` window from Ed 1.14), renders the editor, and
//! closes the window when the editor quits. The testable state machine
//! lives in `screen.rs`; this module only maps egui input to it and
//! draws the result.

use crate::cmdline::Editor;
use crate::screen::{self, Decision, Mode};
use crossterm::event::{KeyCode, KeyModifiers};
use eframe::egui;

/// Text lines visible inside the window (between title and status).
const TEXT_ROWS: usize = 10;

/// Window title, like the original's "Ed 1.14" title bar.
const TITLE: &str = " Neo-Ed";

/// Original Ed screen size in pixels (RAW:0/0/639/199).
const SCREEN_W: f32 = 640.0;
const SCREEN_H: f32 = 200.0;

pub struct EdApp {
    pub ed: Editor,
    pub mode: Mode,
    pub message: Option<String>,
    pub last_command: String,
    pub rows: usize,
    pub quit: bool,
    pub save_on_quit: bool,
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

    fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            // A new key press clears the previous status message; it
            // persists until the next command (spec section 3).
            self.message = None;
            match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    if pressed {
                        if let Some(code) = map_key(key, modifiers) {
                            self.process(code, map_mods(modifiers));
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
                        self.process(KeyCode::Char(c), KeyModifiers::NONE);
                    }
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let rect = ui.max_rect();
        let font = egui::FontId::monospace(14.0);
        let galley = painter.layout_no_wrap("M".to_string(), font.clone(), egui::Color32::WHITE);
        let cell_w = galley.size().x;
        let cell_h = galley.size().y;
        let title_h = cell_h + 4.0;
        let status_h = cell_h + 4.0;
        let text_top = rect.min.y + title_h;

        // title bar (white on black, like an Amiga window title)
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), title_h)),
            0.0,
            egui::Color32::WHITE,
        );
        painter.text(
            rect.min + egui::vec2(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            TITLE,
            font.clone(),
            egui::Color32::BLACK,
        );

        let b = &self.ed.buffer;
        let top = b.top;
        let cols = (rect.width() / cell_w) as usize;

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
        let status: String = match &self.mode {
            Mode::Extended(s) => format!("*{}", s),
            Mode::QuitConfirm => screen::CONFIRM_PROMPT.to_string(),
            Mode::ExchangeConfirm => "Exchange? ".to_string(),
            Mode::ShDisplay => "Type any character to continue ".to_string(),
            Mode::Immediate => self
                .message
                .clone()
                .or_else(|| self.ed.last_message.clone())
                .unwrap_or_default(),
        };
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.max.y - status_h),
            egui::vec2(rect.width(), status_h),
        );
        painter.rect_filled(status_rect, 0.0, egui::Color32::WHITE);
        painter.text(
            status_rect.min + egui::vec2(2.0, 2.0),
            egui::Align2::LEFT_TOP,
            status,
            font.clone(),
            egui::Color32::BLACK,
        );

        // cursor (only in the text area, not in SH display)
        if !matches!(self.mode, Mode::ShDisplay) {
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
        self.handle_input(ui.ctx());
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
        }
    }
}

/// Open the editor in its own window. Returns when the window closes.
pub fn run(ed: Editor) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([SCREEN_W, SCREEN_H])
            .with_title("Neo-Ed"),
        ..Default::default()
    };
    eframe::run_native(
        "Neo-Ed",
        options,
        Box::new(|_cc| Ok(Box::new(EdApp::new(ed)))),
    )
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
