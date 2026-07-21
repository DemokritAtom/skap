//! Consistent, colored terminal output helpers used by every command.
//!
//! Also owns the global "emoji or ASCII" mode, which is initialised once
//! from CLI flags + config + environment heuristics and then read freely
//! from anywhere without further argument plumbing.

use std::sync::atomic::{AtomicU8, Ordering};

use colored::Colorize;

// ---------------------------------------------------------------------------
// Emoji mode (process-global)
// ---------------------------------------------------------------------------

const EMOJI_AUTO: u8 = 0;
const EMOJI_ON: u8 = 1;
const EMOJI_OFF: u8 = 2;

static EMOJI_MODE: AtomicU8 = AtomicU8::new(EMOJI_AUTO);

/// Initialise emoji mode for the running process.
///
/// `cli_no_emoji` is the `--no-emoji` flag, `config_emoji` is the
/// `defaults.emoji` config value (defaults to `true`).
pub fn init_emoji(cli_no_emoji: bool, config_emoji: bool) {
    let mode = if cli_no_emoji || !config_emoji {
        EMOJI_OFF
    } else if emoji_terminal_supported() {
        EMOJI_ON
    } else {
        EMOJI_OFF
    };
    EMOJI_MODE.store(mode, Ordering::Relaxed);
}

/// True if the current process should print emoji.
pub fn emoji_enabled() -> bool {
    match EMOJI_MODE.load(Ordering::Relaxed) {
        EMOJI_ON => true,
        EMOJI_OFF => false,
        _ => emoji_terminal_supported(),
    }
}

/// Heuristic: assume emoji works only if we have a real UTF-8 terminal
/// and aren't running inside a CI environment.
fn emoji_terminal_supported() -> bool {
    if std::env::var("CI").is_ok() {
        return false;
    }
    let term_ok = std::env::var("TERM").map(|t| t != "dumb").unwrap_or(false);
    let lang_ok = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .map(|l| l.to_uppercase().contains("UTF"))
        .unwrap_or(false);
    term_ok && lang_ok
}

/// Symbols for the docker compose status of a project. Returned as a
/// colored, fixed-width string so columns line up in `skap list`.
pub fn status_symbol(s: crate::core::docker::Status) -> String {
    use crate::core::docker::Status::*;
    if emoji_enabled() {
        match s {
            Up => "🟢 UP".to_string(),
            Down => "⚪ OFF".to_string(),
            Error => "🔴 ERR".to_string(),
            Unknown => "─".to_string(),
        }
    } else {
        match s {
            Up => "UP ".green().to_string(),
            Down => "OFF".to_string(),
            Error => "ERR".red().to_string(),
            Unknown => "-  ".to_string(),
        }
    }
}

/// Symbol for a port's "is currently bound by some process" state.
pub fn port_status_symbol(in_use: bool) -> String {
    if emoji_enabled() {
        if in_use {
            "🟢 aktiv".to_string()
        } else {
            "⚪ frei".to_string()
        }
    } else if in_use {
        "USED".green().to_string()
    } else {
        "FREE".to_string()
    }
}

/// Character count of `s` ignoring ANSI escape sequences (`\x1b[...m`).
/// Table layout code needs this instead of `s.chars().count()` for any
/// string that may have already been colored via the `colored` crate –
/// counting the escape bytes as visible characters throws off column
/// alignment as soon as colors are enabled.
pub fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip `ESC [ ... <final byte>` (CSI sequence).
            if chars.next() == Some('[') {
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

// ---------------------------------------------------------------------------
// Print helpers
// ---------------------------------------------------------------------------

fn sym(emoji: &str, plain: &str) -> String {
    if emoji_enabled() {
        emoji.to_string()
    } else {
        plain.to_string()
    }
}

/// Green checkmark – successful operation.
pub fn success(msg: &str) {
    println!("{} {}", sym("✓", "[ok]").green().bold(), msg);
}

/// Red cross – fatal/expected error written to stderr.
pub fn error(msg: &str) {
    eprintln!("{} {}", sym("✗", "[!!]").red().bold(), msg);
}

/// Yellow warning sign – non-fatal issue.
pub fn warn(msg: &str) {
    println!("{} {}", sym("⚠", "[?]").yellow().bold(), msg);
}

/// Cyan arrow – hint, suggested next action, or generic info.
pub fn info(msg: &str) {
    println!("{} {}", sym("→", "->").cyan(), msg);
}

/// Dimmed dot – subordinate progress step.
pub fn step(msg: &str) {
    println!("{} {}", sym("·", "..").dimmed(), msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_width_ignores_ansi_codes() {
        // Built by hand rather than via `colored`, whose output depends
        // on TTY/env detection and would make this test flaky under
        // `cargo test` (no real terminal attached, colors may be
        // suppressed automatically).
        let colored = "\x1b[32mUP\x1b[0m";
        assert_eq!(visible_width(colored), 2);
    }

    #[test]
    fn visible_width_matches_plain_char_count_without_colors() {
        assert_eq!(visible_width("hello"), 5);
        assert_eq!(visible_width(""), 0);
    }

    #[test]
    fn visible_width_handles_multiple_sequences() {
        let s = "\x1b[31ma\x1b[0m\x1b[1mbc\x1b[0m";
        assert_eq!(visible_width(s), 3);
    }
}
