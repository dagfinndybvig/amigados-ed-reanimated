//! Buffer model for the Ed reimplementation.
//!
//! Faithful to the behavior spec (docs/ed-spec.md):
//! - line-based buffer, maximum 255 characters per line
//! - cursor is (line, col); the buffer always has at least one line
//! - block marks are whole lines and are invalidated by any edit except
//!   `IB` (insert block), which keeps them defined
//! - word wrap at the right margin (default 77) while typing at the end
//!   of a line

pub const MAX_LINE: usize = 255;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cursor {
    pub line: usize, // 0-based
    pub col: usize,  // 0-based
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlockMark {
    pub line: usize,
}

#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    /// First line (0-based) of the block, if any.
    pub block_start: Option<usize>,
    /// Last line (0-based) of the block, if any.
    pub block_end: Option<usize>,
    /// Top line of the viewport (0-based).
    pub top: usize,
    /// Horizontal scroll offset in characters.
    pub left: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            lines: vec![String::new()],
            cursor: Cursor { line: 0, col: 0 },
            block_start: None,
            block_end: None,
            top: 0,
            left: 0,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let mut b = Buffer::new();
        b.lines = text.split('\n').map(|l| l.to_string()).collect();
        if b.lines.is_empty() {
            b.lines.push(String::new());
        }
        b
    }

    pub fn to_text(&self) -> String {
        // Writing always terminates the file with a linefeed, even if
        // the original did not end with one (spec section 3).
        let mut s = self.lines.join("\n");
        s.push('\n');
        s
    }

    pub fn current_line(&self) -> &String {
        &self.lines[self.cursor.line]
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Any edit to the text invalidates the block marks (except IB).
    pub fn invalidate_block(&mut self) {
        self.block_start = None;
        self.block_end = None;
    }

    pub fn block_lines(&self) -> Option<(usize, usize)> {
        match (self.block_start, self.block_end) {
            (Some(s), Some(e)) => {
                if s > e {
                    None
                } else {
                    Some((s, e))
                }
            }
            _ => None,
        }
    }

    /// Insert a character at the cursor, refusing lines longer than 255.
    /// Returns Ok(wrapped) where wrapped=true if word wrap moved the
    /// trailing word onto a new line.
    pub fn insert_char(&mut self, c: char, right_margin: usize) -> Result<bool, &'static str> {
        let line = &mut self.lines[self.cursor.line];
        if line.chars().count() >= MAX_LINE {
            return Err("Line too long");
        }
        let col = self.cursor.col.min(line.chars().count());
        line.insert(col, c);
        self.cursor.col = col + 1;
        self.invalidate_block();
        // Word wrap: only while typing at the end of the line at the
        // right margin (spec section 3).
        if self.cursor.col > right_margin {
            let line = self.lines[self.cursor.line].clone();
            if let Some(pos) = line.rfind(' ') {
                if pos > 0 {
                    let (head, tail) = line.split_at(pos);
                    let tail = tail.trim_start();
                    self.lines[self.cursor.line] = head.trim_end().to_string();
                    self.lines.insert(self.cursor.line + 1, tail.to_string());
                    self.cursor.line += 1;
                    self.cursor.col = tail.chars().count();
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Delete the character left of the cursor (Backspace / CTRL-H).
    pub fn delete_left(&mut self) {
        let line = &mut self.lines[self.cursor.line];
        if self.cursor.col > 0 {
            let col = self.cursor.col.min(line.chars().count());
            line.remove(col - 1);
            self.cursor.col = col - 1;
            self.invalidate_block();
        } else if self.cursor.line > 0 {
            // join to previous line
            let cur = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            let prev_len = self.lines[self.cursor.line].chars().count();
            self.lines[self.cursor.line].push_str(&cur);
            self.cursor.col = prev_len;
            self.invalidate_block();
        }
    }

    /// Delete the character at the cursor (DEL).
    pub fn delete_at(&mut self) {
        let line = &mut self.lines[self.cursor.line];
        if self.cursor.col < line.chars().count() {
            line.remove(self.cursor.col);
            self.invalidate_block();
        }
    }

    /// Split the current line at the cursor (RETURN / S command).
    pub fn split_line(&mut self, left_margin: usize) {
        let line = &mut self.lines[self.cursor.line];
        let col = self.cursor.col.min(line.chars().count());
        let tail: String = line.chars().skip(col).collect();
        line.truncate(line.chars().take(col).count());
        let cursor_line = self.cursor.line;
        self.lines.insert(cursor_line + 1, tail);
        self.cursor = Cursor {
            line: cursor_line + 1,
            col: left_margin.saturating_sub(1).max(0),
        };
        self.invalidate_block();
    }

    /// Join the current line with the next (J command).
    pub fn join_line(&mut self) {
        if self.cursor.line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor.line + 1);
            self.lines[self.cursor.line].push_str(&next);
            self.invalidate_block();
        }
    }

    /// Delete the current line (D / CTRL-B). Returns a status message
    /// only when the buffer's last line was deleted.
    pub fn delete_line(&mut self) -> Option<&'static str> {
        let was_last = self.cursor.line + 1 == self.lines.len();
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor.col = 0;
            self.invalidate_block();
            return if was_last { Some("Last line deleted") } else { None };
        }
        self.lines.remove(self.cursor.line);
        if self.cursor.line >= self.lines.len() {
            self.cursor.line = self.lines.len() - 1;
        }
        self.cursor.col = 0;
        self.invalidate_block();
        if was_last {
            Some("Last line deleted")
        } else {
            None
        }
    }

    /// Insert a line with the given text after the current line (A /s/).
    pub fn insert_after(&mut self, text: &str) {
        let at = self.cursor.line + 1;
        self.lines.insert(at, text.to_string());
        self.invalidate_block();
    }

    /// Insert a line with the given text before the current line (I /s/).
    pub fn insert_before(&mut self, text: &str) {
        self.lines.insert(self.cursor.line, text.to_string());
        self.cursor.line += 1;
        self.invalidate_block();
    }

    /// Delete a marked block (DB). Returns an error message or Ok(()).
    pub fn delete_block(&mut self) -> Result<(), &'static str> {
        let (s, e) = self
            .block_lines()
            .ok_or("No block marked")?;
        if self.cursor.line >= s && self.cursor.line <= e {
            return Err("Cursor inside block");
        }
        for _ in s..=e {
            self.lines.remove(s);
        }
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        if self.cursor.line > e {
            self.cursor.line -= e - s + 1;
        } else if self.cursor.line > self.lines.len() - 1 {
            self.cursor.line = self.lines.len() - 1;
        }
        self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].chars().count());
        self.invalidate_block();
        Ok(())
    }

    /// Insert a copy of the marked block after the current line (IB).
    /// Block stays defined.
    pub fn insert_block(&mut self) -> Result<(), &'static str> {
        let (s, e) = self
            .block_lines()
            .ok_or("No block marked")?;
        let copy: Vec<String> = self.lines[s..=e].to_vec();
        let at = self.cursor.line + 1;
        for (i, l) in copy.into_iter().enumerate() {
            self.lines.insert(at + i, l);
        }
        Ok(())
    }
}
