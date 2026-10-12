//! Ed — clean-room reimplementation of the AmigaDOS Ed 1.14 editor.
//!
//! Written from the behavior specification only (docs/ed-spec.md),
//! per the clean-room rules in LEGAL.md. No AmigaOS code is used or
//! included here.

mod buffer;
mod cmdline;
mod headless;
mod screen;

use cmdline::Editor;
use std::io::Read;
use std::process::ExitCode;

/// ReadArgs template is FROM/A,SIZE (spec section 2): SIZE may be given
/// as a keyword (SIZE n) or positionally (n). The buffer floor in the
/// binary is 10,000 bytes (spec section 2 correction).
fn parse_args(args: Vec<String>) -> Result<(String, Option<usize>), String> {
    let mut from: Option<String> = None;
    let mut size: Option<usize> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let upper = a.to_uppercase();
        if upper == "SIZE" {
            if let Some(v) = args.get(i + 1) {
                size = v.parse::<usize>().ok().or(None);
                i += 2;
                continue;
            } else {
                return Err("Invalid number given for SIZE".to_string());
            }
        } else if let Ok(n) = a.parse::<usize>() {
            size = Some(n);
            i += 1;
            continue;
        } else if from.is_none() {
            from = Some(a.clone());
            i += 1;
            continue;
        }
        return Err("Bad arguments".to_string());
    }
    match from {
        Some(f) => Ok((f, size)),
        None => Err("Bad arguments".to_string()),
    }
}

fn load(from: &str, size: usize) -> Result<Option<String>, String> {
    let path = std::path::Path::new(from);
    if path.is_dir() {
        return Err(format!("{} is a directory and cannot be edited", from));
    }
    if !path.exists() {
        println!("Creating new file");
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|_| "Unable to open file".to_string())?;
    if bytes.iter().any(|&b| b == 0) {
        return Err("File contains binary".to_string());
    }
    let s = String::from_utf8_lossy(&bytes).to_string();
    if s.len() > size {
        return Err("File too big".to_string());
    }
    Ok(Some(s))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // --script <file> | --script - (stdin): headless command run
    if args.len() >= 2 && args[0] == "--parse" {
        // debug: parse and print the command tree
        let items = match cmdline::parse_line(&args[1]) {
            Ok(items) => items,
            Err(e) => { eprintln!("parse error: {}", e); return ExitCode::FAILURE; }
        };
        for it in &items { println!("{:#?}", it); }
        return ExitCode::SUCCESS;
    }
    let script = if args.len() >= 2 && args[0] == "--script" {
        Some(args[1].clone())
    } else {
        None
    };
    let rest: Vec<String> = if script.is_some() {
        args[2..].to_vec()
    } else {
        args
    };
    let (from, size) = match parse_args(rest) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    const FLOOR: usize = 10_000;
    let size = size.unwrap_or(FLOOR);
    if size < FLOOR {
        eprintln!("SIZE of {} too small", size);
        return ExitCode::FAILURE;
    }
    let text = match load(&from, size) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    let existed = text.is_some();
    let mut ed = Editor::new(&from, text);
    ed.quiet_create = !existed;

    if let Some(script_src) = script {
        let mut input: Box<dyn Read> = if script_src == "-" {
            Box::new(std::io::stdin())
        } else {
            match std::fs::File::open(&script_src) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    eprintln!("unable to open script: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        };
        let _ = headless::run_script(&mut ed, &mut *input);
        return ExitCode::SUCCESS;
    }

    match screen::run_loop(&mut ed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("terminal error: {}", e);
            ExitCode::FAILURE
        }
    }
}
