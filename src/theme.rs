//! Colors that follow the active Omarchy theme.
//!
//! Omarchy writes the palette of the current theme to `colors.toml`. A theme
//! switch replaces the file (or the directory symlink above it), so rather
//! than relying on inotify we re-read the small file on every tick and reload
//! when its contents change.

use std::fs;
use std::path::PathBuf;

use ratatui::style::Color;
use serde::Deserialize;

/// The accent Omarchy apps fall back to when no theme is installed.
const FALLBACK_ACCENT: Color = Color::Rgb(0xFF, 0xD6, 0x0A);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub accent: Color,
    pub fg: Color,
    pub muted: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub red: Color,
    pub yellow: Color,
    pub green: Color,
    pub cyan: Color,
    pub blue: Color,
    pub magenta: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent: FALLBACK_ACCENT,
            fg: Color::Reset,
            muted: Color::DarkGray,
            selection_bg: Color::Rgb(0x35, 0x35, 0x43),
            selection_fg: Color::Reset,
            red: Color::Red,
            yellow: Color::Yellow,
            green: Color::Green,
            cyan: Color::Cyan,
            blue: Color::Blue,
            magenta: Color::Magenta,
        }
    }
}

#[derive(Deserialize, Default)]
struct RawColors {
    accent: Option<String>,
    foreground: Option<String>,
    muted: Option<String>,
    selection: Option<String>,
    red: Option<String>,
    yellow: Option<String>,
    green: Option<String>,
    cyan: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
}

pub fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

impl Theme {
    /// Builds a theme from the contents of `colors.toml`. Keys that are
    /// missing or malformed keep their default, so a partial theme still works.
    pub fn from_toml(src: &str) -> Theme {
        let raw: RawColors = toml::from_str(src).unwrap_or_default();
        let d = Theme::default();
        let pick = |v: &Option<String>, fallback: Color| {
            v.as_deref().and_then(parse_hex).unwrap_or(fallback)
        };
        Theme {
            accent: pick(&raw.accent, d.accent),
            fg: pick(&raw.foreground, d.fg),
            muted: pick(&raw.muted, d.muted),
            selection_bg: pick(&raw.selection, d.selection_bg),
            // The theme's `selection_foreground` is tuned for an accent-coloured
            // selection; on our muted selection bar the normal foreground reads better.
            selection_fg: pick(&raw.foreground, d.selection_fg),
            red: pick(&raw.red, d.red),
            yellow: pick(&raw.yellow, d.yellow),
            green: pick(&raw.green, d.green),
            cyan: pick(&raw.cyan, d.cyan),
            blue: pick(&raw.blue, d.blue),
            magenta: pick(&raw.magenta, d.magenta),
        }
    }

    /// Green, yellow or red depending on how close `pct` is to full.
    pub fn load_color(&self, pct: f64) -> Color {
        if pct >= 80.0 {
            self.red
        } else if pct >= 50.0 {
            self.yellow
        } else {
            self.green
        }
    }
}

pub struct ThemeWatcher {
    path: PathBuf,
    last: Option<String>,
    pub theme: Theme,
}

impl ThemeWatcher {
    pub fn new(path: PathBuf) -> Self {
        let mut w = ThemeWatcher {
            path,
            last: None,
            theme: Theme::default(),
        };
        w.poll();
        w
    }

    /// The colors file of the active Omarchy theme.
    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        home.join(".local/state/omarchy/current/theme/colors.toml")
    }

    /// Re-reads the file and returns true when the theme changed.
    pub fn poll(&mut self) -> bool {
        let current = fs::read_to_string(&self.path).ok();
        if current == self.last {
            return false;
        }
        self.theme = current.as_deref().map(Theme::from_toml).unwrap_or_default();
        self.last = current;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_theme() {
        let t =
            Theme::from_toml("accent = \"#89b4fa\"\nred = \"#f38ba8\"\nforeground = \"#cdd6f4\"\n");
        assert_eq!(t.accent, Color::Rgb(0x89, 0xb4, 0xfa));
        assert_eq!(t.red, Color::Rgb(0xf3, 0x8b, 0xa8));
        assert_eq!(t.fg, Color::Rgb(0xcd, 0xd6, 0xf4));
    }

    #[test]
    fn missing_keys_fall_back() {
        let t = Theme::from_toml("accent = \"#112233\"\ngreen = \"nope\"\n");
        assert_eq!(t.accent, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(t.green, Theme::default().green);
    }

    #[test]
    fn garbage_gives_default() {
        assert_eq!(Theme::from_toml("{{{ not toml"), Theme::default());
    }

    #[test]
    fn watcher_reloads_on_change_and_survives_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("colors.toml");
        let mut w = ThemeWatcher::new(path.clone());
        assert_eq!(w.theme.accent, FALLBACK_ACCENT);

        fs::write(&path, "accent = \"#010203\"").unwrap();
        assert!(w.poll());
        assert_eq!(w.theme.accent, Color::Rgb(1, 2, 3));
        assert!(!w.poll());

        fs::remove_file(&path).unwrap();
        assert!(w.poll());
        assert_eq!(w.theme.accent, FALLBACK_ACCENT);
    }
}
