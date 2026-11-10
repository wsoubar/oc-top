//! Persistent configuration and user-facing enums.

use std::env;
use std::fs;
use std::path::PathBuf;

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
             theme = {}\n",
            self.update_ms,
            self.graph_symbol.as_str(),
            self.sort_by.as_str(),
            self.reverse,
            self.tree,
            self.theme
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
