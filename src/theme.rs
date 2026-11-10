//! Color themes.
//!
//! The built-in palette mirrors btop++'s default theme, and rtop can also load
//! btop/bpytop theme files directly, so existing themes keep working.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ratatui::style::Color;

use crate::config::config_dir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gradient {
    pub start: [u8; 3],
    pub mid: [u8; 3],
    pub end: [u8; 3],
}

impl Gradient {
    pub const fn new(start: [u8; 3], mid: [u8; 3], end: [u8; 3]) -> Self {
        Self { start, mid, end }
    }

    /// Interpolate the gradient at `t` in `0.0..=1.0`.
    pub fn at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let (a, b, k) = if t < 0.5 {
            (self.start, self.mid, t * 2.0)
        } else {
            (self.mid, self.end, (t - 0.5) * 2.0)
        };
        let [r, g, b] = mix(a, b, k);
        Color::Rgb(r, g, b)
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [lerp(a[0], b[0]), lerp(a[1], b[1]), lerp(a[2], b[2])]
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub main_bg: Option<Color>,
    pub main_fg: Color,
    pub title: Color,
    pub hi_fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub inactive_fg: Color,
    pub graph_text: Color,
    pub meter_bg: Color,
    pub proc_misc: Color,
    pub cpu_box: Color,
    pub mem_box: Color,
    pub net_box: Color,
    pub proc_box: Color,
    pub div_line: Color,
    pub temp: Gradient,
    pub cpu: Gradient,
    pub used: Gradient,
    pub available: Gradient,
    pub cached: Gradient,
    pub free: Gradient,
    pub download: Gradient,
    pub upload: Gradient,
    pub process: Gradient,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            main_bg: Some(Color::Rgb(0, 0, 0)),
            main_fg: color("#cc").unwrap_or(Color::Gray),
            title: color("#ee").unwrap_or(Color::White),
            hi_fg: color("#b54040").unwrap_or(Color::Red),
            selected_bg: color("#6a2f2f").unwrap_or(Color::DarkGray),
            selected_fg: color("#ee").unwrap_or(Color::White),
            inactive_fg: color("#40").unwrap_or(Color::DarkGray),
            graph_text: color("#60").unwrap_or(Color::Gray),
            meter_bg: color("#40").unwrap_or(Color::DarkGray),
            proc_misc: color("#0de756").unwrap_or(Color::Green),
            cpu_box: color("#556d59").unwrap_or(Color::Green),
            mem_box: color("#6c6c4b").unwrap_or(Color::Yellow),
            net_box: color("#5c588d").unwrap_or(Color::Blue),
            proc_box: color("#805252").unwrap_or(Color::Red),
            div_line: color("#30").unwrap_or(Color::DarkGray),
            temp: Gradient::new([0x48, 0x97, 0xd4], [0x54, 0x74, 0xe8], [0xff, 0x40, 0xb6]),
            cpu: Gradient::new([0x77, 0xca, 0x9b], [0xcb, 0xc0, 0x6c], [0xdc, 0x4c, 0x4c]),
            used: Gradient::new([0x59, 0x2b, 0x26], [0xd9, 0x62, 0x6d], [0xff, 0x47, 0x69]),
            available: Gradient::new([0x4e, 0x3f, 0x0e], [0xff, 0xd7, 0x7a], [0xff, 0xb8, 0x14]),
            cached: Gradient::new([0x16, 0x33, 0x50], [0x74, 0xe6, 0xfc], [0x26, 0xc5, 0xff]),
            free: Gradient::new([0x38, 0x4f, 0x21], [0xb5, 0xe6, 0x85], [0xdc, 0xff, 0x85]),
            download: Gradient::new([0x29, 0x1f, 0x75], [0x4f, 0x43, 0xa3], [0xb0, 0xa9, 0xde]),
            upload: Gradient::new([0x62, 0x06, 0x65], [0x7d, 0x41, 0x80], [0xdc, 0xaf, 0xde]),
            process: Gradient::new([0x80, 0xd0, 0xa3], [0xdc, 0xd1, 0x79], [0xd4, 0x54, 0x54]),
        }
    }
}

impl Theme {
    pub fn load(name: &str) -> Self {
        if name.eq_ignore_ascii_case("default") {
            return Theme::default();
        }

        let Some(path) = find_theme(name) else {
            return Theme::default();
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return Theme::default();
        };

        let mut theme = Theme {
            name: name.to_string(),
            ..Theme::default()
        };

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(rest) = line.strip_prefix("theme[") else {
                continue;
            };
            let Some((key, value)) = rest.split_once(']') else {
                continue;
            };
            let value = value
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"');
            if value.is_empty() {
                continue;
            }
            theme.apply(key, value);
        }

        theme
    }

    /// All themes available to the user, including themes installed for btop.
    pub fn available() -> Vec<String> {
        let mut names = BTreeSet::new();
        names.insert("Default".to_string());

        for dir in theme_dirs() {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "theme")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    names.insert(stem.to_string());
                }
            }
        }

        let mut names: Vec<String> = names.into_iter().collect();
        names.sort_by_key(|name| (name != "Default", name.to_lowercase()));
        names
    }

    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "main_bg" => {
                // rtop always paints an opaque background; btop's `#00`
                // (terminal default) becomes pure black.
                self.main_bg = if value.trim_matches('"') == "#00" || value.is_empty() {
                    Some(Color::Rgb(0, 0, 0))
                } else {
                    color(value).or(Some(Color::Rgb(0, 0, 0)))
                };
            }
            "main_fg" => set_color(&mut self.main_fg, value),
            "title" => set_color(&mut self.title, value),
            "hi_fg" => set_color(&mut self.hi_fg, value),
            "selected_bg" => set_color(&mut self.selected_bg, value),
            "selected_fg" => set_color(&mut self.selected_fg, value),
            "inactive_fg" => set_color(&mut self.inactive_fg, value),
            "graph_text" => set_color(&mut self.graph_text, value),
            "meter_bg" => set_color(&mut self.meter_bg, value),
            "proc_misc" => set_color(&mut self.proc_misc, value),
            "cpu_box" => set_color(&mut self.cpu_box, value),
            "mem_box" => set_color(&mut self.mem_box, value),
            "net_box" => set_color(&mut self.net_box, value),
            "proc_box" => set_color(&mut self.proc_box, value),
            "div_line" => set_color(&mut self.div_line, value),
            "temp_start" => set_gradient(&mut self.temp, 0, value),
            "temp_mid" => set_gradient(&mut self.temp, 1, value),
            "temp_end" => set_gradient(&mut self.temp, 2, value),
            "cpu_start" => set_gradient(&mut self.cpu, 0, value),
            "cpu_mid" => set_gradient(&mut self.cpu, 1, value),
            "cpu_end" => set_gradient(&mut self.cpu, 2, value),
            "used_start" => set_gradient(&mut self.used, 0, value),
            "used_mid" => set_gradient(&mut self.used, 1, value),
            "used_end" => set_gradient(&mut self.used, 2, value),
            "available_start" => set_gradient(&mut self.available, 0, value),
            "available_mid" => set_gradient(&mut self.available, 1, value),
            "available_end" => set_gradient(&mut self.available, 2, value),
            "cached_start" => set_gradient(&mut self.cached, 0, value),
            "cached_mid" => set_gradient(&mut self.cached, 1, value),
            "cached_end" => set_gradient(&mut self.cached, 2, value),
            "free_start" => set_gradient(&mut self.free, 0, value),
            "free_mid" => set_gradient(&mut self.free, 1, value),
            "free_end" => set_gradient(&mut self.free, 2, value),
            "download_start" => set_gradient(&mut self.download, 0, value),
            "download_mid" => set_gradient(&mut self.download, 1, value),
            "download_end" => set_gradient(&mut self.download, 2, value),
            "upload_start" => set_gradient(&mut self.upload, 0, value),
            "upload_mid" => set_gradient(&mut self.upload, 1, value),
            "upload_end" => set_gradient(&mut self.upload, 2, value),
            "process_start" => set_gradient(&mut self.process, 0, value),
            "process_mid" => set_gradient(&mut self.process, 1, value),
            "process_end" => set_gradient(&mut self.process, 2, value),
            _ => {}
        }
    }
}

pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(dir) = config_dir() {
        dirs.push(dir.join("themes"));
    }
    if let Some(home) = env::var_os("HOME").filter(|v| !v.is_empty()) {
        dirs.push(
            PathBuf::from(home)
                .join(".config")
                .join("btop")
                .join("themes"),
        );
    }
    dirs.push(PathBuf::from("/usr/local/share/btop/themes"));
    dirs.push(PathBuf::from("/usr/share/btop/themes"));
    dirs.push(PathBuf::from("themes"));

    dirs
}

fn find_theme(name: &str) -> Option<PathBuf> {
    for dir in theme_dirs() {
        let path = dir.join(format!("{name}.theme"));
        if Path::new(&path).is_file() {
            return Some(path);
        }
    }
    None
}

fn set_color(target: &mut Color, value: &str) {
    if let Some(color) = color(value) {
        *target = color;
    }
}

fn set_gradient(target: &mut Gradient, index: usize, value: &str) {
    let Some(rgb) = rgb(value) else {
        return;
    };
    match index {
        0 => target.start = rgb,
        1 => target.mid = rgb,
        _ => target.end = rgb,
    }
}

/// Parse btop-style colors: `#rrggbb` for RGB and `#nn` for a grayscale value.
///
/// btop expands `#nn` to `rgb(nn, nn, nn)` in truecolor mode, so `#40` is the
/// dark gray `rgb(64, 64, 64)` — not ANSI color index 64.
fn color(value: &str) -> Option<Color> {
    let value = value.trim().trim_matches('"');
    if let Some(rgb) = rgb(value) {
        return Some(Color::Rgb(rgb[0], rgb[1], rgb[2]));
    }

    let hex = value.strip_prefix('#')?;
    if hex.len() == 2 {
        let gray = u8::from_str_radix(hex, 16).ok()?;
        return Some(Color::Rgb(gray, gray, gray));
    }

    None
}

fn rgb(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_matches('"');
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_and_long_colors() {
        assert_eq!(color("#0f"), Some(Color::Rgb(15, 15, 15)));
        assert_eq!(color("#40"), Some(Color::Rgb(64, 64, 64)));
        assert_eq!(color("#102030"), Some(Color::Rgb(0x10, 0x20, 0x30)));
        assert_eq!(color("nope"), None);
    }

    #[test]
    fn default_background_is_black() {
        assert_eq!(Theme::default().main_bg, Some(Color::Rgb(0, 0, 0)));

        let mut theme = Theme::default();
        theme.apply("main_bg", "#00");
        assert_eq!(theme.main_bg, Some(Color::Rgb(0, 0, 0)));
        theme.apply("main_bg", "#101820");
        assert_eq!(theme.main_bg, Some(Color::Rgb(0x10, 0x18, 0x20)));
    }

    #[test]
    fn gradient_endpoints() {
        let gradient = Gradient::new([0, 0, 0], [128, 128, 128], [255, 255, 255]);
        assert_eq!(gradient.at(0.0), Color::Rgb(0, 0, 0));
        assert_eq!(gradient.at(1.0), Color::Rgb(255, 255, 255));
    }
}
