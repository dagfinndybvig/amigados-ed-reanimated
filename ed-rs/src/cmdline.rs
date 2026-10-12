//! Extended command line parser and executor for the Ed reimplementation.
//!
//! Faithful to the behavior spec (docs/ed-spec.md section 5):
//! - commands are one or two letters, case-insensitive
//! - multiple commands separated by `;`
//! - a number before a command is a repeat count
//! - command groups nest in parentheses
//! - strings are delimited by any character that is not a letter,
//!   digit, space, semicolon, or bracket
//! - RP repeats until error; any character typed during execution
//!   aborts the sequence ("Commands abandoned")

use crate::buffer::Buffer;
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    None,
    Str(String),
    Two(String, String),
    Num(i64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub name: String,
    pub arg: Arg,
    pub repeat: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Cmd(Command),
    Group(Vec<Item>),
}

/// Parse one extended command line into a list of items.
pub fn parse_line(input: &str) -> Result<Vec<Item>, &'static str> {
    let mut chars = input.chars().peekable();
    let mut items = Vec::new();
    parse_items(&mut chars, &mut items, false)?;
    Ok(items)
}

fn parse_items(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    items: &mut Vec<Item>,
    in_group: bool,
) -> Result<(), &'static str> {
    loop {
        skip_spaces(chars);
        match chars.peek() {
            None => {
                if in_group {
                    return Err("Unmatched ()");
                }
                return Ok(());
            }
            Some(';') => {
                chars.next();
                continue;
            }
            Some(')') => {
                if in_group {
                    chars.next();
                    return Ok(());
                }
                return Err("Unmatched ()");
            }
            Some('(') => {
                chars.next();
                let mut group = Vec::new();
                parse_items(chars, &mut group, true)?;
                items.push(Item::Group(group));
            }
            Some(c) if c.is_ascii_digit() => {
                let num = parse_number(chars)?;
                skip_spaces(chars);
                let cmd = parse_command(chars)?;
                items.push(Item::Cmd(Command {
                    name: cmd.0,
                    arg: cmd.1,
                    repeat: num,
                }));
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let cmd = parse_command(chars)?;
                items.push(Item::Cmd(Command {
                    name: cmd.0,
                    arg: cmd.1,
                    repeat: 1,
                }));
            }
            Some(_) => {
                return Err("Syntax error");
            }
        }
    }
}

fn skip_spaces(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while matches!(chars.peek(), Some(' ')) {
        chars.next();
    }
}

fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<i64, &'static str> {
    let mut n: i64 = 0;
    while let Some(c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            n = n * 10 + d as i64;
            chars.next();
        } else {
            break;
        }
    }
    if n == 0 {
        return Err("Number expected");
    }
    Ok(n)
}

type CmdParse = (String, Arg);

fn parse_command(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<CmdParse, &'static str> {
    let mut name = String::new();
    // one or two letters
    if let Some(c) = chars.next() {
        if !c.is_ascii_alphabetic() {
            return Err("Syntax error");
        }
        name.push(c.to_ascii_uppercase());
    }
    if let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            name.push(c.to_ascii_uppercase());
            chars.next();
        }
    }
    skip_spaces(chars);
    // arguments per command family
    let arg = match name.as_str() {
        "A" | "I" | "F" | "BF" | "IF" | "WB" => match parse_string(chars) {
            Ok(a) => a,
            Err(e) => return Err(e),
        },
        "E" | "EQ" => match parse_two_strings(chars) {
            Ok(a) => a,
            Err(e) => return Err(e),
        },
        "M" | "SL" | "SR" | "ST" => match parse_number(chars) {
            Ok(n) => Arg::Num(n),
            Err(_) => return Err("Number expected"),
        },
        _ => Arg::None,
    };
    Ok((name, arg))
}

type StrResult = Result<Arg, &'static str>;

fn parse_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> StrResult {
    skip_spaces(chars);
    let delim = match chars.next() {
        Some(c) if !is_delim_forbidden(c) => c,
        Some(_) => return Err("Syntax error"),
        None => return Err("Syntax error"),
    };
    let mut s = String::new();
    for c in chars.by_ref() {
        if c == delim {
            return Ok(Arg::Str(s));
        }
        s.push(c);
    }
    Err("Syntax error")
}

fn parse_two_strings(chars: &mut std::iter::Peekable<std::str::Chars>) -> StrResult {
    skip_spaces(chars);
    let delim = match chars.next() {
        Some(c) if !is_delim_forbidden(c) => c,
        Some(_) => return Err("Syntax error"),
        None => return Err("Syntax error"),
    };
    let mut s1 = String::new();
    for c in chars.by_ref() {
        if c == delim {
            let mut s2 = String::new();
            for c in chars.by_ref() {
                if c == delim {
                    return Ok(Arg::Two(s1, s2));
                }
                s2.push(c);
            }
            // second string empty (delete) at end of line is legal
            return Ok(Arg::Two(s1, s2));
        }
        s1.push(c);
    }
    Err("Syntax error")
}

fn is_delim_forbidden(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ' ' || c == ';' || c == '(' || c == ')'
}

/// Result of executing a command: optional status-line message and an
/// error flag (an error aborts the command sequence, ending RP loops).
#[derive(Default)]
pub struct ExecResult {
    pub message: Option<String>,
    pub error: bool,
    pub quit: bool,
    pub quit_save: bool,
}

pub struct Editor {
    pub buffer: Buffer,
    pub from_file: String,
    pub dirty: bool,
    pub tab_distance: usize,
    pub left_margin: usize,
    pub right_margin: usize,
    pub case_insensitive: bool,
    pub abort_requested: bool,
    pub sh_show: bool,
    /// Set by E/EQ with query when a match was found and is awaiting
    /// confirmation in the UI layer.
    pub pending_exchange: Option<(String, String)>,
    /// Message for the UI status line.
    pub last_message: Option<String>,
    /// True when the status line currently shows the Exchange? prompt.
    pub last_exchange_prompt: bool,
    pub quit_pending: bool,
    pub quit_save_pending: bool,
    pub quiet_create: bool,
}

impl Editor {
    pub fn new(from_file: &str, text: Option<String>) -> Self {
        let (buffer, existed) = match text {
            Some(t) => (Buffer::from_text(&t), true),
            None => (Buffer::new(), false),
        };
        Editor {
            buffer,
            from_file: from_file.to_string(),
            dirty: !existed,
            tab_distance: 3,
            left_margin: 1,
            right_margin: 77,
            case_insensitive: false,
            abort_requested: false,
            sh_show: false,
            pending_exchange: None,
            last_message: None,
            last_exchange_prompt: false,
            quit_pending: false,
            quit_save_pending: false,
            quiet_create: false,
        }
    }

    /// Execute one parsed item (command or group), honoring repeat
    /// counts and RP. Returns execution result for this item.
    pub fn execute(&mut self, item: &Item, result: &mut ExecResult) {
        match item {
            Item::Group(items) => {
                for it in items {
                    if self.abort_requested || result.error {
                        break;
                    }
                    self.execute(it, result);
                }
            }
            Item::Cmd(cmd) => {
                for i in 0..cmd.repeat {
                    if self.abort_requested || result.error {
                        break;
                    }
                    if cmd.name == "RP" && i >= 1 {
                        // RP repeats the NEXT item; handled by caller
                        break;
                    }
                    let mut sub = ExecResult::default();
                    self.run_command(cmd, &mut sub);
                    if sub.error {
                        result.error = true;
                        if sub.message.is_some() {
                            result.message = sub.message;
                        }
                    } else if i == 0 {
                        result.message = sub.message.or(result.message.take());
                    }
                    if sub.quit {
                        result.quit = true;
                        result.quit_save = sub.quit_save;
                    }
                }
            }
        }
    }

    /// Execute a whole command line, handling RP (repeat until error).
    pub fn run_line(&mut self, items: &[Item]) -> ExecResult {
        let mut result = ExecResult::default();
        let mut i = 0;
        while i < items.len() {
            if self.abort_requested || result.error || result.quit {
                break;
            }
            let item = &items[i];
            // RP prefix: repeat the next item until error
            if let Item::Cmd(cmd) = item {
                if cmd.name == "RP" {
                    let target = items.get(i + 1);
                    match target {
                        Some(t) => {
                            loop {
                                if self.abort_requested {
                                    result.message =
                                        Some("Commands abandoned".to_string());
                                    break;
                                }
                                let mut sub = ExecResult::default();
                                self.execute(t, &mut sub);
                                if sub.error {
                                    break; // RP ends on error, silently
                                }
                                if sub.quit {
                                    result.quit = true;
                                    result.quit_save = sub.quit_save;
                                    break;
                                }
                            }
                            i += 2;
                            continue;
                        }
                        None => {
                            result.error = true;
                            result.message = Some("Syntax error".to_string());
                            break;
                        }
                    }
                }
            }
            let mut sub = ExecResult::default();
            self.execute(item, &mut sub);
            if sub.error {
                result.error = true;
            }
            if sub.message.is_some() {
                result.message = sub.message;
            }
            if sub.quit {
                result.quit = true;
                result.quit_save = sub.quit_save;
            }
            i += 1;
        }
        if self.abort_requested {
            result.message = Some("Commands abandoned".to_string());
            self.abort_requested = false;
        }
        result
    }

    fn run_command(&mut self, cmd: &Command, result: &mut ExecResult) {
        let b = &mut self.buffer;
        match (cmd.name.as_str(), &cmd.arg) {
            // --- movement ---
            ("T", Arg::None) => {
                b.cursor.line = 0;
                b.cursor.col = 0;
                b.top = 0;
            }
            ("B", Arg::None) => {
                b.cursor.line = b.line_count() - 1;
                b.cursor.col = 0;
            }
            ("N", Arg::None) => {
                if b.cursor.line + 1 >= b.line_count() {
                    result.error = true;
                    result.message = Some("End of file".to_string());
                } else {
                    b.cursor.line += 1;
                    b.cursor.col = 0;
                }
            }
            ("P", Arg::None) => {
                if b.cursor.line == 0 {
                    result.error = true;
                    result.message = Some("Top of file".to_string());
                } else {
                    b.cursor.line -= 1;
                    b.cursor.col = 0;
                }
            }
            ("M", Arg::Num(n)) => {
                let n = *n;
                if n < 1 || (n as usize) > b.line_count() {
                    result.error = true;
                    result.message = Some("Syntax error".to_string());
                } else {
                    b.cursor.line = (n - 1) as usize;
                    b.cursor.col = 0;
                }
            }
            ("CL", Arg::None) => {
                b.cursor.col = b.cursor.col.saturating_sub(1);
            }
            ("CR", Arg::None) => {
                b.cursor.col = (b.cursor.col + 1).min(b.current_line().chars().count().max(1));
            }
            ("CE", Arg::None) => {
                b.cursor.col = b.current_line().chars().count();
            }
            ("CS", Arg::None) => {
                b.cursor.col = 0;
            }
            // --- insertion ---
            ("A", Arg::Str(s)) => {
                b.insert_after(s);
                self.dirty = true;
            }
            ("I", Arg::Str(s)) => {
                b.insert_before(s);
                self.dirty = true;
            }
            ("S", Arg::None) => {
                b.split_line(self.left_margin);
                self.dirty = true;
            }
            ("J", Arg::None) => {
                b.join_line();
                self.dirty = true;
            }
            ("D", Arg::None) => {
                if let Some(msg) = b.delete_line() {
                    result.message = Some(msg.to_string());
                }
                self.dirty = true;
            }
            ("DC", Arg::None) => {
                b.delete_at();
                self.dirty = true;
            }
            // --- search and exchange ---
            ("F", Arg::Str(s)) => {
                if !self.find(s) {
                    result.error = true;
                    result.message = Some("search failed".to_string());
                }
            }
            ("BF", Arg::Str(s)) => {
                if !self.find_back(s) {
                    result.error = true;
                    result.message = Some("search failed".to_string());
                }
            }
            ("E", Arg::Two(s, t)) => {
                if !self.exchange(s, t, false) {
                    result.error = true;
                    result.message = Some("search failed".to_string());
                }
                self.dirty = true;
            }
            ("EQ", Arg::Two(s, t)) => {
                if !self.exchange(s, t, true) {
                    result.error = true;
                    result.message = Some("search failed".to_string());
                }
                self.dirty = true;
            }
            // --- case sensitivity ---
            ("UC", Arg::None) => self.case_insensitive = true,
            ("LC", Arg::None) => self.case_insensitive = false,
            // --- blocks ---
            ("BS", Arg::None) => b.block_start = Some(b.cursor.line),
            ("BE", Arg::None) => b.block_end = Some(b.cursor.line),
            ("DB", Arg::None) => {
                if let Err(msg) = b.delete_block() {
                    result.error = true;
                    result.message = Some(msg.to_string());
                }
                self.dirty = true;
            }
            ("IB", Arg::None) => {
                if let Err(msg) = b.insert_block() {
                    result.error = true;
                    result.message = Some(msg.to_string());
                }
                self.dirty = true;
            }
            ("SB", Arg::None) => {
                if let Some((s, _)) = b.block_lines() {
                    b.top = s;
                    b.cursor.line = s;
                    b.cursor.col = 0;
                } else {
                    result.error = true;
                    result.message = Some("No block marked".to_string());
                }
            }
            // --- margins and tabs ---
            ("SL", Arg::Num(n)) => self.left_margin = (*n).max(1) as usize,
            ("SR", Arg::Num(n)) => self.right_margin = *n as usize,
            ("ST", Arg::Num(n)) => self.tab_distance = *n as usize,
            ("EX", Arg::None) => self.right_margin = usize::MAX,
            // --- state ---
            ("SH", Arg::None) => self.sh_show = true,
            // --- files ---
            ("SA", arg) => {
                let target = match arg {
                    Arg::Str(s) => s.clone(),
                    _ => self.from_file.clone(),
                };
                match std::fs::write(&target, b.to_text()) {
                    Ok(()) => {
                        self.dirty = false;
                        result.message = Some("Writing to file".to_string());
                    }
                    Err(_) => {
                        result.error = true;
                        result.message = Some("Unable to open file".to_string());
                    }
                }
            }
            ("IF", Arg::Str(path)) => match std::fs::read_to_string(path) {
                Ok(t) => {
                    let tail = Buffer::from_text(&t);
                    let at = b.cursor.line + 1;
                    for (i, l) in tail.lines.iter().enumerate() {
                        b.lines.insert(at + i, l.clone());
                    }
                    self.dirty = true;
                }
                Err(_) => {
                    result.error = true;
                    result.message = Some("Unable to open file".to_string());
                }
            },
            ("WB", Arg::Str(path)) => {
                let (s, e) = match b.block_lines() {
                    Some(x) => x,
                    None => {
                        result.error = true;
                        result.message = Some("No block marked".to_string());
                        return;
                    }
                };
                let text: String = b.lines[s..=e]
                    .iter()
                    .map(|l| {
                        let mut s = l.clone();
                        s.push('\n');
                        s
                    })
                    .collect();
                match std::fs::write(path, text) {
                    Ok(()) => result.message = Some("Writing to file".to_string()),
                    Err(_) => {
                        result.error = true;
                        result.message = Some("Unable to open file".to_string());
                    }
                }
            }
            // --- program control ---
            ("Q", Arg::None) => {
                if self.dirty {
                    result.message = Some("Edits will be lost - type Y to confirm: ".to_string());
                    result.quit = true; // UI layer asks for Y/N
                } else {
                    result.quit = true;
                }
            }
            ("X", Arg::None) => {
                result.quit = true;
                result.quit_save = true;
            }
            _ => {
                result.error = true;
                result.message = Some("Unknown command".to_string());
            }
        }
    }

    /// Forward find: searches from one place past the cursor
    /// (spec 5.1). Cursor lands at the start of the match.
    fn find(&mut self, needle: &str) -> bool {
        let b = &mut self.buffer;
        let starts = [
            (b.cursor.line, b.cursor.col + 1),
            if b.cursor.line + 1 < b.line_count() {
                (b.cursor.line + 1, 0)
            } else {
                (usize::MAX, 0)
            },
        ];
        for &(line, col) in &starts {
            if line == usize::MAX {
                break;
            }
            for l in line..b.line_count() {
                let text = &b.lines[l];
                let from = if l == line { col } else { 0 };
                if let Some(pos) = find_in(text, needle, from, self.case_insensitive) {
                    b.cursor.line = l;
                    b.cursor.col = pos;
                    return true;
                }
            }
        }
        false
    }

    /// Backward find: last occurrence before the cursor.
    fn find_back(&mut self, needle: &str) -> bool {
        let b = &mut self.buffer;
        for l in (0..=b.cursor.line).rev() {
            let text = &b.lines[l];
            let from = if l == b.cursor.line {
                b.cursor.col
            } else {
                text.chars().count()
            };
            if let Some(pos) = rfind_in(text, needle, from, self.case_insensitive) {
                b.cursor.line = l;
                b.cursor.col = pos;
                return true;
            }
        }
        false
    }

    /// Exchange the next occurrence; query=true asks (handled by the UI
    /// layer via the pending_exchange state).
    fn exchange(&mut self, needle: &str, repl: &str, query: bool) -> bool {
        if needle.is_empty() {
            // empty search string inserts the second string at the
            // cursor (spec 5.1)
            let col = self.buffer.cursor.col;
            let line = &mut self.buffer.lines[self.buffer.cursor.line];
            let col_b = char_to_byte(line, col);
            line.insert_str(col_b, repl);
            self.buffer.cursor.col += repl.chars().count();
            return true;
        }
        if !self.find(needle) {
            return false;
        }
        self.pending_exchange = Some((needle.to_string(), repl.to_string()));
        if query {
            // UI layer confirms with Y/N before apply
            return true;
        }
        self.apply_exchange(repl);
        true
    }

    fn apply_exchange(&mut self, repl: &str) {
        let needle = match &self.pending_exchange {
            Some((n, _)) => n.clone(),
            None => return,
        };
        let b = &mut self.buffer;
        let pos = b.cursor.col; // find() left the cursor at the match
        let line = &mut b.lines[b.cursor.line];
        let pos_b = char_to_byte(line, pos);
        let needle_len_b = needle.len();
        if line.as_bytes().len() >= pos_b + needle_len_b {
            line.replace_range(pos_b..pos_b + needle_len_b, repl);
            b.cursor.col = pos + repl.chars().count();
        }
        self.pending_exchange = None;
    }

    /// SH display content (spec section 7).
    pub fn sh_text(&self) -> String {
        let b = &self.buffer;
        let bs = b
            .block_start
            .map(|l| b.lines[l].clone())
            .unwrap_or_else(|| "Unset".to_string());
        let be = b
            .block_end
            .map(|l| b.lines[l].clone())
            .unwrap_or_else(|| "Unset".to_string());
        let mut s = String::new();
        let _ = writeln!(
            s,
            "Editing file   : {}\n\
             Tab distance  : {:03}\n\
             Left margin   : {:03}\n\
             Right margin  : {:03}\n\
             Block start   : {}\n\
             Block end     : {}\n\
             Buffer usage  : {:03}%",
            self.from_file,
            self.tab_distance,
            self.left_margin,
            self.right_margin,
            truncate(&bs, 30),
            truncate(&be, 30),
            0
        );
        s
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect()
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices()
        .nth(ci)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn find_in(hay: &str, needle: &str, from_char: usize, ci: bool) -> Option<usize> {
    if needle.is_empty() {
        return Some(from_char.min(hay.chars().count()));
    }
    let hay_v: Vec<char> = hay.chars().collect();
    let need_v: Vec<char> = needle.chars().collect();
    if from_char >= hay_v.len() {
        return None;
    }
    for start in from_char..=hay_v.len().saturating_sub(need_v.len()) {
        let mut ok = true;
        for (i, nc) in need_v.iter().enumerate() {
            let hc = hay_v[start + i];
            let (hc, nc) = if ci {
                (hc.to_lowercase().next().unwrap(), nc.to_lowercase().next().unwrap())
            } else {
                (hc, *nc)
            };
            if hc != nc {
                ok = false;
                break;
            }
        }
        if ok {
            return Some(start);
        }
    }
    None
}

fn rfind_in(hay: &str, needle: &str, upto_char: usize, ci: bool) -> Option<usize> {
    let hay_v: Vec<char> = hay.chars().collect();
    let need_v: Vec<char> = needle.chars().collect();
    if needle.is_empty() || hay_v.len() < need_v.len() {
        return None;
    }
    let max_start = upto_char.saturating_sub(need_v.len());
    let mut start = max_start;
    loop {
        let mut ok = true;
        for (i, nc) in need_v.iter().enumerate() {
            let hc = hay_v[start + i];
            let (hc, nc) = if ci {
                (hc.to_lowercase().next().unwrap(), nc.to_lowercase().next().unwrap())
            } else {
                (hc, *nc)
            };
            if hc != nc {
                ok = false;
                break;
            }
        }
        if ok {
            return Some(start);
        }
        if start == 0 {
            return None;
        }
        start -= 1;
    }
}
