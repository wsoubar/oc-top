//! Disks panel: mount usage meters and aggregate I/O speed.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::util;
use crate::widgets::{Meter, panel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 12 {
        return;
    }

    let theme = &app.theme;
    let disks = &app.snap.disks;
    let block = panel("disks", None, theme.mem_box, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if disks.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no disks found",
                Style::default().fg(theme.inactive_fg),
            ))),
            inner,
        );
        return;
    }

    let read_bps: f64 = disks.iter().map(|disk| disk.read_bps).sum();
    let write_bps: f64 = disks.iter().map(|disk| disk.write_bps).sum();
    let show_io = inner.height as usize > disks.len() && inner.width >= 26;
    let max_rows = if show_io {
        inner.height.saturating_sub(1) as usize
    } else {
        inner.height as usize
    };
    let show_values = inner.width >= 34;

    for (index, disk) in disks.iter().take(max_rows).enumerate() {
        let y = inner.y + index as u16;
        let value_text = show_values.then(|| {
            format!(
                "{}/{}",
                util::fmt_bytes_short(disk.used),
                util::fmt_bytes_short(disk.total)
            )
        });
        let label = util::truncate(&disk.mount, 11);
        let meter = Meter {
            label: &label,
            label_width: 11,
            value: disk.used_pct(),
            value_text,
            gradient: theme.used,
            theme,
            show_pct: true,
        };
        frame.render_widget(meter, Rect::new(inner.x, y, inner.width, 1));
    }

    if disks.len() > max_rows && max_rows > 0 {
        let y = inner.y + max_rows as u16 - 1;
        let text = format!("+{} more", disks.len() - max_rows + 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                text,
                Style::default().fg(theme.inactive_fg),
            ))),
            Rect::new(inner.x + 1, y, inner.width.saturating_sub(1), 1),
        );
    }

    if show_io {
        let y = inner.y + inner.height - 1;
        let line = Line::from(vec![
            Span::styled("↓ ", Style::default().fg(theme.download.at(0.6))),
            Span::styled(
                util::fmt_speed(read_bps),
                Style::default().fg(theme.inactive_fg),
            ),
            Span::styled("  ↑ ", Style::default().fg(theme.upload.at(0.6))),
            Span::styled(
                util::fmt_speed(write_bps),
                Style::default().fg(theme.inactive_fg),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
    }
}
