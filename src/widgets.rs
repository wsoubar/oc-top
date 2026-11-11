//! Reusable drawing widgets: panels, meters and braille/block history graphs.

use std::collections::VecDeque;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use crate::config::GraphSymbol;
use crate::theme::{Gradient, Theme};
use crate::util;

/// A rounded box with a btop-style numbered title.
pub fn panel(title: &str, number: Option<u8>, color: Color, theme: &Theme) -> Block<'static> {
    let mut spans = Vec::new();
    if let Some(number) = number {
        spans.push(Span::styled(
            number.to_string(),
            Style::default()
                .fg(theme.hi_fg)
                .add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(
        format!(" {title} "),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    ));

    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(Line::from(spans));

    if let Some(background) = theme.main_bg {
        block = block.style(Style::default().bg(background));
    }
    block
}

/// A labelled gradient bar, e.g. `Used   4.03 GiB  █████░░░ 26%`.
pub struct Meter<'a> {
    pub label: &'a str,
    pub label_width: u16,
    pub value: f32,
    pub value_text: Option<String>,
    pub gradient: Gradient,
    pub theme: &'a Theme,
    pub show_pct: bool,
}

impl Widget for Meter<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height == 0 {
            return;
        }

        let y = area.y;
        let label_width = self.label_width.min(area.width / 2).max(1);
        buf.set_stringn(
            area.x,
            y,
            util::truncate(self.label, label_width as usize),
            label_width as usize,
            Style::default().fg(self.theme.inactive_fg),
        );

        let value = self.value.clamp(0.0, 100.0);
        let pct = format!("{:.0}%", value);
        let pct_width = if self.show_pct {
            (pct.chars().count() as u16).min(area.width / 3)
        } else {
            0
        };
        let right_start = area.x + area.width - pct_width;
        let mut x = area.x + label_width;

        if let Some(text) = self.value_text.as_deref() {
            let available = right_start.saturating_sub(x + 1);
            if available > 3 {
                let text_width = (text.chars().count() as u16 + 1).min(available);
                let printed = util::truncate(text, text_width.saturating_sub(1) as usize);
                buf.set_stringn(
                    x + 1,
                    y,
                    printed,
                    text_width.saturating_sub(1) as usize,
                    Style::default().fg(self.theme.main_fg),
                );
                x += text_width;
            }
        }

        let bar_start = x + 1;
        let bar_end = right_start.saturating_sub(1);
        MeterBar {
            value,
            gradient: self.gradient,
            background: self.theme.meter_bg,
        }
        .render(
            Rect::new(bar_start, y, bar_end.saturating_sub(bar_start), 1),
            buf,
        );

        if pct_width > 0 {
            let printed = util::truncate(&pct, pct_width as usize);
            let x = area.x + area.width - printed.chars().count() as u16;
            buf.set_stringn(
                x,
                y,
                &printed,
                printed.chars().count(),
                Style::default().fg(self.theme.main_fg),
            );
        }
    }
}

pub struct MeterBar {
    pub value: f32,
    pub gradient: Gradient,
    pub background: Color,
}

impl Widget for MeterBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for offset in 0..area.width {
            let position = (offset as f32 + 0.5) / area.width as f32;
            let background = if position * 100.0 <= self.value {
                self.gradient.at(position)
            } else {
                self.background
            };
            buf[(area.x + offset, area.y)]
                .set_symbol(" ")
                .set_bg(background);
        }
    }
}

/// A time-series graph drawn with braille dots or block characters.
pub struct Graph<'a> {
    pub data: &'a VecDeque<f32>,
    /// Fixed maximum. `0.0` auto-scales to the visible data.
    pub max: f32,
    pub gradient: Gradient,
    pub symbol: GraphSymbol,
    pub theme: &'a Theme,
    /// Optional top-left label. When `None` an auto-scaled maximum is shown if
    /// `max` is zero.
    pub label: Option<String>,
}

impl Widget for Graph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let columns = match self.symbol {
            GraphSymbol::Braille => width * 2,
            GraphSymbol::Block => width,
        };

        let offset = columns.saturating_sub(self.data.len());
        let mut values = Vec::with_capacity(columns);
        for column in 0..columns {
            let value = if column < offset {
                0.0
            } else {
                self.data[self.data.len() - (columns - column)]
            };
            values.push(if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            });
        }

        let visible_max = values.iter().copied().fold(0.0_f32, f32::max);
        let max = if self.max > 0.0 {
            self.max
        } else {
            (visible_max * 1.1).max(1024.0)
        };

        match self.symbol {
            GraphSymbol::Braille => {
                self.render_braille(area, buf, &values, max);
            }
            GraphSymbol::Block => {
                self.render_block(area, buf, &values, max);
            }
        }

        let label = match &self.label {
            Some(label) => Some(label.clone()),
            None if self.max <= 0.0 => Some(util::fmt_speed(max as f64)),
            None => None,
        };
        if let Some(label) = label
            && height > 1
            && width > 4
        {
            buf.set_stringn(
                area.x,
                area.y,
                util::truncate(&label, width),
                width,
                Style::default().fg(self.theme.graph_text),
            );
        }
    }
}

impl Graph<'_> {
    fn render_braille(&self, area: Rect, buf: &mut Buffer, values: &[f32], max: f32) {
        let width = area.width as usize;
        let height = area.height as usize;
        let dots_per_column = height * 4;
        let mut bits = vec![0_u16; width * height];

        for x in 0..width {
            for sub in 0..2 {
                let value = values[x * 2 + sub];
                let filled =
                    ((value / max).clamp(0.0, 1.0) * dots_per_column as f32).round() as usize;

                for dot in 0..filled.min(dots_per_column) {
                    let row_from_bottom = dot / 4;
                    let dot_row = dot % 4;
                    let y = height - 1 - row_from_bottom;
                    let bit = match (dot_row, sub) {
                        (0, 0) => 0x40,
                        (0, 1) => 0x80,
                        (1, 0) => 0x04,
                        (1, 1) => 0x20,
                        (2, 0) => 0x02,
                        (2, 1) => 0x10,
                        (3, 0) => 0x01,
                        (3, 1) => 0x08,
                        _ => 0,
                    };
                    bits[y * width + x] |= bit;
                }
            }
        }

        for y in 0..height {
            for x in 0..width {
                let dots = bits[y * width + x];
                if dots == 0 {
                    continue;
                }
                let symbol = char::from_u32(0x2800 + u32::from(dots)).unwrap_or(' ');
                let t = if height > 1 {
                    (height - 1 - y) as f32 / (height - 1) as f32
                } else {
                    0.5
                };
                buf[(area.x + x as u16, area.y + y as u16)]
                    .set_char(symbol)
                    .set_fg(self.gradient.at(t));
            }
        }
    }

    fn render_block(&self, area: Rect, buf: &mut Buffer, values: &[f32], max: f32) {
        const PARTIAL: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];
        let width = area.width as usize;
        let height = area.height as usize;

        for x in 0..width {
            let filled_total =
                ((values[x] / max).clamp(0.0, 1.0) * height as f32 * 8.0).round() as usize;

            for row_from_bottom in 0..height {
                let remaining = filled_total.saturating_sub(row_from_bottom * 8).min(8);
                if remaining == 0 {
                    continue;
                }

                let symbol = if remaining == 8 {
                    '█'
                } else {
                    PARTIAL[remaining]
                };
                let y = area.y + (height - 1 - row_from_bottom) as u16;
                let t = if height > 1 {
                    row_from_bottom as f32 / (height - 1) as f32
                } else {
                    0.5
                };
                buf[(area.x + x as u16, y)]
                    .set_char(symbol)
                    .set_fg(self.gradient.at(t));
            }
        }
    }
}
