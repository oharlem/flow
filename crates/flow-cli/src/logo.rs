//! Animated brand logo rendered on bare `flow` and top-level `flow --help`.
//!
//! A single ~620ms reveal: vertical blue gradient with a diagonal aurora
//! highlight sweeping upper-left → lower-right across the wordmark, with
//! a static cyan bar-wave underline. Renders an 8-row footprint via
//! cursor-up redraws so the final frame settles into scrollback.
//!
//! CLI startup gates on [`can_animate`] before calling [`show`]; piped,
//! `NO_COLOR`, `CI`, and `TERM=dumb` invocations stay logo-free.

use std::io::{self, IsTerminal, Write};
use std::thread::sleep;
use std::time::Duration;

const WORDMARK: [&str; 6] = [
    "███████╗██╗      ██████╗ ██╗    ██╗",
    "██╔════╝██║     ██╔═══██╗██║    ██║",
    "█████╗  ██║     ██║   ██║██║ █╗ ██║",
    "██╔══╝  ██║     ██║   ██║██║███╗██║",
    "██║     ███████╗╚██████╔╝╚███╔███╔╝",
    "╚═╝     ╚══════╝ ╚═════╝  ╚══╝╚══╝ ",
];
const WAVE: &str = "▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇";
const TAGLINE: &str = "  spec-driven workflow toolkit";

const FOOTPRINT: usize = 8;

pub(crate) type Rgb = (u8, u8, u8);

pub(crate) const BRAND_BLUE: Rgb = (30, 136, 229);

const GRADIENT: [Rgb; 6] = [
    (168, 218, 220),
    (91, 192, 235),
    BRAND_BLUE,
    (21, 101, 192),
    (13, 71, 161),
    (10, 42, 94),
];
const WAVE_COLOR: Rgb = (79, 195, 247);
const HIGHLIGHT: Rgb = (224, 247, 250);

const RESET: &str = "\x1B[0m";
const HIDE_CURSOR: &str = "\x1B[?25l";
const SHOW_CURSOR: &str = "\x1B[?25h";
const CLEAR_LINE: &str = "\r\x1B[2K";

/// Returns `true` when stdout is an interactive, color-capable terminal
/// suitable for animation. CLI startup paths gate on this so that piped,
/// scripted, or CI invocations stay logo-free.
pub fn can_animate() -> bool {
    if !can_color() {
        return false;
    }
    if std::env::var("CI").map(|v| !v.is_empty()).unwrap_or(false) {
        return false;
    }
    true
}

/// Returns `true` when stdout can receive ANSI color.
pub fn can_color() -> bool {
    if !io::stdout().is_terminal() {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if matches!(std::env::var("TERM").as_deref(), Ok("dumb")) {
        return false;
    }
    true
}

/// Play the animated logo. Caller must verify [`can_animate`] first.
pub fn show() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "{HIDE_CURSOR}")?;
    for _ in 0..FOOTPRINT {
        writeln!(out)?;
    }
    let result = animate(&mut out);
    write!(out, "{SHOW_CURSOR}")?;
    out.flush()?;
    result
}

fn animate<W: Write>(out: &mut W) -> io::Result<()> {
    let frames = 22;
    for f in 0..=frames {
        let sweep = -4.0 + 50.0 * (f as f32 / frames as f32);
        render(out, sweep)?;
        out.flush()?;
        sleep(Duration::from_millis(28));
    }
    render(out, 9999.0)
}

fn render<W: Write>(out: &mut W, sweep: f32) -> io::Result<()> {
    write!(out, "\x1B[{FOOTPRINT}A")?;
    for (r, line) in WORDMARK.iter().enumerate() {
        write!(out, "{CLEAR_LINE}")?;
        for (c, ch) in line.chars().enumerate() {
            if ch == ' ' {
                write!(out, " ")?;
                continue;
            }
            let diagonal = c as f32 + (r as f32) * 1.8;
            let dist = (diagonal - sweep).abs();
            let color = if dist < 2.0 {
                lerp(GRADIENT[r], HIGHLIGHT, 1.0 - dist / 2.0)
            } else {
                GRADIENT[r]
            };
            write_fg(out, color)?;
            write!(out, "{ch}{RESET}")?;
        }
        writeln!(out)?;
    }
    write!(out, "{CLEAR_LINE}")?;
    write_fg(out, WAVE_COLOR)?;
    writeln!(out, "{WAVE}{RESET}")?;
    write!(out, "{CLEAR_LINE}\x1B[2;3m{TAGLINE}{RESET}")?;
    writeln!(out)?;
    Ok(())
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (
        lerp_u8(a.0, b.0, t),
        lerp_u8(a.1, b.1, t),
        lerp_u8(a.2, b.2, t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn write_fg<W: Write>(out: &mut W, c: Rgb) -> io::Result<()> {
    write!(out, "\x1B[38;2;{};{};{}m", c.0, c.1, c.2)
}
