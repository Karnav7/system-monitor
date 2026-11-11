//! The Overview tab: CPU, memory, GPU, temperatures, network and disks.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph, Row, Sparkline, Table};
use ratatui::Frame;

use super::{bytes, empty_message, panel, rate};
use crate::app::App;
use crate::collect::history::History;
use crate::theme::Theme;

/// Width of one core cell in the per-core grid: "31 ▕██████▏100%" plus a gap.
const CORE_CELL: u16 = 18;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let s = &app.collector.snap;
    let core_cols = ((area.width.saturating_sub(2) * 3 / 5) / CORE_CELL).max(1) as usize;
    let core_rows = s.cores.len().div_ceil(core_cols) as u16;
    let cpu_h = (core_rows + 2).clamp(6, 14);

    let [cpu, mid, bottom] = Layout::vertical([
        Constraint::Length(cpu_h),
        Constraint::Length(8),
        Constraint::Min(6),
    ])
    .areas(area);
    draw_cpu(f, app, cpu, core_cols);

    let [mem, gpu, temps] = Layout::horizontal([Constraint::Ratio(1, 3); 3]).areas(mid);
    draw_mem(f, app, mem);
    draw_gpu(f, app, gpu);
    draw_temps(f, app, temps);

    let [net, disks] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(bottom);
    draw_net(f, app, net);
    draw_disks(f, app, disks);
}

fn spark<'a>(
    h: &History,
    width: u16,
    color: ratatui::style::Color,
    max: Option<u64>,
) -> Sparkline<'a> {
    let data = h.tail(width as usize);
    let s = Sparkline::default()
        .data(&data)
        .style(Style::new().fg(color));
    match max {
        Some(m) => s.max(m),
        None => s.max(data.iter().copied().max().unwrap_or(1).max(1)),
    }
}

fn gauge<'a>(t: &Theme, label: String, ratio: f64) -> LineGauge<'a> {
    let ratio = ratio.clamp(0.0, 1.0);
    LineGauge::default()
        .label(label)
        .ratio(ratio)
        .line_set(symbols::line::THICK)
        .filled_style(Style::new().fg(t.load_color(ratio * 100.0)))
        .unfilled_style(Style::new().fg(t.muted))
}

fn draw_cpu(f: &mut Frame, app: &App, area: Rect, core_cols: usize) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let title = format!(" CPU {:.0}%  {} ", s.cpu_total, s.cpu_brand);
    let block = panel(t, title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(inner);
    f.render_widget(
        spark(&app.collector.hist.cpu, left.width, t.accent, Some(100)),
        left,
    );

    let bar_w = (CORE_CELL - 9) as usize;
    let lines: Vec<Line> = s
        .cores
        .chunks(core_cols)
        .enumerate()
        .map(|(row, chunk)| {
            let mut spans = Vec::new();
            for (col, &u) in chunk.iter().enumerate() {
                let i = row * core_cols + col;
                let filled = ((u as f64 / 100.0) * bar_w as f64).round() as usize;
                spans.push(Span::styled(format!("{i:>2} "), Style::new().fg(t.muted)));
                spans.push(Span::styled(
                    "█".repeat(filled.min(bar_w)),
                    Style::new().fg(t.load_color(u as f64)),
                ));
                spans.push(Span::styled(
                    "░".repeat(bar_w - filled.min(bar_w)),
                    Style::new().fg(t.muted),
                ));
                spans.push(Span::raw(format!("{u:>4.0}% ")));
            }
            Line::from(spans)
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines),
        right.inner(ratatui::layout::Margin::new(1, 0)),
    );
}

fn draw_mem(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let block = panel(t, " Memory ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let ratio = |used: u64, total: u64| {
        if total == 0 {
            0.0
        } else {
            used as f64 / total as f64
        }
    };
    let [m, sw, sp] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(inner);
    f.render_widget(
        gauge(
            t,
            format!("RAM  {:>6}/{:<6}", bytes(s.mem_used), bytes(s.mem_total)),
            ratio(s.mem_used, s.mem_total),
        ),
        m,
    );
    let swap_label = if s.swap_total == 0 {
        "Swap  none        ".to_string()
    } else {
        format!("Swap {:>6}/{:<6}", bytes(s.swap_used), bytes(s.swap_total))
    };
    f.render_widget(gauge(t, swap_label, ratio(s.swap_used, s.swap_total)), sw);
    f.render_widget(
        spark(&app.collector.hist.mem, sp.width, t.magenta, Some(100)),
        sp,
    );
}

fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let Some(g) = s.gpus.first() else {
        let block = panel(t, " GPU ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        return f.render_widget(
            empty_message(t, "No amdgpu device found in /sys/class/drm"),
            inner,
        );
    };
    let block = panel(t, format!(" GPU {} ", g.card));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [b, v, sp] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(inner);
    let busy = g.busy.unwrap_or(0) as f64;
    f.render_widget(
        gauge(t, format!("Busy {busy:>5.0}%        "), busy / 100.0),
        b,
    );
    match (g.vram_used, g.vram_total) {
        (Some(u), Some(tot)) if tot > 0 => f.render_widget(
            gauge(
                t,
                format!("VRAM {:>6}/{:<6}", bytes(u), bytes(tot)),
                u as f64 / tot as f64,
            ),
            v,
        ),
        _ => f.render_widget(empty_message(t, "VRAM unknown"), v),
    }
    f.render_widget(
        spark(&app.collector.hist.gpu, sp.width, t.cyan, Some(100)),
        sp,
    );
}

fn draw_temps(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let block = panel(t, " Temperatures ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if s.temps.is_empty() {
        return f.render_widget(
            empty_message(t, "No sensors found (is lm_sensors / hwmon available?)"),
            inner,
        );
    }
    let label_w = inner.width.saturating_sub(8) as usize;
    // Two columns when there are more sensors than lines.
    let cols = if s.temps.len() > inner.height as usize && inner.width >= 44 {
        2
    } else {
        1
    };
    let areas = Layout::horizontal(vec![Constraint::Ratio(1, cols as u32); cols]).split(inner);
    let per_col = (inner.height as usize).max(1);
    let col_label_w = if cols == 2 {
        (inner.width / 2).saturating_sub(9) as usize
    } else {
        label_w
    };
    for (c, area) in areas.iter().enumerate() {
        let lines: Vec<Line> = s
            .temps
            .iter()
            .skip(c * per_col)
            .take(per_col)
            .map(|tp| {
                let color = if tp.celsius >= 85.0 {
                    t.red
                } else if tp.celsius >= 65.0 {
                    t.yellow
                } else {
                    t.green
                };
                let mut label = tp.label.clone();
                label.truncate(col_label_w);
                Line::from(vec![
                    Span::raw(format!("{label:<col_label_w$} ")),
                    Span::styled(format!("{:>5.1}°C", tp.celsius), Style::new().fg(color)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), *area);
    }
}

fn draw_net(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let h = &app.collector.hist;
    let block = panel(t, " Network ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let graph_h = inner.height.saturating_sub(2 + s.nets.len().min(6) as u16) / 2;
    let [rx_l, rx, tx_l, tx, list] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(graph_h.max(1)),
        Constraint::Length(1),
        Constraint::Length(graph_h.max(1)),
        Constraint::Min(0),
    ])
    .areas(inner);
    let label = |arrow: &str, now: u64, peak: u64, color| {
        Line::from(vec![
            Span::styled(
                format!("{arrow} {:<10}", rate(now as f64)),
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" peak {}", rate(peak as f64)),
                Style::new().fg(t.muted),
            ),
        ])
    };
    let peak = |hist: &History| hist.tail(usize::MAX).into_iter().max().unwrap_or(0);
    f.render_widget(label("↓", h.net_rx.last(), peak(&h.net_rx), t.green), rx_l);
    f.render_widget(spark(&h.net_rx, rx.width, t.green, None), rx);
    f.render_widget(label("↑", h.net_tx.last(), peak(&h.net_tx), t.blue), tx_l);
    f.render_widget(spark(&h.net_tx, tx.width, t.blue, None), tx);

    let rows: Vec<Row> = s
        .nets
        .iter()
        .map(|n| {
            Row::new(vec![
                n.name.clone(),
                format!("↓{}", rate(n.rx_rate)),
                format!("↑{}", rate(n.tx_rate)),
                format!("{} / {}", bytes(n.rx_total), bytes(n.tx_total)),
            ])
        })
        .collect();
    let widths = [
        Constraint::Min(8),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(15),
    ];
    f.render_widget(Table::new(rows, widths).style(Style::new().fg(t.fg)), list);
}

fn draw_disks(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme.theme;
    let s = &app.collector.snap;
    let h = &app.collector.hist;
    let title = Line::from(vec![
        Span::raw(" Disks "),
        Span::styled(
            format!(
                "read {}  write {} ",
                rate(h.disk_read.last() as f64),
                rate(h.disk_write.last() as f64)
            ),
            Style::new().fg(t.muted).add_modifier(Modifier::empty()),
        ),
    ]);
    let block = panel(t, title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if s.disks.is_empty() {
        return f.render_widget(empty_message(t, "No disks found"), inner);
    }

    let bar_w = 10usize;
    let rows: Vec<Row> = s
        .disks
        .iter()
        .map(|d| {
            let used = d.total.saturating_sub(d.available);
            let pct = if d.total == 0 {
                0.0
            } else {
                used as f64 * 100.0 / d.total as f64
            };
            let filled = ((pct / 100.0) * bar_w as f64).round() as usize;
            let bar = Line::from(vec![
                Span::styled(
                    "█".repeat(filled.min(bar_w)),
                    Style::new().fg(t.load_color(pct)),
                ),
                Span::styled(
                    "░".repeat(bar_w - filled.min(bar_w)),
                    Style::new().fg(t.muted),
                ),
                Span::raw(format!(" {pct:>3.0}%")),
            ]);
            Row::new(vec![
                Line::from(d.mount.clone()),
                Line::from(d.device.trim_start_matches("/dev/").to_string()),
                bar,
                Line::from(format!("{}/{}", bytes(used), bytes(d.total))),
                Line::from(rate(d.read_rate)),
                Line::from(rate(d.write_rate)),
            ])
        })
        .collect();
    let widths = [
        Constraint::Min(8),
        Constraint::Length(12),
        Constraint::Length(bar_w as u16 + 5),
        Constraint::Length(13),
        Constraint::Length(9),
        Constraint::Length(9),
    ];
    let header = Row::new(["MOUNT", "DEVICE", "USED", "SIZE", "READ", "WRITE"])
        .style(Style::new().fg(t.muted));
    f.render_widget(
        Table::new(rows, widths)
            .header(header)
            .style(Style::new().fg(t.fg)),
        inner,
    );
}
