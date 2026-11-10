//! Memory and swap panel.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::Gradient;
use crate::util;
use crate::widgets::{Meter, panel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 12 {
        return;
    }

    let theme = &app.theme;
    let mem = &app.snap.mem;
    let swap = &app.snap.swap;

    let block = panel("mem", Some(2), theme.mem_box, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut spans = vec![
        Span::styled("Total: ", Style::default().fg(theme.inactive_fg)),
        Span::styled(
            util::fmt_bytes(mem.total),
            Style::default()
                .fg(theme.main_fg)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if swap.total > 0 && inner.width >= 42 {
        spans.push(Span::styled(
            "   Swap: ",
            Style::default().fg(theme.inactive_fg),
        ));
        spans.push(Span::styled(
            util::fmt_bytes(swap.total),
            Style::default().fg(theme.main_fg),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    if inner.height < 2 {
        return;
    }

    let show_values = inner.width >= 34;
    let metrics: [(&str, f32, u64, Gradient); 4] = [
        ("Used", mem.used_pct(), mem.used, theme.used),
        (
            "Available",
            mem.available_pct(),
            mem.available,
            theme.available,
        ),
        ("Cached", mem.cached_pct(), mem.cached, theme.cached),
        ("Free", mem.free_pct(), mem.free, theme.free),
    ];

    let mut y = inner.y + 1;
    let bottom = inner.y + inner.height;

    for (label, percent, value, gradient) in metrics {
        if y >= bottom {
            break;
        }
        let meter = Meter {
            label,
            label_width: 9,
            value: percent,
            value_text: show_values.then(|| util::fmt_bytes(value)),
            gradient,
            theme,
            show_pct: true,
        };
        frame.render_widget(meter, Rect::new(inner.x, y, inner.width, 1));
        y += 1;
    }

    if y < bottom && swap.total > 0 {
        let meter = Meter {
            label: "Swap",
            label_width: 9,
            value: swap.used_pct(),
            value_text: show_values.then(|| {
                format!(
                    "{} / {}",
                    util::fmt_bytes(swap.used),
                    util::fmt_bytes(swap.total)
                )
            }),
            gradient: theme.used,
            theme,
            show_pct: true,
        };
        frame.render_widget(meter, Rect::new(inner.x, y, inner.width, 1));
    }
}
