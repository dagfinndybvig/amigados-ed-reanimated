//! Headless mode: run Ed extended commands from a script against a
//! buffer without the TUI. Used for testing the command engine and,
//! later, for differential testing against the original editor.

use crate::cmdline::{self, Editor};
use std::io::{BufRead, BufReader, Read};

/// Run commands from `input` (one command line per line) against the
/// editor. On X, saves the FROM file and the t/ed-backup file (spec
/// section 6: FROM first, then t/ed-backup relative to the current
/// directory). Prints status messages to stdout.
pub fn run_script(ed: &mut Editor, input: &mut dyn Read) -> ScriptOutcome {
    let reader = BufReader::new(input);
    let mut outcome = ScriptOutcome::Ran;
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let items = match cmdline::parse_line(line) {
            Ok(items) => items,
            Err(e) => {
                println!("? {}", e);
                continue;
            }
        };
        let r = ed.run_line(&items);
        if let Some(m) = &r.message {
            if m != "Exchange? " {
                println!("> {}", m);
            }
        }
        if ed.sh_show {
            println!("{}", ed.sh_text());
            ed.sh_show = false;
        }
        if r.quit {
            if r.quit_save {
                save_files(ed);
            }
            outcome = ScriptOutcome::Quit;
            break;
        }
    }
    outcome
}

pub enum ScriptOutcome {
    Ran,
    Quit,
}

/// X semantics (spec section 6, corrected): write the buffer to the
/// FROM file first, then write the buffer to t/ed-backup relative to
/// the current directory.
pub fn save_files(ed: &Editor) {
    let text = ed.buffer.to_text();
    match std::fs::write(&ed.from_file, &text) {
        Ok(()) => println!("Writing to file"),
        Err(_) => println!("Unable to open file"),
    }
    let backup = std::path::Path::new("t").join("ed-backup");
    if let Some(parent) = backup.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&backup, &text) {
        Ok(()) => println!("Writing to file"),
        Err(_) => println!("Unable to open file"),
    }
}
