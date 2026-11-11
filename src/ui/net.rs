//! Network panel with download/upload history graphs.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::util;
use crate::widgets::{Graph, panel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 12 {
        return;
    }

    let theme = &app.theme;
    let interface = util::truncate(
        &interface_label(app),
        usize::from(area.width.saturating_sub(10)),
    );
    let block = panel("net", Some(3), theme.net_box, theme).title_top(
        Line::from(Span::styled(
            format!("{interface} "),
            Style::default().fg(theme.graph_text),
        ))
        .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if inner.height == 1 {
        let line = Line::from(vec![
            Span::styled(
                "▼ ",
                Style::default()
                    .fg(theme.download.at(0.6))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                util::fmt_speed(app.snap.net_rx_bps),
                Style::default().fg(theme.main_fg),
            ),
            Span::styled("   ▲ ", Style::default().fg(theme.upload.at(0.6))),
            Span::styled(
                util::fmt_speed(app.snap.net_tx_bps),
                Style::default().fg(theme.main_fg),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), inner);
        return;
    }

    let halves =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);
    render_direction(frame, halves[0], app, false);
    render_direction(frame, halves[1], app, true);
}

fn render_direction(frame: &mut Frame, area: Rect, app: &App, upload: bool) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let theme = &app.theme;
    let (speed, top, total, gradient, arrow, name) = if upload {
        (
            app.snap.net_tx_bps,
            app.top_tx,
            app.snap.net_total_tx,
            theme.upload,
            "▲",
            "upload",
        )
    } else {
        (
            app.snap.net_rx_bps,
            app.top_rx,
            app.snap.net_total_rx,
            theme.download,
            "▼",
            "download",
        )
    };

    let mut spans = vec![
        Span::styled(
            format!("{arrow} "),
            Style::default()
                .fg(gradient.at(0.6))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if area.width >= 34 {
                format!("{name} ")
            } else {
                String::new()
            },
            Style::default().fg(theme.inactive_fg),
        ),
        Span::styled(
            util::fmt_speed(speed),
            Style::default()
                .fg(theme.main_fg)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if area.width >= 34 {
        spans.push(Span::styled(
            format!("  top {}", util::fmt_speed(top)),
            Style::default().fg(theme.inactive_fg),
        ));
    }
    if area.width >= 52 {
        spans.push(Span::styled(
            "  total ",
            Style::default().fg(theme.inactive_fg),
        ));
        spans.push(Span::styled(
            util::fmt_bytes(total),
            Style::default().fg(theme.inactive_fg),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if area.height < 2 {
        return;
    }

    let graph_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let data = if upload { &app.up_hist } else { &app.down_hist };
    let graph = Graph {
        data,
        max: 0.0,
        gradient,
        symbol: app.config.graph_symbol,
        theme,
        label: None,
    };
    frame.render_widget(graph, graph_area);
}

fn interface_label(app: &App) -> String {
    match app.snap.nets.as_slice() {
        [] => "no interface".to_string(),
        [single] => single.name.clone(),
        nets => {
            let mut label = nets
                .iter()
                .take(3)
                .map(|net| net.name.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if nets.len() > 3 {
                label.push_str(&format!(" +{}", nets.len() - 3));
            }
            label
        }
    }
}
