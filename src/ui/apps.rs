//! The Apps tab: Hyprland windows grouped by app.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Row, Table};
use ratatui::Frame;

use super::{bytes, empty_message, panel};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme.theme;
    let detail_h = app
        .selected_group()
        .map(|g| g.windows.len() as u16 + 2)
        .unwrap_or(3)
        .clamp(3, 10);
    let [list, detail] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(detail_h)]).areas(area);

    let block = panel(&t, format!(" Apps {} ", app.groups.len()));
    if app.groups.is_empty() {
        let msg = app
            .collector
            .snap
            .hypr_error
            .clone()
            .unwrap_or_else(|| "No open windows".into());
        let inner = block.inner(list);
        f.render_widget(block, list);
        f.render_widget(empty_message(&t, &msg).wrap(Default::default()), inner);
        return;
    }

    let rows: Vec<Row> = app
        .groups
        .iter()
        .map(|g| {
            Row::new(vec![
                Line::styled(g.class.clone(), Style::new().fg(t.accent)),
                Line::from(g.windows.len().to_string()).right_aligned(),
                Line::from(g.pids.len().to_string()).right_aligned(),
                Line::from(format!("{:.1}", g.cpu)).right_aligned(),
                Line::from(bytes(g.mem)).right_aligned(),
                Line::styled(
                    g.windows
                        .first()
                        .map(|w| w.title.clone())
                        .unwrap_or_default(),
                    Style::new().fg(t.muted),
                ),
            ])
        })
        .collect();
    let widths = [
        Constraint::Length(24),
        Constraint::Length(4),
        Constraint::Length(5),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Min(10),
    ];
    let header = Row::new(vec![
        Line::from("APP"),
        Line::from("WIN").right_aligned(),
        Line::from("PROC").right_aligned(),
        Line::from("CPU%").right_aligned(),
        Line::from("MEM").right_aligned(),
        Line::from("TITLE"),
    ])
    .style(Style::new().fg(t.muted).add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .style(Style::new().fg(t.fg))
        .row_highlight_style(
            Style::new()
                .bg(t.selection_bg)
                .fg(t.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌");
    f.render_stateful_widget(table, list, &mut app.app_state);

    if let Some(g) = app.selected_group() {
        let block = panel(&t, format!(" {} windows ", g.class));
        let lines: Vec<Line> = g
            .windows
            .iter()
            .map(|w| {
                Line::from(vec![
                    Span::styled(
                        format!("ws {:<4} pid {:<8} ", w.workspace, w.pid),
                        Style::new().fg(t.muted),
                    ),
                    Span::raw(w.title.clone()),
                ])
            })
            .collect();
        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .style(Style::new().fg(t.fg)),
            detail,
        );
    }
}
