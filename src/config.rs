//! Persistent configuration and user-facing enums.

use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Cpu,
    Memory,
    Network,
    Disks,
    Processes,
}

impl Panel {
    pub const ALL: [Self; 5] = [
        Self::Cpu,
        Self::Memory,
        Self::Network,
        Self::Disks,
        Self::Processes,
    ];

    pub fn key(self) -> char {
        char::from(b'1' + self as u8)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "mem",
            Self::Network => "net",
            Self::Disks => "disks",
            Self::Processes => "proc",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panels(u8);

impl Panels {
    pub const COMPLETE: Self = Self(0b11111);
    const PRESETS: [(Self, &'static str); 3] = [
        (Self::COMPLETE, "Complete"),
        (Self(Panel::Processes.bit()), "Processes"),
        (Self(Panel::Network.bit()), "Network"),
    ];

    pub fn contains(self, panel: Panel) -> bool {
        self.0 & panel.bit() != 0
    }

    pub fn toggle(&mut self, panel: Panel) {
        let next = self.0 ^ panel.bit();
        if next != 0 {
            self.0 = next;
        }
    }

    fn preset_index(self) -> Option<usize> {
        Self::PRESETS.iter().position(|(panels, _)| *panels == self)
    }

    pub fn preset_name(self) -> &'static str {
        self.preset_index()
            .map_or("Custom", |index| Self::PRESETS[index].1)
    }

    pub fn cycle_preset(&mut self, forward: bool) {
        let count = Self::PRESETS.len();
        let next = match self.preset_index() {
            Some(index) => (index + if forward { 1 } else { count - 1 }) % count,
            None if forward => 0,
            None => count - 1,
        };
        *self = Self::PRESETS[next].0;
    }

    fn parse(value: &str) -> Self {
        let mut panels = 0;
        for panel in Panel::ALL {
            if value.split_whitespace().any(|name| name == panel.name()) {
                panels |= panel.bit();
            }
        }
        if panels == 0 {
            Self::COMPLETE
        } else {
            Self(panels)
        }
    }

    fn config_value(self) -> String {
        Panel::ALL
            .into_iter()
            .filter(|panel| self.contains(*panel))
            .map(Panel::name)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphSymbol {
    Braille,
    Block,
}

impl GraphSymbol {
    pub fn label(self) -> &'static str {
        match self {
            GraphSymbol::Braille => "Braille",
            GraphSymbol::Block => "Block",
        }
    }

    pub fn next(self) -> Self {
        match self {
            GraphSymbol::Braille => GraphSymbol::Block,
            GraphSymbol::Block => GraphSymbol::Braille,
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "block" => GraphSymbol::Block,
            _ => GraphSymbol::Braille,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            GraphSymbol::Braille => "braille",
            GraphSymbol::Block => "block",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Pid,
    Program,
    Command,
    Threads,
    User,
    Memory,
    Cpu,
}

impl SortBy {
    pub fn label(self) -> &'static str {
        match self {
            SortBy::Pid => "pid",
            SortBy::Program => "program",
            SortBy::Command => "command",
            SortBy::Threads => "threads",
            SortBy::User => "user",
            SortBy::Memory => "memory",
            SortBy::Cpu => "cpu",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SortBy::Pid => SortBy::Program,
            SortBy::Program => SortBy::Command,
            SortBy::Command => SortBy::Threads,
            SortBy::Threads => SortBy::User,
            SortBy::User => SortBy::Memory,
            SortBy::Memory => SortBy::Cpu,
            SortBy::Cpu => SortBy::Pid,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            SortBy::Pid => SortBy::Cpu,
            SortBy::Program => SortBy::Pid,
            SortBy::Command => SortBy::Program,
            SortBy::Threads => SortBy::Command,
            SortBy::User => SortBy::Threads,
            SortBy::Memory => SortBy::User,
            SortBy::Cpu => SortBy::Memory,
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "pid" => SortBy::Pid,
            "program" => SortBy::Program,
            "command" | "arguments" => SortBy::Command,
            "threads" => SortBy::Threads,
            "user" => SortBy::User,
            "memory" | "mem" => SortBy::Memory,
            _ => SortBy::Cpu,
        }
    }

    fn as_str(self) -> &'static str {
        self.label()
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub update_ms: u64,
    pub graph_symbol: GraphSymbol,
    pub sort_by: SortBy,
    pub reverse: bool,
    pub tree: bool,
    pub theme: String,
    pub panels: Panels,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            update_ms: 2000,
            graph_symbol: GraphSymbol::Braille,
            sort_by: SortBy::Cpu,
            reverse: false,
            tree: false,
            theme: "Default".to_string(),
            panels: Panels::COMPLETE,
        }
    }
}

impl Config {
    /// Load configuration from `$XDG_CONFIG_HOME/rtop/rtop.conf` (or the
    /// equivalent `~/.config` path). Missing or malformed files fall back to
    /// defaults.
    pub fn load() -> Self {
        let mut config = Config::default();

        let Some(path) = config_path() else {
            return config;
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return config;
        };

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "update_ms" => {
                    if let Ok(ms) = value.parse::<u64>() {
                        config.update_ms = ms.clamp(250, 10_000);
                    }
                }
                "graph_symbol" => config.graph_symbol = GraphSymbol::parse(value),
                "proc_sorting" => config.sort_by = SortBy::parse(value),
                "proc_reversed" => config.reverse = parse_bool(value, config.reverse),
                "proc_tree" => config.tree = parse_bool(value, config.tree),
                "shown_panels" => config.panels = Panels::parse(value),
                "theme" if !value.is_empty() => {
                    config.theme = value.to_string();
                }
                _ => {}
            }
        }

        config
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let contents = format!(
            "# rtop configuration\n\
             # update time in milliseconds (250-10000)\n\
             update_ms = {}\n\
             # braille or block\n\
             graph_symbol = {}\n\
             # pid, program, command, threads, user, memory, cpu\n\
             proc_sorting = {}\n\
             proc_reversed = {}\n\
             proc_tree = {}\n\
             # name of a theme in ~/.config/rtop/themes or /usr/share/btop/themes\n\
             theme = {}\n\
             shown_panels = {}\n",
            self.update_ms,
            self.graph_symbol.as_str(),
            self.sort_by.as_str(),
            self.reverse,
            self.tree,
            self.theme,
            self.panels.config_value()
        );

        let _ = fs::write(path, contents);
    }
}

/// Directory used for rtop configuration and user themes.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("rtop"));
    }
    env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".config").join("rtop"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("rtop.conf"))
}

fn parse_bool(value: &str, fallback: bool) -> bool {
    match value.trim().to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => true,
        "false" | "no" | "off" | "0" => false,
        _ => fallback,
    }
}
