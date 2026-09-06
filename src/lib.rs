//! Parsing and description of ANSI/VT terminal escape sequences found in raw byte streams.
//!
//! This does not try to be a full terminfo/terminal emulator. It recognizes the three
//! shapes of escape sequence that show up in practice - CSI (`ESC [ ... final`),
//! OSC (`ESC ] ... BEL` or `... ESC \`), and short two-byte sequences like `ESC 7` -
//! and attaches a human-readable description to each one it can identify.

pub const ESC: u8 = 0x1B;
const BEL: u8 = 0x07;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// CSI: ESC [ params final_byte (e.g. ESC [ 3 1 m)
    Csi { params: Vec<i64>, final_byte: u8 },
    /// OSC: ESC ] data, terminated by BEL or ST (ESC \)
    Osc { data: String },
    /// A short escape sequence that isn't CSI or OSC, e.g. ESC 7, ESC M
    Simple { final_byte: u8 },
    /// A trailing ESC byte that wasn't followed by a recognizable sequence
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub offset: usize,
    pub raw: Vec<u8>,
    pub kind: Kind,
}

impl Token {
    pub fn description(&self) -> String {
        match &self.kind {
            Kind::Csi { params, final_byte } => describe_csi(params, *final_byte),
            Kind::Osc { data } => format!("OSC: {}", data),
            Kind::Simple { final_byte } => describe_simple(*final_byte),
            Kind::Unknown => "stray ESC byte, no recognized sequence followed".to_string(),
        }
    }

    /// Raw bytes rendered as a readable escape form, e.g. "\x1b[31m"
    pub fn raw_display(&self) -> String {
        let mut out = String::new();
        for &b in &self.raw {
            match b {
                ESC => out.push_str("\\x1b"),
                BEL => out.push_str("\\x07"),
                b'\\' => out.push_str("\\\\"),
                0x20..=0x7e => out.push(b as char),
                other => out.push_str(&format!("\\x{:02x}", other)),
            }
        }
        out
    }
}

/// Scan a byte slice and return every escape sequence found, in order of appearance.
/// Plain text between sequences is skipped; callers who need it can use `offset` and
/// `raw.len()` on consecutive tokens to find the gaps.
pub fn scan(input: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] != ESC {
            i += 1;
            continue;
        }
        let start = i;
        let (kind, len) = parse_escape(&input[i..]);
        tokens.push(Token {
            offset: start,
            raw: input[start..start + len].to_vec(),
            kind,
        });
        i += len;
    }
    tokens
}

fn parse_escape(input: &[u8]) -> (Kind, usize) {
    if input.len() < 2 {
        return (Kind::Unknown, input.len());
    }
    match input[1] {
        b'[' => parse_csi(input),
        b']' => parse_osc(input),
        b => (Kind::Simple { final_byte: b }, 2),
    }
}

fn parse_csi(input: &[u8]) -> (Kind, usize) {
    // input[0] == ESC, input[1] == '['
    let mut i = 2;
    while i < input.len() && !(0x40..=0x7e).contains(&input[i]) {
        i += 1;
    }
    if i >= input.len() {
        return (Kind::Unknown, input.len());
    }
    let final_byte = input[i];
    let params = parse_params(&input[2..i]);
    (Kind::Csi { params, final_byte }, i + 1)
}

fn parse_params(bytes: &[u8]) -> Vec<i64> {
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes
        .split(|&b| b == b';')
        .map(|field| {
            std::str::from_utf8(field)
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
        })
        .collect()
}

fn parse_osc(input: &[u8]) -> (Kind, usize) {
    // input[0] == ESC, input[1] == ']'
    let mut i = 2;
    loop {
        if i >= input.len() {
            let data = String::from_utf8_lossy(&input[2..i]).into_owned();
            return (Kind::Osc { data }, i);
        }
        if input[i] == BEL {
            let data = String::from_utf8_lossy(&input[2..i]).into_owned();
            return (Kind::Osc { data }, i + 1);
        }
        if input[i] == ESC && i + 1 < input.len() && input[i + 1] == b'\\' {
            let data = String::from_utf8_lossy(&input[2..i]).into_owned();
            return (Kind::Osc { data }, i + 2);
        }
        i += 1;
    }
}

fn describe_simple(final_byte: u8) -> String {
    match final_byte {
        b'7' => "save cursor position and attributes".to_string(),
        b'8' => "restore cursor position and attributes".to_string(),
        b'c' => "reset terminal to initial state".to_string(),
        b'D' => "index (move down, scroll if at bottom)".to_string(),
        b'M' => "reverse index (move up, scroll if at top)".to_string(),
        b'E' => "next line".to_string(),
        other => format!("simple escape sequence ESC {}", other as char),
    }
}

fn describe_csi(params: &[i64], final_byte: u8) -> String {
    match final_byte {
        b'm' => describe_sgr(params),
        b'A' => format!("cursor up {}", params.first().copied().unwrap_or(1)),
        b'B' => format!("cursor down {}", params.first().copied().unwrap_or(1)),
        b'C' => format!("cursor forward {}", params.first().copied().unwrap_or(1)),
        b'D' => format!("cursor back {}", params.first().copied().unwrap_or(1)),
        b'H' | b'f' => {
            let row = params.first().copied().unwrap_or(1);
            let col = params.get(1).copied().unwrap_or(1);
            format!("cursor position: row {}, col {}", row, col)
        }
        b'J' => match params.first().copied().unwrap_or(0) {
            0 => "erase from cursor to end of screen".to_string(),
            1 => "erase from start of screen to cursor".to_string(),
            2 => "erase entire screen".to_string(),
            3 => "erase entire screen and scrollback".to_string(),
            n => format!("erase display, unknown mode {}", n),
        },
        b'K' => match params.first().copied().unwrap_or(0) {
            0 => "erase from cursor to end of line".to_string(),
            1 => "erase from start of line to cursor".to_string(),
            2 => "erase entire line".to_string(),
            n => format!("erase line, unknown mode {}", n),
        },
        other => format!(
            "CSI sequence, final byte '{}', params {:?}",
            other as char, params
        ),
    }
}

fn describe_sgr(params: &[i64]) -> String {
    if params.is_empty() {
        return sgr_name(0);
    }
    params
        .iter()
        .map(|&p| sgr_name(p))
        .collect::<Vec<_>>()
        .join(", ")
}

fn sgr_name(code: i64) -> String {
    match code {
        0 => "reset".to_string(),
        1 => "bold".to_string(),
        2 => "dim".to_string(),
        3 => "italic".to_string(),
        4 => "underline".to_string(),
        5 => "blink".to_string(),
        7 => "reverse video".to_string(),
        8 => "hidden".to_string(),
        9 => "strikethrough".to_string(),
        22 => "normal intensity".to_string(),
        23 => "italic off".to_string(),
        24 => "underline off".to_string(),
        25 => "blink off".to_string(),
        27 => "reverse off".to_string(),
        28 => "hidden off".to_string(),
        29 => "strikethrough off".to_string(),
        30..=37 => format!("{} foreground", color_name((code - 30) as u8)),
        39 => "default foreground".to_string(),
        40..=47 => format!("{} background", color_name((code - 40) as u8)),
        49 => "default background".to_string(),
        90..=97 => format!("bright {} foreground", color_name((code - 90) as u8)),
        100..=107 => format!("bright {} background", color_name((code - 100) as u8)),
        other => format!("SGR {}", other),
    }
}

fn color_name(n: u8) -> &'static str {
    match n {
        0 => "black",
        1 => "red",
        2 => "green",
        3 => "yellow",
        4 => "blue",
        5 => "magenta",
        6 => "cyan",
        7 => "white",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_sgr_color() {
        let tokens = scan(b"\x1b[31mred\x1b[0m");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].description(), "red foreground");
        assert_eq!(tokens[1].description(), "reset");
    }

    #[test]
    fn finds_cursor_position() {
        let tokens = scan(b"\x1b[10;5H");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].description(), "cursor position: row 10, col 5");
    }

    #[test]
    fn finds_osc_title() {
        let tokens = scan(b"\x1b]0;my title\x07");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].description(), "OSC: 0;my title");
    }
}
