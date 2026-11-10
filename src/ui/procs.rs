//! Process list panel.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Row, Table, TableState};

use crate::app::App;
use crate::config::SortBy;
use crate::theme::Theme;
use crate::util;
use crate::widgets::panel;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 3 || area.width < 24 {
        return;
    }

    let theme = app.theme.clone();
    let sort = app.config.sort_by;
    let arrow = sort_arrow(sort, app.config.reverse);
    let show_threads = area.width >= 74;
    let show_user = area.width >= 86;

    let mut right_spans = vec![Span::styled(
        format!("{} {arrow} ", sort.label()),
        Style::default().fg(theme.hi_fg),
    )];
    if app.config.tree {
        right_spans.push(Span::styled("tree ", Style::default().fg(theme.hi_fg)));
    }
    if !app.filter.is_empty() {
        right_spans.push(Span::styled(
            format!("filter: {} ", util::truncate(&app.filter, 14)),
            Style::default().fg(theme.graph_text),
        ));
    }
    right_spans.push(Span::styled(
        format!("{} ", app.view.len()),
        Style::default().fg(theme.graph_text),
    ));

    let mut block = panel("proc", Some(4), theme.proc_box, &theme);
    block = block.title_top(Line::from(right_spans).alignment(Alignment::Right));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width == 0 {
        app.proc_area = Rect::default();
        return;
    }

    let program_width = if inner.width >= 100 {
        20
    } else if inner.width >= 80 {
        16
    } else {
        12
    };

    let mut widths = vec![
        Constraint::Length(7),
        Constraint::Length(program_width),
        Constraint::Fill(1),
    ];
    if show_threads {
        widths.push(Constraint::Length(7));
    }
    if show_user {
        widths.push(Constraint::Length(9));
    }
    widths.push(Constraint::Length(8));
    widths.push(Constraint::Length(6));

    let header = Row::new(header_cells(&theme, sort, show_threads, show_user))
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row<'static>> = app
        .view
        .iter()
        .map(|item| {
            let process = &app.snap.procs[item.proc_idx];
            let mut cells = Vec::with_capacity(7);

            cells.push(Cell::from(Span::styled(
                format!("{:>6}", process.pid),
                Style::default().fg(theme.inactive_fg),
            )));

            let indent = "  ".repeat(item.depth.min(12));
            let indent_width = indent.chars().count() as u16;
            let name_width = program_width.saturating_sub(indent_width).max(3) as usize;
            cells.push(Cell::from(Span::styled(
                format!("{indent}{}", util::truncate(&process.name, name_width)),
                Style::default().fg(theme.main_fg),
            )));

            cells.push(Cell::from(Span::styled(
                util::truncate(&process.cmd, 220),
                Style::default().fg(theme.main_fg),
            )));

            if show_threads {
                cells.push(Cell::from(Span::styled(
                    format!("{:>6}", process.threads),
                    Style::default().fg(theme.main_fg),
                )));
            }
            if show_user {
                cells.push(Cell::from(Span::styled(
                    util::truncate(&process.user, 8),
                    Style::default().fg(theme.main_fg),
                )));
            }

            cells.push(Cell::from(Span::styled(
                format!("{:>6}", util::fmt_bytes_short(process.mem)),
                Style::default().fg(theme.proc_misc),
            )));

            let cpu_color = theme.process.at((process.cpu / 100.0).clamp(0.0, 1.0));
            cells.push(Cell::from(Span::styled(
                format!("{:>5.1}", process.cpu),
                Style::default().fg(cpu_color),
            )));

            Row::new(cells)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(theme.selected_bg)
                .fg(theme.selected_fg)
                .add_modifier(Modifier::BOLD),
        );

    let visible = inner.height.saturating_sub(1) as usize;
    let mut offset = app.offset;

    if visible > 0 && !app.view.is_empty() {
        if app.selected < offset {
            offset = app.selected;
        }
        if app.selected >= offset + visible {
            offset = app.selected + 1 - visible;
        }
        offset = offset.min(app.view.len().saturating_sub(visible));
    } else {
        offset = 0;
    }

    app.offset = offset;
    app.proc_area = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );

    let mut state = TableState::default();
    if !app.view.is_empty() {
        state.select(Some(app.selected));
    }
    *state.offset_mut() = offset;

    frame.render_stateful_widget(table, inner, &mut state);
}

fn header_cells(
    theme: &Theme,
    sort: SortBy,
    show_threads: bool,
    show_user: bool,
) -> Vec<Cell<'static>> {
    let cell = |label: &str, active: bool| {
        let style = if active {
            Style::default().fg(theme.hi_fg)
        } else {
            Style::default().fg(theme.inactive_fg)
        };
        Cell::from(Span::styled(label.to_string(), style))
    };

    let mut cells = vec![
        cell("Pid", sort == SortBy::Pid),
        cell("Program", sort == SortBy::Program),
        cell("Command", sort == SortBy::Command),
    ];
    if show_threads {
        cells.push(cell("Threads", sort == SortBy::Threads));
    }
    if show_user {
        cells.push(cell("User", sort == SortBy::User));
    }
    cells.push(cell("Mem", sort == SortBy::Memory));
    cells.push(cell("Cpu%", sort == SortBy::Cpu));
    cells
}

fn sort_arrow(sort: SortBy, reverse: bool) -> &'static str {
    let default_descending = matches!(sort, SortBy::Cpu | SortBy::Memory | SortBy::Threads);
    let descending = default_descending ^ reverse;
    if descending { "↓" } else { "↑" }
}
