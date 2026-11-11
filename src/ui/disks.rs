//! Disks panel: mount usage meters and aggregate I/O speed.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::util;
use crate::widgets::{MeterBar, panel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 12 {
        return;
    }

    let theme = &app.theme;
    let disks = &app.snap.disks;
    let block = panel("disks", Some(4), theme.mem_box, theme);
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
    let show_sizes = inner.width >= 34;
    let mount_width = 12.min(inner.width / 2);
    let columns = Layout::horizontal([
        Constraint::Length(mount_width),
        Constraint::Length(if show_sizes { 14 } else { 0 }),
        Constraint::Fill(1),
        Constraint::Length(5),
    ])
    .split(inner);

    let show_overflow = disks.len() > max_rows && max_rows > 1;
    let meter_rows = max_rows - usize::from(show_overflow);

    for (index, disk) in disks.iter().take(meter_rows).enumerate() {
        let y = inner.y + index as u16;
        let row = |column: usize| Rect {
            y,
            height: 1,
            ..columns[column]
        };
        let label = util::truncate(&disk.mount, usize::from(mount_width - 1));
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(theme.inactive_fg)),
            row(0),
        );
        if show_sizes {
            let size = format!(
                "{}/{}",
                util::fmt_bytes_short(disk.used),
                util::fmt_bytes_short(disk.total)
            );
            frame.render_widget(
                Paragraph::new(size).style(Style::default().fg(theme.main_fg)),
                row(1),
            );
        }
        let percent = disk.used_pct();
        frame.render_widget(
            MeterBar {
                value: percent,
                gradient: theme.used,
                background: theme.meter_bg,
            },
            row(2),
        );
        frame.render_widget(
            Paragraph::new(format!("{percent:.0}%"))
                .right_aligned()
                .style(Style::default().fg(theme.main_fg)),
            row(3),
        );
    }

    if show_overflow {
        let y = inner.y + max_rows as u16 - 1;
        let text = format!("+{} more", disks.len() - meter_rows);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::DiskInfo;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn aligns_disk_bars_for_different_sizes_and_percentages() {
        let mut app = App::new().expect("app");
        app.snap.disks = vec![
            DiskInfo {
                mount: "/".into(),
                used: 1,
                total: 100,
                ..DiskInfo::default()
            },
            DiskInfo {
                mount: "/boot/efi".into(),
                used: 512,
                total: 1024,
                ..DiskInfo::default()
            },
            DiskInfo {
                mount: "/home".into(),
                used: 1024,
                total: 1024,
                ..DiskInfo::default()
            },
        ];

        for (width, bar_start) in [(52, 27), (22, 11)] {
            let mut terminal = Terminal::new(TestBackend::new(width, 5)).expect("terminal");
            terminal
                .draw(|frame| render(frame, frame.area(), &app))
                .expect("draw");
            let buffer = terminal.backend().buffer();
            let bar_end = width - 7;

            for y in 1..=3 {
                assert_eq!(buffer[(1, y)].symbol(), "/");
                assert_eq!(buffer[(width - 2, y)].symbol(), "%");
                for x in [bar_start, bar_end] {
                    assert_eq!(buffer[(x, y)].symbol(), " ");
                    assert_ne!(Some(buffer[(x, y)].bg), app.theme.main_bg);
                }
                for x in [bar_start - 1, bar_end + 1] {
                    assert_eq!(Some(buffer[(x, y)].bg), app.theme.main_bg);
                }
            }

            if width >= 34 {
                assert_eq!(buffer[(13, 1)].symbol(), "1");
                assert_eq!(buffer[(13, 2)].symbol(), "5");
                assert_eq!(buffer[(13, 3)].symbol(), "1");
            }
        }
    }
}
