//! Terminal layout, header/footer, overlays and panel rendering.

mod cpu;
mod disks;
mod mem;
mod net;
mod procs;

use std::time::Duration;

use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::{App, Overlay};
use crate::util;
use crate::widgets::panel;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if let Some(background) = app.theme.main_bg {
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            area,
        );
    }

    if area.width < 60 || area.height < 18 {
        let message = Paragraph::new(vec![
            Line::from(Span::styled(
                "rtop",
                Style::default()
                    .fg(app.theme.title)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("terminal too small"),
            Line::from(format!("{}x{} (minimum 60x18)", area.width, area.height)),
        ])
        .alignment(Alignment::Center)
        .block(panel("rtop", None, app.theme.proc_box, &app.theme));
        frame.render_widget(message, area);
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    header(frame, rows[0], app);
    body(frame, rows[1], app);
    footer(frame, rows[2], app);
    overlays(frame, area, app);
}

fn body(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width >= 100 {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        let left = Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(columns[0]);
        let right = Layout::vertical([
            Constraint::Percentage(30),
            Constraint::Percentage(42),
            Constraint::Percentage(28),
        ])
        .split(columns[1]);

        cpu::render(frame, left[0], app);
        procs::render(frame, left[1], app);
        net::render(frame, right[0], app);
        mem::render(frame, right[1], app);
        disks::render(frame, right[2], app);
    } else if area.width >= 78 {
        let columns = Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(area);
        let left = Layout::vertical([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(columns[0]);
        let right = Layout::vertical([Constraint::Percentage(46), Constraint::Percentage(54)])
            .split(columns[1]);

        cpu::render(frame, left[0], app);
        procs::render(frame, left[1], app);
        net::render(frame, right[0], app);
        mem::render(frame, right[1], app);
    } else {
        let rows = Layout::vertical([
            Constraint::Percentage(28),
            Constraint::Percentage(20),
            Constraint::Percentage(24),
            Constraint::Percentage(28),
        ])
        .split(area);

        cpu::render(frame, rows[0], app);
        net::render(frame, rows[1], app);
        mem::render(frame, rows[2], app);
        procs::render(frame, rows[3], app);
    }
}

fn header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let now = Local::now();

    let left = Line::from(vec![
        Span::styled(
            " rtop",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.inactive_fg),
        ),
        Span::styled("│", Style::default().fg(theme.div_line)),
        Span::styled(
            format!(" {} ", now.format("%H:%M:%S")),
            Style::default().fg(theme.main_fg),
        ),
        Span::styled("│", Style::default().fg(theme.div_line)),
        Span::styled(
            format!(" up {} ", util::fmt_uptime(app.snap.uptime)),
            Style::default().fg(theme.inactive_fg),
        ),
    ]);
    frame.render_widget(Paragraph::new(left), area);

    if area.width >= 78 {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        let text = if area.width >= 108 {
            format!("{}@{}  {} ", user, app.snap.hostname, app.snap.kernel)
        } else {
            format!("{}@{} ", user, app.snap.hostname)
        };
        let right = Line::from(Span::styled(text, Style::default().fg(theme.inactive_fg)))
            .alignment(Alignment::Right);
        frame.render_widget(Paragraph::new(right), area);
    }
}

fn footer(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    if app.filter_edit {
        let line = Line::from(vec![
            Span::styled(
                " filter: ",
                Style::default()
                    .fg(theme.hi_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(app.filter.clone(), Style::default().fg(theme.main_fg)),
            Span::styled(
                "█",
                Style::default()
                    .fg(theme.hi_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  enter apply · esc cancel · delete clear",
                Style::default().fg(theme.inactive_fg),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    if let Some((message, at)) = &app.status
        && at.elapsed() < Duration::from_secs(4)
    {
        let line = Line::from(vec![
            Span::styled(
                format!(" {message} "),
                Style::default()
                    .fg(theme.hi_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ q quit", Style::default().fg(theme.inactive_fg)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let mut spans = Vec::new();
    if app.paused {
        spans.push(Span::styled(
            " PAUSED ",
            Style::default()
                .fg(theme.selected_fg)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(" │ ", Style::default().fg(theme.div_line)));
    }

    let hints: &[(&str, &str)] = if area.width >= 108 {
        HINTS_FULL
    } else if area.width >= 92 {
        HINTS_MEDIUM
    } else if area.width >= 68 {
        HINTS_COMPACT
    } else {
        HINTS_MINIMAL
    };
    for (index, (key, description)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme.div_line)));
        }
        spans.push(Span::styled(
            format!(" {key}"),
            Style::default()
                .fg(theme.hi_fg)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {description}"),
            Style::default().fg(theme.inactive_fg),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

const HINTS_FULL: &[(&str, &str)] = &[
    ("q", "quit"),
    ("m", "menu"),
    ("↑↓", "select"),
    ("f", "filter"),
    ("e", "tree"),
    ("←→", "sort"),
    ("t", "term"),
    ("k", "kill"),
    ("enter", "info"),
    ("+/-", "rate"),
];

const HINTS_MEDIUM: &[(&str, &str)] = &[
    ("q", "quit"),
    ("m", "menu"),
    ("↑↓", "select"),
    ("f", "filter"),
    ("e", "tree"),
    ("←→", "sort"),
    ("t", "term"),
    ("k", "kill"),
    ("enter", "info"),
];

const HINTS_COMPACT: &[(&str, &str)] = &[
    ("q", "quit"),
    ("m", "menu"),
    ("f", "filter"),
    ("t", "term"),
    ("k", "kill"),
    ("enter", "info"),
];

const HINTS_MINIMAL: &[(&str, &str)] =
    &[("q", "quit"), ("m", "menu"), ("f", "filter"), ("k", "kill")];

fn overlays(frame: &mut Frame, area: Rect, app: &App) {
    match app.overlay.clone() {
        Overlay::None => {}
        Overlay::Menu { selected } => menu(frame, area, app, selected),
        Overlay::Options { selected } => options(frame, area, app, selected),
        Overlay::Help => help(frame, area, app),
        Overlay::ConfirmKill { pid, name, signal } => {
            confirm_kill(frame, area, app, pid, &name, signal)
        }
        Overlay::Info { pid } => process_info(frame, area, app, pid),
    }
}

fn menu(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let theme = &app.theme;
    let rect = popup(area, 44, 10);
    frame.render_widget(Clear, rect);

    let items = ["Options", "Help", "Quit"];
    let mut lines = vec![Line::from("")];
    for (index, item) in items.iter().enumerate() {
        let style = if index == selected {
            Style::default()
                .fg(theme.selected_fg)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.main_fg)
        };
        lines.push(Line::from(Span::styled(
            format!("  {}  {item}", index + 1),
            style,
        )));
        lines.push(Line::from(""));
    }

    let block = panel("menu", None, theme.hi_fg, theme).title_bottom(Line::from(Span::styled(
        " esc close ",
        Style::default().fg(theme.inactive_fg),
    )));
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn options(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let theme = &app.theme;
    let rect = popup(area, 64, 11);
    frame.render_widget(Clear, rect);

    let sort_arrow = if app.config.reverse { "↑" } else { "↓" };
    let values = [
        (format!("{} ms", app.config.update_ms), "Update interval"),
        (app.config.graph_symbol.label().to_string(), "Graph symbols"),
        (
            format!("{} {sort_arrow}", app.config.sort_by.label()),
            "Sorting",
        ),
        (
            if app.config.reverse { "on" } else { "off" }.to_string(),
            "Reverse sort",
        ),
        (
            if app.config.tree { "on" } else { "off" }.to_string(),
            "Tree view",
        ),
        (app.theme.name.clone(), "Theme"),
    ];

    let mut lines = vec![Line::from("")];
    for (index, (value, label)) in values.iter().enumerate() {
        let style = if index == selected {
            Style::default()
                .fg(theme.selected_fg)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.main_fg)
        };
        let value_style = if index == selected {
            style
        } else {
            Style::default().fg(theme.hi_fg)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {label:<18}"), style),
            Span::styled(format!("{value:<20}"), value_style),
            Span::styled("◂ ▸", Style::default().fg(theme.inactive_fg)),
        ]));
    }
    lines.push(Line::from(""));

    let block = panel("options", None, theme.hi_fg, theme).title_bottom(Line::from(Span::styled(
        " ←→ change · enter cycle · esc back ",
        Style::default().fg(theme.inactive_fg),
    )));
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn help(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let rect = popup(area, 78, 24);
    frame.render_widget(Clear, rect);

    let mut lines: Vec<Line<'static>> = Vec::new();
    help_section(&mut lines, theme, "Global");
    help_entry(&mut lines, theme, "q", "quit rtop");
    help_entry(&mut lines, theme, "m / esc", "main menu");
    help_entry(&mut lines, theme, "h / ? / F1", "this help");
    help_entry(&mut lines, theme, "o / F2", "options");
    help_entry(&mut lines, theme, "+ / -", "change update interval");
    help_section(&mut lines, theme, "Process list");
    help_entry(&mut lines, theme, "↑ / ↓", "select process");
    help_entry(&mut lines, theme, "pgup / pgdn", "move by one page");
    help_entry(&mut lines, theme, "home / end", "first / last process");
    help_entry(&mut lines, theme, "enter", "show process details");
    help_entry(&mut lines, theme, "f / /", "filter processes");
    help_entry(&mut lines, theme, "e", "toggle tree view");
    help_entry(
        &mut lines,
        theme,
        "+ / -",
        "expand / collapse tree children",
    );
    help_entry(&mut lines, theme, "← / →", "cycle sorting column");
    help_entry(&mut lines, theme, "r", "reverse sorting");
    help_entry(&mut lines, theme, "u", "pause process list");
    help_entry(&mut lines, theme, "t / k", "send SIGTERM / SIGKILL");
    help_entry(&mut lines, theme, "delete", "clear filter");
    help_section(&mut lines, theme, "Mouse");
    help_entry(
        &mut lines,
        theme,
        "click / scroll",
        "select process in the list",
    );

    let block = panel("help", None, theme.hi_fg, theme).title_bottom(Line::from(Span::styled(
        " esc close ",
        Style::default().fg(theme.inactive_fg),
    )));
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn help_section(lines: &mut Vec<Line<'static>>, theme: &crate::theme::Theme, title: &str) {
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )));
}

fn help_entry(
    lines: &mut Vec<Line<'static>>,
    theme: &crate::theme::Theme,
    key: &str,
    description: &str,
) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {key:<16}"), Style::default().fg(theme.hi_fg)),
        Span::styled(description.to_string(), Style::default().fg(theme.main_fg)),
    ]));
}

fn confirm_kill(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    pid: u32,
    name: &str,
    signal: sysinfo::Signal,
) {
    let theme = &app.theme;
    let rect = popup(area, 64, 7);
    frame.render_widget(Clear, rect);

    let signal_name = match signal {
        sysinfo::Signal::Term => "SIGTERM",
        sysinfo::Signal::Kill => "SIGKILL",
        _ => "signal",
    };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Send ", Style::default().fg(theme.main_fg)),
            Span::styled(
                signal_name.to_string(),
                Style::default()
                    .fg(theme.hi_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to ", Style::default().fg(theme.main_fg)),
            Span::styled(
                format!("{name} ({pid})"),
                Style::default()
                    .fg(theme.title)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(theme.main_fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  y / enter confirm          n / esc cancel",
            Style::default().fg(theme.inactive_fg),
        )),
    ];

    let block = panel("signals", None, theme.hi_fg, theme);
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn process_info(frame: &mut Frame, area: Rect, app: &App, pid: u32) {
    let theme = &app.theme;
    let rect = popup(area, 78, 15);
    frame.render_widget(Clear, rect);

    let mut lines = vec![Line::from("")];
    if let Some(proc) = app.find_proc(pid) {
        info_row(&mut lines, theme, "Name", proc.name.clone());
        info_row(&mut lines, theme, "PID", proc.pid.to_string());
        info_row(
            &mut lines,
            theme,
            "Parent",
            proc.parent
                .map(|parent| parent.to_string())
                .unwrap_or_else(|| "-".into()),
        );
        info_row(&mut lines, theme, "User", proc.user.clone());
        info_row(&mut lines, theme, "Status", proc.status.clone());
        info_row(&mut lines, theme, "Threads", proc.threads.to_string());
        info_row(&mut lines, theme, "CPU", format!("{:.1}%", proc.cpu));
        info_row(&mut lines, theme, "Memory", util::fmt_bytes(proc.mem));
        info_row(
            &mut lines,
            theme,
            "Runtime",
            util::fmt_duration(proc.run_time),
        );
        info_row(&mut lines, theme, "Command", util::truncate(&proc.cmd, 60));
        info_row(
            &mut lines,
            theme,
            "Executable",
            util::truncate(&proc.exe, 60),
        );
    } else {
        lines.push(Line::from(Span::styled(
            "  process is no longer running",
            Style::default().fg(theme.inactive_fg),
        )));
    }

    let block =
        panel(&format!("process {pid}"), None, theme.hi_fg, theme).title_bottom(Line::from(
            Span::styled(" esc close ", Style::default().fg(theme.inactive_fg)),
        ));
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn info_row(
    lines: &mut Vec<Line<'static>>,
    theme: &crate::theme::Theme,
    label: &str,
    value: String,
) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {label:<12}"),
            Style::default().fg(theme.inactive_fg),
        ),
        Span::styled(value, Style::default().fg(theme.main_fg)),
    ]));
}

fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(10);
    let height = height.min(area.height.saturating_sub(2)).max(3);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Overlay;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn render_buffer(app: &mut App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal.draw(|frame| draw(frame, app)).expect("draw");
        terminal.backend().buffer().clone()
    }

    fn render(app: &mut App, width: u16, height: u16) -> String {
        let buffer = render_buffer(app, width, height);
        let mut output = String::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                line.push_str(buffer[(x, y)].symbol());
            }
            output.push_str(line.trim_end());
            output.push('\n');
        }
        output
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn paints_an_opaque_black_background() {
        let mut app = App::new().expect("app");
        let buffer = render_buffer(&mut app, 120, 36);
        let black = ratatui::style::Color::Rgb(0, 0, 0);

        for (x, y) in [(0, 0), (119, 0), (0, 35), (119, 35), (60, 18), (1, 10)] {
            assert_eq!(buffer[(x, y)].bg, black, "cell {x},{y} is not black");
        }
    }

    #[test]
    fn renders_main_view_without_panicking() {
        let mut app = App::new().expect("app");
        let output = render(&mut app, 120, 36);

        println!("{output}");

        assert!(output.contains("cpu"));
        assert!(output.contains("proc"));
        assert!(output.contains("mem"));
        assert!(output.contains("net"));
        assert!(output.contains("disks"));
    }

    #[test]
    fn renders_the_small_terminal_fallback() {
        let mut app = App::new().expect("app");
        let output = render(&mut app, 50, 15);
        assert!(output.contains("too small"));
    }

    #[test]
    fn help_and_options_overlays_render() {
        let mut app = App::new().expect("app");

        press(&mut app, KeyCode::Char('h'));
        assert!(matches!(app.overlay, Overlay::Help));
        let output = render(&mut app, 120, 36);
        assert!(output.contains("Global"));
        assert!(output.contains("SIGTERM"));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.overlay, Overlay::None));

        press(&mut app, KeyCode::Char('o'));
        assert!(matches!(app.overlay, Overlay::Options { .. }));
        let before = app.config.update_ms;
        press(&mut app, KeyCode::Right);
        assert!(app.config.update_ms > before);
        let output = render(&mut app, 120, 36);
        assert!(output.contains("Update interval"));
        assert!(output.contains("Theme"));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.overlay, Overlay::Menu { .. }));
    }

    #[test]
    fn filter_and_sort_work() {
        let mut app = App::new().expect("app");

        press(&mut app, KeyCode::Char('f'));
        for ch in "systemd".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        assert_eq!(app.filter, "systemd");
        assert!(!app.view.is_empty());
        for item in &app.view {
            let proc = &app.snap.procs[item.proc_idx];
            assert!(
                proc.name.to_lowercase().contains("systemd")
                    || proc.cmd.to_lowercase().contains("systemd")
                    || proc.user.to_lowercase().contains("systemd")
                    || proc.pid.to_string().contains("systemd")
            );
        }
        press(&mut app, KeyCode::Enter);
        assert!(!app.filter_edit);

        let before = app.config.sort_by;
        press(&mut app, KeyCode::Right);
        assert_ne!(app.config.sort_by, before);

        press(&mut app, KeyCode::Delete);
        assert!(app.filter.is_empty());

        press(&mut app, KeyCode::Char('e'));
        assert!(app.config.tree);
        let _ = render(&mut app, 120, 36);
    }

    #[test]
    fn renders_with_an_installed_btop_theme() {
        let mut app = App::new().expect("app");
        let Some(name) = app
            .theme_names
            .iter()
            .find(|name| !name.eq_ignore_ascii_case("Default"))
            .cloned()
        else {
            return;
        };

        app.theme = crate::theme::Theme::load(&name);
        let output = render(&mut app, 120, 36);
        assert!(output.contains("cpu"));

        app.overlay = Overlay::Help;
        let output = render(&mut app, 120, 36);
        assert!(output.contains("Global"));
    }

    #[test]
    fn renders_all_layouts_and_overlays() {
        let mut app = App::new().expect("app");

        for (width, height) in [(60, 18), (64, 20), (78, 20), (80, 24), (100, 30), (160, 48)] {
            let _ = render(&mut app, width, height);
        }

        app.config.tree = true;
        app.rebuild_view();
        let _ = render(&mut app, 120, 36);
        app.config.tree = false;

        let pid = app.snap.procs.first().map(|proc| proc.pid).unwrap_or(1);
        let name = app
            .find_proc(pid)
            .map(|proc| proc.name.clone())
            .unwrap_or_default();
        let overlays = [
            Overlay::Menu { selected: 2 },
            Overlay::Options { selected: 5 },
            Overlay::Help,
            Overlay::Info { pid },
            Overlay::ConfirmKill {
                pid,
                name,
                signal: sysinfo::Signal::Term,
            },
        ];

        for overlay in overlays {
            app.overlay = overlay;
            let _ = render(&mut app, 60, 18);
            let _ = render(&mut app, 120, 36);
        }
    }

    #[test]
    fn tree_view_collapses_and_expands_children() {
        let mut app = App::new().expect("app");
        app.config.tree = true;
        app.rebuild_view();

        let Some(index) = (0..app.view.len().saturating_sub(1))
            .find(|&index| app.view[index + 1].depth > app.view[index].depth)
        else {
            return;
        };

        let pid = app.snap.procs[app.view[index].proc_idx].pid;
        app.selected = index;
        app.selected_pid = Some(pid);
        let expanded_len = app.view.len();

        press(&mut app, KeyCode::Char('-'));
        assert!(app.collapsed.contains(&pid));
        assert!(app.view.len() < expanded_len);

        press(&mut app, KeyCode::Char('+'));
        assert!(!app.collapsed.contains(&pid));
        assert_eq!(app.view.len(), expanded_len);
        assert_eq!(app.selected, index);
    }

    #[test]
    fn info_and_kill_confirmation_open() {
        let mut app = App::new().expect("app");

        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.overlay, Overlay::Info { .. }));
        let output = render(&mut app, 120, 36);
        assert!(output.contains("process"));
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Char('t'));
        assert!(matches!(app.overlay, Overlay::ConfirmKill { .. }));
        let output = render(&mut app, 120, 36);
        assert!(output.contains("SIGTERM"));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.overlay, Overlay::None));
    }
}
