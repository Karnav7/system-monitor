//! The Processes tab.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Row, Table};
use ratatui::Frame;

use super::{bytes, panel};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme.theme;
    let procs = &app.collector.snap.procs;
    let arrow = if app.sort.reverse { "↑" } else { "↓" };
    let mut title = vec![
        Span::raw(format!(" Processes {} ", app.rows.len())),
        Span::styled(
            format!("sort {} {arrow} ", app.sort.key.label()),
            Style::new().fg(t.muted),
        ),
    ];
    if app.tree {
        title.push(Span::styled("tree ", Style::new().fg(t.muted)));
    }
    if !app.filter.is_empty() {
        title.push(Span::styled(
            format!("filter \"{}\" ", app.filter),
            Style::new().fg(t.yellow),
        ));
    }

    let rows: Vec<Row> = app
        .rows
        .iter()
        .map(|r| {
            let p = &procs[r.idx];
            let name = if app.tree && r.depth > 0 {
                format!("{}└ {}", "  ".repeat(r.depth - 1), p.name)
            } else {
                p.name.clone()
            };
            let cpu_style = Style::new().fg(if p.cpu >= 50.0 {
                t.red
            } else if p.cpu >= 10.0 {
                t.yellow
            } else {
                t.fg
            });
            Row::new(vec![
                Line::from(p.pid.to_string()).right_aligned(),
                Line::from(p.user.clone()),
                Line::styled(format!("{:.1}", p.cpu), cpu_style).right_aligned(),
                Line::from(bytes(p.mem)).right_aligned(),
                Line::from(p.threads.to_string()).right_aligned(),
                Line::from(p.state.to_string()),
                Line::styled(name, Style::new().fg(t.accent)),
                Line::styled(p.cmd.clone(), Style::new().fg(t.muted)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Length(10),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(if app.tree { 32 } else { 22 }),
        Constraint::Min(10),
    ];
    let header = Row::new(vec![
        Line::from("PID").right_aligned(),
        Line::from("USER"),
        Line::from("CPU%").right_aligned(),
        Line::from("MEM").right_aligned(),
        Line::from("THR").right_aligned(),
        Line::from("S"),
        Line::from("NAME"),
        Line::from("COMMAND"),
    ])
    .style(Style::new().fg(t.muted).add_modifier(Modifier::BOLD));

    let table = Table::new(rows, widths)
        .header(header)
        .block(panel(&t, Line::from(title)))
        .style(Style::new().fg(t.fg))
        .row_highlight_style(
            Style::new()
                .bg(t.selection_bg)
                .fg(t.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌")
        .column_spacing(1);
    f.render_stateful_widget(table, area, &mut app.proc_state);
}
