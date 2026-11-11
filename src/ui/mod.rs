//! Drawing. Each tab lives in its own module; modals are drawn on top.

mod apps;
mod confirm;
mod help;
mod overview;
mod processes;

use std::time::Duration;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode, Tab};
use crate::theme::Theme;

const STATUS_TTL: Duration = Duration::from_secs(5);

pub fn draw(f: &mut Frame, app: &mut App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());

    draw_header(f, app, header);
    match app.tab {
        Tab::Overview => overview::draw(f, app, body),
        Tab::Processes => processes::draw(f, app, body),
        Tab::Apps => apps::draw(f, app, body),
    }
    draw_footer(f, app, footer);

    match &app.mode {
        Mode::Help => help::draw(f, &app.theme.theme),
        Mode::Confirm(p) => confirm::draw(f, &app.theme.theme, p),
        _ => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let mut spans = vec![
        Span::styled(
            " System Monitor ",
            Style::new().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let style = if *tab == app.tab {
            Style::new()
                .fg(t.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::new().fg(t.muted)
        };
        spans.push(Span::styled(format!(" {} {} ", i + 1, tab.title()), style));
    }
    f.render_widget(Line::from(spans), area);

    let s = &app.collector.snap;
    let right = format!(
        "load {:.2} {:.2} {:.2}  up {}  every {}ms ",
        s.load.0,
        s.load.1,
        s.load.2,
        uptime(s.uptime),
        app.tick.as_millis()
    );
    f.render_widget(
        Line::from(Span::styled(right, Style::new().fg(t.muted))).right_aligned(),
        area,
    );
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let line = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" filter: ", Style::new().fg(t.accent)),
            Span::raw(app.filter.clone()),
            Span::styled("▏", Style::new().fg(t.accent)),
            Span::styled("   Enter keep · Esc clear", Style::new().fg(t.muted)),
        ])
    } else if let Some(st) = app.status.as_ref().filter(|s| s.at.elapsed() < STATUS_TTL) {
        Line::from(Span::styled(
            format!(" {}", st.text),
            Style::new().fg(if st.error { t.red } else { t.green }),
        ))
    } else {
        let hints = match app.tab {
            Tab::Overview => "Tab switch · / search processes · +/- speed · ? help · q quit",
            Tab::Processes => {
                "j/k move · x stop · X kill · s sort · r reverse · t tree · / filter · ? help"
            }
            Tab::Apps => "j/k move · x close windows · X kill app · ? help · q quit",
        };
        Line::from(Span::styled(format!(" {hints}"), Style::new().fg(t.muted)))
    };
    f.render_widget(line, area);
}

/// A rounded panel with the title in the accent color.
pub fn panel<'a>(t: &Theme, title: impl Into<Line<'a>>) -> Block<'a> {
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(t.muted))
        .title(
            title
                .into()
                .style(Style::new().fg(t.accent).add_modifier(Modifier::BOLD)),
        )
}

/// A rectangle of `w`×`h` cells in the middle of `area`.
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let [r] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [r] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(r);
    r
}

pub fn bytes(b: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut v = b as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{b}B")
    } else if v >= 100.0 {
        format!("{v:.0}{}", UNITS[u])
    } else {
        format!("{v:.1}{}", UNITS[u])
    }
}

pub fn rate(bps: f64) -> String {
    format!("{}/s", bytes(bps as u64))
}

fn uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, secs / 3600 % 24, secs / 60 % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else {
        format!("{h}h {m:02}m")
    }
}

pub fn empty_message<'a>(t: &Theme, text: &'a str) -> Paragraph<'a> {
    Paragraph::new(Span::styled(text, Style::new().fg(t.muted)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        assert_eq!(bytes(512), "512B");
        assert_eq!(bytes(1536), "1.5K");
        assert_eq!(bytes(300 * 1024 * 1024), "300M");
        assert_eq!(bytes(16 * 1024 * 1024 * 1024), "16.0G");
    }

    #[test]
    fn formats_uptime() {
        assert_eq!(uptime(3 * 3600 + 5 * 60), "3h 05m");
        assert_eq!(uptime(2 * 86400 + 4 * 3600), "2d 4h");
    }
}
