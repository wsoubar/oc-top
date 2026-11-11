//! Application state, input handling and the main event-loop helpers.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::time::Instant;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use sysinfo::Signal;

use crate::collect::{Collector, MAX_UPDATE_MS, MIN_UPDATE_MS, Snapshot, UPDATE_STEP_MS};
use crate::config::{Config, GraphSymbol, Panel, SortBy};
use crate::theme::Theme;

pub const PANEL_OPTIONS_START: usize = 7;

const HISTORY_LEN: usize = 600;
pub const CORE_HISTORY_LEN: usize = 10;

#[derive(Clone, Debug)]
pub enum Overlay {
    None,
    Menu {
        selected: usize,
    },
    Options {
        selected: usize,
    },
    Help,
    ConfirmKill {
        pid: u32,
        name: String,
        signal: Signal,
    },
    Info {
        pid: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ViewItem {
    pub proc_idx: usize,
    pub depth: usize,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub theme_names: Vec<String>,
    pub snap: Snapshot,
    collector: Collector,

    pub cpu_hist: VecDeque<f32>,
    pub core_history: VecDeque<Vec<f32>>,
    pub down_hist: VecDeque<f32>,
    pub up_hist: VecDeque<f32>,
    pub top_rx: f64,
    pub top_tx: f64,

    pub view: Vec<ViewItem>,
    pub selected: usize,
    pub selected_pid: Option<u32>,
    pub offset: usize,

    pub filter: String,
    pub filter_backup: String,
    pub filter_edit: bool,
    pub paused: bool,
    pub collapsed: HashSet<u32>,

    pub overlay: Overlay,
    pub quit: bool,
    pub dirty: bool,

    /// Area of the process table rows, used for mouse hit-testing.
    pub proc_area: Rect,
    pub status: Option<(String, Instant)>,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let config = Config::load();
        let theme = Theme::load(&config.theme);
        let theme_names = Theme::available();

        let mut collector = Collector::new()?;
        let snap = collector.sample();

        let mut app = Self {
            config,
            theme,
            theme_names,
            snap,
            collector,
            cpu_hist: VecDeque::with_capacity(HISTORY_LEN),
            core_history: VecDeque::with_capacity(CORE_HISTORY_LEN),
            down_hist: VecDeque::with_capacity(HISTORY_LEN),
            up_hist: VecDeque::with_capacity(HISTORY_LEN),
            top_rx: 0.0,
            top_tx: 0.0,
            view: Vec::new(),
            selected: 0,
            selected_pid: None,
            offset: 0,
            filter: String::new(),
            filter_backup: String::new(),
            filter_edit: false,
            paused: false,
            collapsed: HashSet::new(),
            overlay: Overlay::None,
            quit: false,
            dirty: true,
            proc_area: Rect::default(),
            status: None,
        };

        // Start the graphs with a little bit of flat history so they don't look
        // broken on the first frames.
        for _ in 0..60 {
            app.push_cpu(0.0);
            app.push_net(0.0, 0.0);
        }
        app.push_history();
        app.rebuild_view();
        Ok(app)
    }

    pub fn save_config(&mut self) {
        self.config.theme = self.theme.name.clone();
        self.config.save();
    }

    pub fn tick(&mut self) {
        if !self.paused {
            self.snap = self.collector.sample();
            self.push_history();
            self.top_rx = self.top_rx.max(self.snap.net_rx_bps);
            self.top_tx = self.top_tx.max(self.snap.net_tx_bps);
        }
        self.rebuild_view();
        self.dirty = true;
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        self.dirty = true;
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }

        if self.filter_edit {
            self.on_filter_key(key);
            return;
        }

        let overlay = self.overlay.clone();
        match overlay {
            Overlay::None => self.on_main_key(key),
            Overlay::Menu { selected } => self.on_menu_key(key, selected),
            Overlay::Options { selected } => self.on_options_key(key, selected),
            Overlay::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc
                        | KeyCode::Enter
                        | KeyCode::Char('q')
                        | KeyCode::Char('h')
                        | KeyCode::Char('?')
                ) {
                    self.overlay = Overlay::None;
                }
            }
            Overlay::ConfirmKill { pid, name, signal } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.kill(pid, &name, signal)
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.overlay = Overlay::None;
                }
                _ => {}
            },
            Overlay::Info { .. } => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('i')
                ) {
                    self.overlay = Overlay::None;
                }
            }
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        self.dirty = true;
        if !matches!(self.overlay, Overlay::None) || self.filter_edit || self.proc_area.is_empty() {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => self.select_delta(-3),
            MouseEventKind::ScrollDown => self.select_delta(3),
            MouseEventKind::Down(MouseButton::Left) => {
                let area = self.proc_area;
                if area.width > 0
                    && area.height > 0
                    && mouse.row >= area.y
                    && mouse.row < area.y + area.height
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    let index = self.offset + (mouse.row - area.y) as usize;
                    if index < self.view.len() {
                        self.selected = index;
                        self.sync_selected_pid();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn selected_proc(&self) -> Option<&crate::collect::ProcInfo> {
        self.view
            .get(self.selected)
            .map(|item| &self.snap.procs[item.proc_idx])
    }

    pub fn find_proc(&self, pid: u32) -> Option<&crate::collect::ProcInfo> {
        self.snap.procs.iter().find(|proc| proc.pid == pid)
    }

    pub fn proc_rows(&self) -> usize {
        self.proc_area.height.max(1) as usize
    }

    fn on_main_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc | KeyCode::Char('m') => {
                self.overlay = Overlay::Menu { selected: 0 };
            }
            KeyCode::F(1) | KeyCode::Char('?') | KeyCode::Char('h') => {
                self.overlay = Overlay::Help;
            }
            KeyCode::F(2) | KeyCode::Char('o') => {
                self.overlay = Overlay::Options { selected: 0 };
            }
            KeyCode::Char('0') => self.config.panels.cycle_preset(true),
            KeyCode::Char(number @ '1'..='5') => {
                self.config
                    .panels
                    .toggle(Panel::ALL[(number as u8 - b'1') as usize]);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if self.config.tree && self.config.panels.contains(Panel::Processes) {
                    self.set_collapsed(false);
                } else {
                    self.adjust_interval(UPDATE_STEP_MS as i64);
                }
            }
            KeyCode::Char('-') => {
                if self.config.tree && self.config.panels.contains(Panel::Processes) {
                    self.set_collapsed(true);
                } else {
                    self.adjust_interval(-(UPDATE_STEP_MS as i64));
                }
            }
            _ if !self.config.panels.contains(Panel::Processes) => {}
            KeyCode::Up => self.select_delta(-1),
            KeyCode::Down => self.select_delta(1),
            KeyCode::PageUp => {
                let rows = self.proc_rows() as isize;
                self.select_delta(-rows);
            }
            KeyCode::PageDown => {
                let rows = self.proc_rows() as isize;
                self.select_delta(rows);
            }
            KeyCode::Home => self.select_to(0),
            KeyCode::End => {
                let last = self.view.len().saturating_sub(1);
                self.select_to(last);
            }
            KeyCode::Left => self.cycle_sort(false),
            KeyCode::Right => self.cycle_sort(true),
            KeyCode::Char('r') => {
                self.config.reverse = !self.config.reverse;
                self.rebuild_view();
            }
            KeyCode::Char('f') | KeyCode::Char('/') => {
                self.filter_edit = true;
                self.filter_backup = self.filter.clone();
            }
            KeyCode::Delete => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.rebuild_view();
                }
            }
            KeyCode::Char('e') => {
                self.config.tree = !self.config.tree;
                self.rebuild_view();
            }
            KeyCode::Char('u') => {
                self.paused = !self.paused;
                self.status = Some((
                    if self.paused {
                        "process list paused".to_string()
                    } else {
                        "process list resumed".to_string()
                    },
                    Instant::now(),
                ));
            }
            KeyCode::Char('t') => self.start_kill(Signal::Term),
            KeyCode::Char('k') => self.start_kill(Signal::Kill),
            KeyCode::Enter | KeyCode::Char('i') => self.show_info(),
            _ => {}
        }
    }

    fn on_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.filter_edit = false;
                self.rebuild_view();
            }
            KeyCode::Esc => {
                self.filter = self.filter_backup.clone();
                self.filter_edit = false;
                self.rebuild_view();
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.rebuild_view();
            }
            KeyCode::Delete => {
                self.filter.clear();
                self.rebuild_view();
            }
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                self.filter_edit = false;
                self.rebuild_view();
            }
            KeyCode::Char(ch)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.filter.push(ch);
                self.rebuild_view();
            }
            _ => {}
        }
    }

    fn on_menu_key(&mut self, key: KeyEvent, selected: usize) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.overlay = Overlay::Menu {
                    selected: (selected + 2) % 3,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.overlay = Overlay::Menu {
                    selected: (selected + 1) % 3,
                };
            }
            KeyCode::Enter => match selected {
                0 => self.overlay = Overlay::Options { selected: 0 },
                1 => self.overlay = Overlay::Help,
                _ => self.quit = true,
            },
            KeyCode::Char('o') => self.overlay = Overlay::Options { selected: 0 },
            KeyCode::Char('h') | KeyCode::Char('?') | KeyCode::F(1) => self.overlay = Overlay::Help,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc | KeyCode::Char('m') => self.overlay = Overlay::None,
            _ => {}
        }
    }

    fn on_options_key(&mut self, key: KeyEvent, selected: usize) {
        const OPTIONS: usize = PANEL_OPTIONS_START + Panel::ALL.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.overlay = Overlay::Options {
                    selected: (selected + OPTIONS - 1) % OPTIONS,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.overlay = Overlay::Options {
                    selected: (selected + 1) % OPTIONS,
                };
            }
            KeyCode::Left => self.adjust_option(selected, -1),
            KeyCode::Right | KeyCode::Enter => self.adjust_option(selected, 1),
            KeyCode::Esc | KeyCode::Char('m') => {
                self.overlay = Overlay::Menu { selected: 0 };
            }
            _ => {}
        }
    }

    fn adjust_option(&mut self, selected: usize, delta: i64) {
        match selected {
            0 => self.adjust_interval(delta * UPDATE_STEP_MS as i64),
            1 => {
                self.config.graph_symbol = if delta >= 0 {
                    self.config.graph_symbol.next()
                } else if self.config.graph_symbol == GraphSymbol::Braille {
                    GraphSymbol::Block
                } else {
                    GraphSymbol::Braille
                };
            }
            2 => {
                self.config.sort_by = if delta >= 0 {
                    self.config.sort_by.next()
                } else {
                    self.config.sort_by.previous()
                };
                self.rebuild_view();
            }
            3 => self.config.reverse = !self.config.reverse,
            4 => {
                self.config.tree = !self.config.tree;
                self.rebuild_view();
            }
            5 => self.cycle_theme(delta),
            6 => self.config.panels.cycle_preset(delta >= 0),
            _ => self
                .config
                .panels
                .toggle(Panel::ALL[selected - PANEL_OPTIONS_START]),
        }
        self.dirty = true;
    }

    fn adjust_interval(&mut self, delta: i64) {
        let ms = (self.config.update_ms as i64 + delta)
            .clamp(MIN_UPDATE_MS as i64, MAX_UPDATE_MS as i64) as u64;
        if ms != self.config.update_ms {
            self.config.update_ms = ms;
            self.status = Some((format!("update interval: {ms} ms"), Instant::now()));
        }
    }

    fn cycle_sort(&mut self, forward: bool) {
        self.config.sort_by = if forward {
            self.config.sort_by.next()
        } else {
            self.config.sort_by.previous()
        };
        self.rebuild_view();
        self.dirty = true;
    }

    fn cycle_theme(&mut self, delta: i64) {
        if self.theme_names.is_empty() {
            return;
        }

        let current = self
            .theme_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(&self.theme.name))
            .unwrap_or(0);
        let len = self.theme_names.len() as i64;
        let next = (current as i64 + delta).rem_euclid(len) as usize;
        let name = self.theme_names[next].clone();

        self.theme = Theme::load(&name);
        self.config.theme = name.clone();
        self.status = Some((format!("theme: {name}"), Instant::now()));
    }

    fn start_kill(&mut self, signal: Signal) {
        let Some(proc) = self.selected_proc() else {
            return;
        };
        let (pid, name) = (proc.pid, proc.name.clone());
        self.overlay = Overlay::ConfirmKill { pid, name, signal };
    }

    fn kill(&mut self, pid: u32, name: &str, signal: Signal) {
        let signal_name = match signal {
            Signal::Term => "SIGTERM",
            Signal::Kill => "SIGKILL",
            _ => "signal",
        };

        let ok = self.collector.kill(pid, signal);
        self.status = Some((
            format!(
                "{} {} ({pid} {name})",
                if ok { "sent" } else { "failed to send" },
                signal_name
            ),
            Instant::now(),
        ));
        self.overlay = Overlay::None;
        self.tick();
    }

    fn show_info(&mut self) {
        if let Some(proc) = self.selected_proc() {
            self.overlay = Overlay::Info { pid: proc.pid };
        }
    }

    fn select_delta(&mut self, delta: isize) {
        if self.view.is_empty() {
            return;
        }
        let last = self.view.len() as isize - 1;
        let next = (self.selected as isize + delta).clamp(0, last) as usize;
        if next != self.selected {
            self.selected = next;
            self.sync_selected_pid();
        }
    }

    fn select_to(&mut self, index: usize) {
        if self.view.is_empty() {
            return;
        }
        self.selected = index.min(self.view.len() - 1);
        self.sync_selected_pid();
    }

    /// Collapse or expand the children of the selected process in tree view,
    /// mirroring btop's `-` / `+` keys.
    fn set_collapsed(&mut self, collapse: bool) {
        if !self.config.tree {
            return;
        }
        let Some(current) = self.view.get(self.selected).copied() else {
            return;
        };
        let pid = self.snap.procs[current.proc_idx].pid;

        if collapse {
            let has_children = self
                .view
                .get(self.selected + 1)
                .is_some_and(|item| item.depth > current.depth);
            if has_children && self.collapsed.insert(pid) {
                self.status = Some(("children collapsed".to_string(), Instant::now()));
            }
        } else {
            // Expanding a node also unlocks every descendant currently visible.
            let mut targets = vec![pid];
            for item in self.view.iter().skip(self.selected + 1) {
                if item.depth <= current.depth {
                    break;
                }
                targets.push(self.snap.procs[item.proc_idx].pid);
            }
            let was_collapsed = targets.iter().any(|pid| self.collapsed.contains(pid));
            for target in targets {
                self.collapsed.remove(&target);
            }
            if was_collapsed {
                self.status = Some(("children expanded".to_string(), Instant::now()));
            }
        }

        self.rebuild_view();
    }

    fn sync_selected_pid(&mut self) {
        self.selected_pid = self
            .view
            .get(self.selected)
            .map(|item| self.snap.procs[item.proc_idx].pid);
    }

    pub fn rebuild_view(&mut self) {
        // Drop collapse state for processes that have exited (and might see
        // their pid reused).
        if self.config.tree && !self.collapsed.is_empty() {
            let alive: HashSet<u32> = self.snap.procs.iter().map(|proc| proc.pid).collect();
            self.collapsed.retain(|pid| alive.contains(pid));
        }

        let filter = self.filter.to_lowercase();
        let matches = |proc: &crate::collect::ProcInfo| {
            if filter.is_empty() {
                return true;
            }
            proc.name.to_lowercase().contains(&filter)
                || proc.cmd.to_lowercase().contains(&filter)
                || proc.user.to_lowercase().contains(&filter)
                || proc.pid.to_string().contains(&filter)
        };

        let mut indices: Vec<usize> = self
            .snap
            .procs
            .iter()
            .enumerate()
            .filter(|(_, proc)| matches(proc))
            .map(|(index, _)| index)
            .collect();

        if self.config.tree && filter.is_empty() {
            let visible: HashSet<u32> = indices.iter().map(|&i| self.snap.procs[i].pid).collect();
            let mut children: HashMap<u32, Vec<usize>> = HashMap::new();
            let mut roots = Vec::new();

            for &index in &indices {
                let proc = &self.snap.procs[index];
                match proc.parent {
                    Some(parent) if parent != proc.pid && visible.contains(&parent) => {
                        children.entry(parent).or_default().push(index);
                    }
                    _ => roots.push(index),
                }
            }

            let key = self.config.sort_by;
            let reverse = self.config.reverse;
            let procs = &self.snap.procs;
            roots.sort_by(|a, b| compare(&procs[*a], &procs[*b], key, reverse));
            for siblings in children.values_mut() {
                siblings.sort_by(|a, b| compare(&procs[*a], &procs[*b], key, reverse));
            }

            let mut view = Vec::with_capacity(indices.len());
            for &root in &roots {
                walk_tree(root, 0, procs, &children, &self.collapsed, &mut view);
            }
            self.view = view;
        } else {
            let key = self.config.sort_by;
            let reverse = self.config.reverse;
            let procs = &self.snap.procs;
            indices.sort_by(|a, b| compare(&procs[*a], &procs[*b], key, reverse));
            self.view = indices
                .into_iter()
                .map(|proc_idx| ViewItem { proc_idx, depth: 0 })
                .collect();
        }

        let found = self.selected_pid.and_then(|pid| {
            self.view
                .iter()
                .position(|item| self.snap.procs[item.proc_idx].pid == pid)
        });
        self.selected = match found {
            Some(index) => index,
            None => self.selected.min(self.view.len().saturating_sub(1)),
        };
        self.sync_selected_pid();
    }

    fn push_cpu(&mut self, value: f32) {
        if self.cpu_hist.len() >= HISTORY_LEN {
            self.cpu_hist.pop_front();
        }
        self.cpu_hist.push_back(value);
    }

    fn push_net(&mut self, down: f32, up: f32) {
        if self.down_hist.len() >= HISTORY_LEN {
            self.down_hist.pop_front();
        }
        if self.up_hist.len() >= HISTORY_LEN {
            self.up_hist.pop_front();
        }
        self.down_hist.push_back(down);
        self.up_hist.push_back(up);
    }

    fn push_history(&mut self) {
        self.push_cpu(self.snap.cpu_usage);
        if self.core_history.len() == CORE_HISTORY_LEN {
            self.core_history.pop_front();
        }
        self.core_history.push_back(self.snap.cores.clone());
        self.push_net(self.snap.net_rx_bps as f32, self.snap.net_tx_bps as f32);
    }
}

fn walk_tree(
    index: usize,
    depth: usize,
    procs: &[crate::collect::ProcInfo],
    children: &HashMap<u32, Vec<usize>>,
    collapsed: &HashSet<u32>,
    view: &mut Vec<ViewItem>,
) {
    view.push(ViewItem {
        proc_idx: index,
        depth,
    });

    let pid = procs[index].pid;
    if depth >= 64 || collapsed.contains(&pid) {
        return;
    }
    if let Some(siblings) = children.get(&pid) {
        for &child in siblings {
            walk_tree(child, depth + 1, procs, children, collapsed, view);
        }
    }
}

fn compare(
    a: &crate::collect::ProcInfo,
    b: &crate::collect::ProcInfo,
    key: SortBy,
    reverse: bool,
) -> Ordering {
    let ordering = match key {
        SortBy::Pid => a.pid.cmp(&b.pid),
        SortBy::Program => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortBy::Command => a.cmd.to_lowercase().cmp(&b.cmd.to_lowercase()),
        SortBy::Threads => b.threads.cmp(&a.threads),
        SortBy::User => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
        SortBy::Memory => b.mem.cmp(&a.mem),
        SortBy::Cpu => b.cpu.total_cmp(&a.cpu),
    };
    let ordering = if reverse {
        ordering.reverse()
    } else {
        ordering
    };

    if ordering == Ordering::Equal {
        a.pid.cmp(&b.pid)
    } else {
        ordering
    }
}
