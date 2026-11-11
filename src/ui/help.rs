//! The `?` overlay listing every key.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use super::{centered, panel};
use crate::theme::Theme;

pub const KEYS: &[(&str, &str)] = &[
    ("Tab / Shift+Tab", "next / previous tab"),
    ("1 2 3", "Overview, Processes, Apps"),
    ("j k / ↓ ↑", "move selection"),
    ("PgDn PgUp", "move ten rows"),
    ("g G / Home End", "first / last row"),
    ("/", "filter processes (Enter keeps, Esc clears)"),
    ("s", "cycle sort: cpu, mem, pid, name"),
    ("r", "reverse sort"),
    ("t", "toggle process tree"),
    ("x / Del", "stop process (SIGTERM) · close app windows"),
    ("X", "kill process or whole app (SIGKILL)"),
    ("+ / -", "refresh faster / slower"),
    ("?", "this help"),
    ("q / Ctrl+C", "quit"),
];

pub fn draw(f: &mut Frame, t: &Theme) {
    let area = centered(f.area(), 64, KEYS.len() as u16 + 4);
    let mut lines: Vec<Line> = KEYS
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<18}"), Style::new().fg(t.accent)),
                Span::raw(*d),
            ])
        })
        .collect();
    lines.push(Line::default());
    lines.push(Line::styled(
        " press any key to close",
        Style::new().fg(t.muted),
    ));
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .block(panel(t, " Keys "))
            .style(Style::new().fg(t.fg)),
        area,
    );
}
