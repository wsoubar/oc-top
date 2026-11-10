//! System data collection built on top of [`sysinfo`].
//!
//! [`Collector`] owns the `sysinfo` handles and produces a cheap-to-render
//! [`Snapshot`] on every tick. Network and disk speeds are derived from the
//! counters reported by `sysinfo` using the elapsed time between samples.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::time::Instant;

use sysinfo::{
    Components, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind,
};

#[derive(Clone, Debug, Default)]
pub struct MemInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub free: u64,
    pub cached: u64,
}

impl MemInfo {
    pub fn used_pct(&self) -> f32 {
        percent(self.used, self.total)
    }

    pub fn available_pct(&self) -> f32 {
        percent(self.available, self.total)
    }

    pub fn cached_pct(&self) -> f32 {
        percent(self.cached, self.total)
    }

    pub fn free_pct(&self) -> f32 {
        percent(self.free, self.total)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SwapInfo {
    pub total: u64,
    pub used: u64,
}

impl SwapInfo {
    pub fn used_pct(&self) -> f32 {
        percent(self.used, self.total)
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiskInfo {
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub read_bps: f64,
    pub write_bps: f64,
}

impl DiskInfo {
    pub fn used_pct(&self) -> f32 {
        percent(self.used, self.total)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetInfo {
    pub name: String,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub total_rx: u64,
    pub total_tx: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ProcInfo {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub cmd: String,
    pub exe: String,
    pub user: String,
    pub status: String,
    pub threads: usize,
    pub cpu: f32,
    pub mem: u64,
    pub run_time: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub cpu_usage: f32,
    pub cores: Vec<f32>,
    pub freqs: Vec<u64>,
    pub cpu_brand: String,
    pub cpu_temp: Option<f32>,
    pub load: (f64, f64, f64),
    pub mem: MemInfo,
    pub swap: SwapInfo,
    pub disks: Vec<DiskInfo>,
    pub nets: Vec<NetInfo>,
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
    pub net_total_rx: u64,
    pub net_total_tx: u64,
    pub procs: Vec<ProcInfo>,
    pub uptime: u64,
    pub hostname: String,
    pub kernel: String,
}

impl Snapshot {
    pub fn core_count(&self) -> usize {
        self.cores.len()
    }
}

pub struct Collector {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    prev_disk: HashMap<String, (u64, u64)>,
    prev_net: HashMap<String, (u64, u64)>,
    last_sample: Instant,
    users: HashMap<u32, String>,
    tick: u64,
}

impl Collector {
    pub fn new() -> io::Result<Self> {
        let mut sys = System::new_all().map_err(io::Error::other)?;

        // The first CPU usage measurement needs a small window to be useful.
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_all();

        let disks = Disks::new_with_refreshed_list().map_err(io::Error::other)?;
        let networks = Networks::new_with_refreshed_list().map_err(io::Error::other)?;
        let components = Components::new_with_refreshed_list().map_err(io::Error::other)?;

        Ok(Self {
            sys,
            disks,
            networks,
            components,
            prev_disk: HashMap::new(),
            prev_net: HashMap::new(),
            last_sample: Instant::now(),
            users: load_users(),
            tick: 0,
        })
    }

    /// Refresh every data source and return a render-ready snapshot.
    pub fn sample(&mut self) -> Snapshot {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        // Threads are included because btop shows the per-process thread count.
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_user(UpdateKind::OnlyIfNotSet)
                .with_tasks(),
        );

        if self.tick.is_multiple_of(5) {
            self.sys.refresh_cpu_frequency();
        }

        self.networks.refresh(true);
        self.disks.refresh(true);
        self.components.refresh(true);

        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_sample)
            .as_secs_f64()
            .max(0.001);
        self.last_sample = now;
        self.tick = self.tick.wrapping_add(1);

        let cores: Vec<f32> = self.sys.cpus().iter().map(|cpu| cpu.usage()).collect();
        let freqs: Vec<u64> = self.sys.cpus().iter().map(|cpu| cpu.frequency()).collect();
        let cpu_brand = self
            .sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|brand| !brand.is_empty())
            .unwrap_or_else(|| "CPU".to_string());

        let load = System::load_average()
            .map(|load| (load.one, load.five, load.fifteen))
            .unwrap_or((0.0, 0.0, 0.0));

        let mem = MemInfo {
            total: self.sys.total_memory(),
            used: self.sys.used_memory(),
            available: self.sys.available_memory(),
            free: self.sys.free_memory(),
            cached: self
                .sys
                .available_memory()
                .saturating_sub(self.sys.free_memory()),
        };
        let swap = SwapInfo {
            total: self.sys.total_swap(),
            used: self.sys.used_swap(),
        };

        let disks = self.collect_disks(elapsed);
        let nets = self.collect_networks(elapsed);

        let net_rx_bps = nets.iter().map(|net| net.rx_bps).sum();
        let net_tx_bps = nets.iter().map(|net| net.tx_bps).sum();
        let net_total_rx = nets.iter().map(|net| net.total_rx).sum();
        let net_total_tx = nets.iter().map(|net| net.total_tx).sum();

        let procs = self.collect_processes();

        Snapshot {
            cpu_usage: self.sys.global_cpu_usage(),
            cores,
            freqs,
            cpu_brand,
            cpu_temp: self.cpu_temperature(),
            load,
            mem,
            swap,
            disks,
            nets,
            net_rx_bps,
            net_tx_bps,
            net_total_rx,
            net_total_tx,
            procs,
            uptime: System::uptime().unwrap_or(0),
            hostname: System::host_name().unwrap_or_else(|_| "localhost".to_string()),
            kernel: System::kernel_version().unwrap_or_default(),
        }
    }

    pub fn kill(&mut self, pid: u32, signal: sysinfo::Signal) -> bool {
        let pid = sysinfo::Pid::from_u32(pid);
        self.sys
            .process(pid)
            .and_then(|process| process.kill_with(signal))
            .unwrap_or(false)
    }

    fn collect_disks(&mut self, elapsed: f64) -> Vec<DiskInfo> {
        let mut physical = Vec::new();
        let mut others = Vec::new();

        for disk in &self.disks {
            let total = disk.total_space();
            if total == 0 {
                continue;
            }

            let mount = disk.mount_point().to_string_lossy().to_string();
            if is_virtual_mount(&mount) {
                continue;
            }

            let fs = disk.file_system().to_string_lossy().to_lowercase();
            let usage = disk.usage();
            let total_read = usage.total_read_bytes;
            let total_written = usage.total_written_bytes;

            let previous = self
                .prev_disk
                .insert(mount.clone(), (total_read, total_written));
            let (read_delta, write_delta) = previous
                .map(|(old_read, old_write)| {
                    (
                        total_read.saturating_sub(old_read),
                        total_written.saturating_sub(old_write),
                    )
                })
                .unwrap_or((0, 0));

            let info = DiskInfo {
                mount,
                used: total.saturating_sub(disk.available_space()),
                total,
                read_bps: read_delta as f64 / elapsed,
                write_bps: write_delta as f64 / elapsed,
            };

            if is_physical_fs(&fs) {
                physical.push(info);
            } else {
                others.push(info);
            }
        }

        let mut disks = if physical.is_empty() {
            others
        } else {
            physical
        };
        disks.sort_by(|a, b| a.mount.cmp(&b.mount));
        disks
    }

    fn collect_networks(&mut self, elapsed: f64) -> Vec<NetInfo> {
        let mut regular = Vec::new();
        let mut loopback = Vec::new();

        for (name, data) in &self.networks {
            let name = name.to_string_lossy().to_string();
            let total_rx = data.total_received();
            let total_tx = data.total_transmitted();

            let previous = self.prev_net.insert(name.clone(), (total_rx, total_tx));
            let (rx_delta, tx_delta) = previous
                .map(|(old_rx, old_tx)| {
                    (
                        total_rx.saturating_sub(old_rx),
                        total_tx.saturating_sub(old_tx),
                    )
                })
                .unwrap_or((0, 0));

            let info = NetInfo {
                name: name.clone(),
                rx_bps: rx_delta as f64 / elapsed,
                tx_bps: tx_delta as f64 / elapsed,
                total_rx,
                total_tx,
            };

            if name == "lo" || name.starts_with("lo") {
                loopback.push(info);
            } else {
                regular.push(info);
            }
        }

        // Prefer the interface carrying the default route, like btop's default
        // interface selection. Fall back to the busiest-looking regular
        // interface, then to loopback when nothing else exists.
        let mut nets = if let Some(primary) = primary_interface() {
            let filtered: Vec<NetInfo> = regular
                .iter()
                .filter(|net| net.name == primary)
                .cloned()
                .collect();
            if filtered.is_empty() {
                regular
            } else {
                filtered
            }
        } else {
            regular
        };

        if nets.is_empty() {
            nets = loopback;
        }

        nets.sort_by(|a, b| a.name.cmp(&b.name));
        nets
    }

    fn collect_processes(&self) -> Vec<ProcInfo> {
        self.sys
            .processes()
            .values()
            .map(|process| {
                let name = process.name().to_string_lossy().to_string();
                let cmd = if process.cmd().is_empty() {
                    process
                        .exe()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| name.clone())
                } else {
                    process
                        .cmd()
                        .iter()
                        .map(|arg| arg.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                let uid = process.user_id().map(|uid| **uid);
                let user = uid
                    .and_then(|uid| self.users.get(&uid).cloned())
                    .or_else(|| uid.map(|uid| uid.to_string()))
                    .unwrap_or_else(|| "-".to_string());

                ProcInfo {
                    pid: process.pid().as_u32(),
                    parent: process.parent().map(|pid| pid.as_u32()),
                    name,
                    cmd,
                    exe: process
                        .exe()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default(),
                    user,
                    status: format!("{:?}", process.status()),
                    threads: process.tasks().map_or(0, |tasks| tasks.len()),
                    cpu: process.cpu_usage(),
                    mem: process.memory(),
                    run_time: process.run_time(),
                }
            })
            .collect()
    }

    fn cpu_temperature(&self) -> Option<f32> {
        let mut package_temp: Option<f32> = None;
        let mut any_temp: Option<f32> = None;

        for component in &self.components {
            let Some(temp) = component.temperature() else {
                continue;
            };
            any_temp = Some(any_temp.map_or(temp, |current| current.max(temp)));

            let label = component.label().to_lowercase();
            if label.contains("package")
                || label.contains("tctl")
                || label.contains("tdie")
                || label.contains("cpu")
                || label.contains("coretemp")
                || label.contains("k10temp")
            {
                package_temp = Some(package_temp.map_or(temp, |current| current.max(temp)));
            }
        }

        package_temp.or(any_temp)
    }
}

fn percent(part: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64 * 100.0) as f32
    }
}

/// Interface associated with the default IPv4 route, if any.
fn primary_interface() -> Option<String> {
    let contents = fs::read_to_string("/proc/net/route").ok()?;
    let mut best: Option<(u32, String)> = None;

    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 8 || fields[1] != "00000000" {
            continue;
        }
        let metric = fields[6].parse::<u32>().unwrap_or(u32::MAX);
        if best.as_ref().is_none_or(|(current, _)| metric < *current) {
            best = Some((metric, fields[0].to_string()));
        }
    }

    best.map(|(_, interface)| interface)
}

fn load_users() -> HashMap<u32, String> {
    let mut users = HashMap::new();
    let Ok(contents) = fs::read_to_string("/etc/passwd") else {
        return users;
    };

    for line in contents.lines() {
        let mut fields = line.split(':');
        let Some(name) = fields.next() else { continue };
        let _password = fields.next();
        let Some(uid) = fields.next() else { continue };
        if let Ok(uid) = uid.parse::<u32>() {
            users.insert(uid, name.to_string());
        }
    }

    users
}

fn is_virtual_mount(mount: &str) -> bool {
    ["/proc", "/sys", "/dev", "/snap"]
        .iter()
        .any(|prefix| mount == *prefix || mount.starts_with(&format!("{prefix}/")))
}

fn is_physical_fs(fs: &str) -> bool {
    matches!(
        fs,
        "ext2"
            | "ext3"
            | "ext4"
            | "btrfs"
            | "xfs"
            | "zfs"
            | "f2fs"
            | "jfs"
            | "reiserfs"
            | "bcachefs"
            | "vfat"
            | "exfat"
            | "ntfs"
            | "ntfs3"
            | "fuseblk"
            | "hfs"
            | "hfsplus"
            | "apfs"
            | "ufs"
    )
}

/// Minimum supported update interval, mirroring btop's lower bound.
pub const MIN_UPDATE_MS: u64 = 250;
pub const MAX_UPDATE_MS: u64 = 10_000;
pub const UPDATE_STEP_MS: u64 = 250;
