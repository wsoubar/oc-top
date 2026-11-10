//! CPU panel: usage history graph and per-core meters.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::util;
use crate::widgets::{Graph, Meter, panel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 10 {
        return;
    }

    let theme = &app.theme;
    let snap = &app.snap;

    let block = panel("cpu", Some(1), theme.cpu_box, theme).title_top(
        Line::from(Span::styled(
            format!("{}ms ", app.config.update_ms),
            Style::default().fg(theme.graph_text),
        ))
        .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut spans = vec![
        Span::styled("usage ", Style::default().fg(theme.inactive_fg)),
        Span::styled(
            format!("{:.0}%", snap.cpu_usage),
            Style::default()
                .fg(theme.main_fg)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let max_freq = snap.freqs.iter().copied().max().unwrap_or(0);
    if max_freq > 0 {
        spans.push(Span::styled(
            "  freq ",
            Style::default().fg(theme.inactive_fg),
        ));
        spans.push(Span::styled(
            format!("{:.2}GHz", max_freq as f32 / 1000.0),
            Style::default().fg(theme.main_fg),
        ));
    }

    if let Some(temp) = snap.cpu_temp {
        spans.push(Span::styled(
            "  temp ",
            Style::default().fg(theme.inactive_fg),
        ));
        spans.push(Span::styled(
            format!("{temp:.0}°C"),
            Style::default().fg(theme.temp.at((temp / 100.0).clamp(0.0, 1.0))),
        ));
    }

    spans.push(Span::styled(
        "  load ",
        Style::default().fg(theme.inactive_fg),
    ));
    spans.push(Span::styled(
        format!("{:.2} {:.2} {:.2}", snap.load.0, snap.load.1, snap.load.2),
        Style::default().fg(theme.main_fg),
    ));

    spans.push(Span::styled(
        format!("  {} cores", snap.core_count()),
        Style::default().fg(theme.inactive_fg),
    ));

    if inner.width >= 96 {
        spans.push(Span::styled(
            format!("  {}", util::truncate(&snap.cpu_brand, 34)),
            Style::default().fg(theme.inactive_fg),
        ));
    }

    let stats_area = Rect::new(inner.x, inner.y, inner.width, 1);
    frame.render_widget(Paragraph::new(Line::from(spans)), stats_area);

    if inner.height < 2 {
        return;
    }

    let body = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
    let show_cores = body.width >= 52 && !snap.cores.is_empty();

    if show_cores {
        let columns = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(body);
        render_graph(frame, columns[0], app);
        render_cores(frame, columns[1], app);
    } else {
        render_graph(frame, body, app);
    }
}

fn render_graph(frame: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let graph = Graph {
        data: &app.cpu_hist,
        max: 100.0,
        gradient: app.theme.cpu,
        symbol: app.config.graph_symbol,
        theme: &app.theme,
        label: Some("100%".to_string()),
    };
    frame.render_widget(graph, area);
}

fn render_cores(frame: &mut Frame, area: Rect, app: &App) {
    let cores = &app.snap.cores;
    let height = area.height as usize;
    if cores.is_empty() || height == 0 || area.width == 0 {
        return;
    }

    let columns = if cores.len() > height && area.width >= 42 {
        2
    } else {
        1
    };
    let mut per_column = cores.len().div_ceil(columns);
    let mut hidden = 0;
    if per_column > height {
        per_column = height.saturating_sub(1).max(1);
        hidden = cores.len().saturating_sub(per_column * columns);
    }
    let column_width = area.width / columns as u16;
    let label_width = if cores.len() >= 10 { 4 } else { 3 };
    let theme = &app.theme;

    for column in 0..columns {
        for row in 0..per_column {
            let index = column * per_column + row;
            if index >= cores.len() {
                break;
            }

            let y = area.y + row as u16;
            let x = area.x + column as u16 * column_width;
            let width = column_width.saturating_sub(1);
            if width < 8 {
                continue;
            }

            let frequency = app.snap.freqs.get(index).copied().unwrap_or(0);
            let value_text = if width >= 26 && frequency > 0 {
                Some(format!("{:.1}G", frequency as f32 / 1000.0))
            } else {
                None
            };

            let label = format!("C{index}");
            let meter = Meter {
                label: &label,
                label_width,
                value: cores[index],
                value_text,
                gradient: theme.cpu,
                theme,
                show_pct: true,
            };
            frame.render_widget(meter, Rect::new(x, y, width, 1));
        }
    }

    if hidden > 0 && per_column < height {
        let text = format!("+{hidden} cores");
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                text,
                Style::default().fg(theme.inactive_fg),
            ))),
            Rect::new(area.x, area.y + per_column as u16, area.width, 1),
        );
    }
}
