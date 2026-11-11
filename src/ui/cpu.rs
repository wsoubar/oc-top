//! CPU panel: usage history graph and per-core meters.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, CORE_HISTORY_LEN};
use crate::config::GraphSymbol;
use crate::util;
use crate::widgets::{Graph, panel};

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
        let columns =
            Layout::horizontal([Constraint::Percentage(57), Constraint::Fill(1)]).split(body);
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

    frame.render_widget(
        Paragraph::new("·".repeat(usize::from(area.width)))
            .style(Style::default().fg(app.theme.meter_bg)),
        Rect::new(area.x, area.bottom() - 1, area.width, 1),
    );
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

    let label_width = (cores.len() - 1).to_string().len() + 2;
    let min_column_width = label_width + 10;
    let columns = cores
        .len()
        .div_ceil(4)
        .max(cores.len().div_ceil(height))
        .min((usize::from(area.width) / min_column_width).max(1));
    let column_width = usize::from(area.width) / columns;
    let mut rows = cores.len().div_ceil(columns).min(height);
    let overflow = cores.len() > rows * columns && rows > 1;
    if overflow {
        rows -= 1;
    }

    let theme = &app.theme;
    let show_frequency = column_width >= label_width + 22;
    let frequency_width = if show_frequency { 7 } else { 0 };
    let chart_width = column_width
        .saturating_sub(label_width + 6 + frequency_width)
        .min(CORE_HISTORY_LEN);

    let symbols = match app.config.graph_symbol {
        GraphSymbol::Braille => ["⣀", "⣤", "⣶", "⣿"],
        GraphSymbol::Block => ["▂", "▄", "▆", "█"],
    };

    for (index, usage) in cores.iter().take(rows * columns).copied().enumerate() {
        let label = format!("C{index}");
        let mut spans = vec![Span::styled(
            format!("{label:<label_width$}"),
            Style::default().fg(theme.main_fg),
        )];
        let history_start = app.core_history.len().saturating_sub(chart_width);
        let samples = app.core_history.iter().skip(history_start);
        let padding = chart_width - samples.len();
        spans.push(Span::styled(
            "·".repeat(padding),
            Style::default().fg(theme.meter_bg),
        ));
        for sample in samples {
            let value = sample.get(index).copied().unwrap_or_default() / 100.0;
            let value = value.clamp(0.0, 1.0);
            let (symbol, color) = if value > 0.0 {
                let level = ((value * 4.0).ceil() as usize).saturating_sub(1);
                (symbols[level], theme.cpu.at(value))
            } else {
                ("·", theme.meter_bg)
            };
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }

        let usage = usage.clamp(0.0, 100.0);
        spans.push(Span::styled(
            format!(" {usage:>3.0}%"),
            Style::default().fg(theme.cpu.at(usage / 100.0)),
        ));
        if show_frequency {
            let frequency = app.snap.freqs.get(index).copied().unwrap_or_default();
            if frequency > 0 {
                let ghz = frequency as f32 / 1000.0;
                spans.push(Span::styled(
                    format!(" {ghz:>4.1}G"),
                    Style::default().fg(theme.inactive_fg),
                ));
            }
        }

        let x = area.x + (index / rows * column_width) as u16;
        let y = area.y + (index % rows) as u16;
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(x, y, column_width as u16, 1),
        );
    }

    if overflow {
        let hidden = cores.len() - rows * columns;
        frame.render_widget(
            Paragraph::new(format!("+{hidden} cores"))
                .style(Style::default().fg(theme.inactive_fg)),
            Rect::new(area.x, area.y + rows as u16, area.width, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    #[test]
    fn aligns_per_core_history_and_percentages_without_background_blocks() {
        let mut app = App::new().expect("app");
        app.config.graph_symbol = GraphSymbol::Braille;
        app.snap.cores = [0.0, 4.0, 24.0, 100.0].repeat(3);
        app.core_history = [app.snap.cores.clone()].into();

        let mut terminal = Terminal::new(TestBackend::new(58, 6)).expect("terminal");
        terminal
            .draw(|frame| render_cores(frame, frame.area(), &app))
            .expect("draw");
        let buffer = terminal.backend().buffer();

        for x in [0, 19, 38] {
            assert_eq!(buffer[(x, 0)].symbol(), "C");
            assert_eq!(buffer[(x + 12, 1)].symbol(), "⣀");
            assert_eq!(buffer[(x + 12, 3)].symbol(), "⣿");
            for y in 0..4 {
                assert_eq!(buffer[(x + 17, y)].symbol(), "%");
                assert_eq!(buffer[(x + 12, y)].bg, Color::Reset);
            }
        }
    }
}
