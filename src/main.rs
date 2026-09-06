use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use ansi_escape_scanner::{scan, Token};

fn main() -> ExitCode {
    let mut json = false;
    let mut path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other if !other.starts_with('-') => path = Some(other.to_string()),
            other => {
                eprintln!("escan: unrecognized option '{}'", other);
                print_usage();
                return ExitCode::FAILURE;
            }
        }
    }

    let input = match read_input(path.as_deref()) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("escan: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let tokens = scan(&input);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let result = if json {
        write_json(&mut out, &tokens)
    } else {
        write_human(&mut out, &tokens)
    };

    if let Err(e) = result {
        eprintln!("escan: failed to write output: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn read_input(path: Option<&str>) -> io::Result<Vec<u8>> {
    match path {
        Some(p) => fs::read(p),
        None => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;
            Ok(buf)
        }
    }
}

fn write_human(out: &mut impl Write, tokens: &[Token]) -> io::Result<()> {
    if tokens.is_empty() {
        writeln!(out, "no escape sequences found")?;
        return Ok(());
    }
    for t in tokens {
        writeln!(
            out,
            "{:>6}  {:<28}  {}",
            t.offset,
            t.raw_display(),
            t.description()
        )?;
    }
    Ok(())
}

fn write_json(out: &mut impl Write, tokens: &[Token]) -> io::Result<()> {
    write!(out, "[")?;
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            write!(out, ",")?;
        }
        write!(
            out,
            "{{\"offset\":{},\"raw\":\"{}\",\"description\":\"{}\"}}",
            t.offset,
            json_escape(&t.raw_display()),
            json_escape(&t.description())
        )?;
    }
    writeln!(out, "]")?;
    Ok(())
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn print_usage() {
    eprintln!("usage: escan [--json] [file]");
    eprintln!("  reads from stdin if no file is given");
}
