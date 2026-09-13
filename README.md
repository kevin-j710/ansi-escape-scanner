# ansi-escape-scanner

Terminal escape codes are invisible until they aren't. A log file with color
codes in it looks fine in your terminal and turns into `^[[31m` garbage in an
editor, a CI log viewer, or anywhere else that doesn't interpret them. When
something is emitting the wrong sequence, or too many of them, or a stray one
that resets your terminal, you end up eyeballing raw bytes trying to figure
out what `\x1b]0;...` or `\x1b[38;5;208m` actually means.

This is a small library (`ansi_escape_scanner`) plus a CLI (`escan`) that
scans a byte stream, finds every escape sequence in it, and tells you what
each one does - in plain text or as JSON.

It only recognizes ANSI/VT100-family sequences: CSI (`ESC [ ... final`),
OSC (`ESC ] ... BEL` / `ESC ] ... ST`), and short two-byte sequences like
`ESC 7`. It does not try to be a terminfo database or a terminal emulator.

## Building

Standard library only, no dependencies to fetch:

```
cargo build --release
```

## Usage

Scan a file:

```
$ escan build.log
```

Scan whatever a command prints, by piping it in:

```
$ printf 'plain \x1b[1;31mbold red\x1b[0m plain\n' | escan
     6  \x1b[1;31m                   bold, red foreground
    20  \x1b[0m                      reset
```

Same thing as JSON, for feeding into another tool:

```
$ printf 'plain \x1b[1;31mbold red\x1b[0m plain\n' | escan --json
[{"offset":6,"raw":"\x1b[1;31m","description":"bold, red foreground"},{"offset":20,"raw":"\x1b[0m","description":"reset"}]
```

256-color and truecolor SGR sequences are decoded too:

```
$ printf '\x1b[38;5;208mtext\x1b[38;2;0;128;255m\x1b[0m' | escan
     0  \x1b[38;5;208m                256-color foreground (index 208)
    24  \x1b[38;2;0;128;255m          truecolor foreground (#0080ff)
    43  \x1b[0m                      reset
```

An OSC sequence (these are what set a terminal's window title, among other
things):

```
$ printf '\x1b]0;session name\x07' | escan
     0  \x1b]0;session name\x07       OSC: 0;session name
```

## As a library

```rust
use ansi_escape_scanner::scan;

let tokens = scan(b"\x1b[2J\x1b[H");
for t in &tokens {
    println!("{}: {}", t.offset, t.description());
}
// 0: erase entire screen
// 4: cursor position: row 1, col 1
```

## Scope

This decodes sequences; it doesn't build them, and it doesn't track
terminal state (cursor position, scroll region, current attributes) across
a stream. See the roadmap for what's planned next.

## License

MIT, see LICENSE.
