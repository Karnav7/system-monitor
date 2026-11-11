//! The confirmation shown before anything is stopped.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::{centered, panel};
use crate::actions::Sig;
use crate::app::Pending;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, t: &Theme, p: &Pending) {
    let (question, what, warning, danger) = match p {
        Pending::Signal {
            sig: Sig::Term,
            what,
            warning,
            ..
        } => ("Stop (SIGTERM)", what, warning.as_deref(), false),
        Pending::Signal {
            sig: Sig::Kill,
            what,
            warning,
            ..
        } => (
            "Force kill (SIGKILL)",
            what,
            warning.as_deref().or(Some("Unsaved work will be lost.")),
            true,
        ),
        Pending::CloseWindows { what, .. } => ("Close the windows of", what, None, false),
    };
    let mut lines = vec![
        Line::from(Span::styled(
            question,
            Style::new()
                .fg(if danger { t.red } else { t.accent })
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(what.as_str()),
    ];
    if let Some(w) = warning {
        lines.push(Line::default());
        lines.push(Line::styled(w, Style::new().fg(t.yellow)));
    }
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(
            " y ",
            Style::new().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("confirm   "),
        Span::styled(" any other key ", Style::new().fg(t.muted)),
        Span::raw("cancel"),
    ]));

    let area = centered(f.area(), 60, lines.len() as u16 + 3);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(t, " Confirm "))
            .style(Style::new().fg(t.fg)),
        area,
    );
}
