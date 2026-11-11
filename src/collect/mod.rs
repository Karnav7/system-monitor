//! Gathers one snapshot of the system per tick.

pub mod gpu;
pub mod history;
pub mod hypr;
pub mod temps;

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use sysinfo::{
    Disks, Networks, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind,
    Users,
};

pub use gpu::GpuInfo;
use history::{rate, Histories};
pub use hypr::Window;
pub use temps::Temp;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub cmd: String,
    pub user: String,
    /// Percent of one core, like top: 200 means two full cores.
    pub cpu: f32,
    /// Resident memory in bytes.
    pub mem: u64,
    pub state: char,
    pub threads: usize,
}

#[derive(Debug, Clone, Default)]
pub struct NetIf {
    pub name: String,
    pub rx_rate: f64,
    pub tx_rate: f64,
    pub rx_total: u64,
    pub tx_total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DiskInfo {
    pub device: String,
    pub mount: String,
    pub total: u64,
    pub available: u64,
    pub read_rate: f64,
    pub write_rate: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub cpu_brand: String,
    pub cpu_total: f32,
    pub cores: Vec<f32>,
    pub load: (f64, f64, f64),
    pub uptime: u64,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub procs: Vec<ProcInfo>,
    pub nets: Vec<NetIf>,
    pub disks: Vec<DiskInfo>,
    pub temps: Vec<Temp>,
    pub gpus: Vec<GpuInfo>,
    pub windows: Vec<Window>,
    pub hypr_error: Option<String>,
}

pub struct Collector {
    sys: System,
    users: Users,
    nets: Networks,
    disks: Disks,
    last: Instant,
    ticks: u64,
    pub hist: Histories,
    pub snap: Snapshot,
}

fn state_char(s: ProcessStatus) -> char {
    match s {
        ProcessStatus::Run => 'R',
        ProcessStatus::Sleep => 'S',
        ProcessStatus::Idle => 'I',
        ProcessStatus::Zombie => 'Z',
        ProcessStatus::Stop => 'T',
        ProcessStatus::Tracing => 't',
        ProcessStatus::UninterruptibleDiskSleep => 'D',
        ProcessStatus::Dead => 'X',
        _ => '?',
    }
}

impl Collector {
    pub fn new() -> Self {
        let mut c = Collector {
            sys: System::new(),
            users: Users::new_with_refreshed_list(),
            nets: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last: Instant::now(),
            ticks: 0,
            hist: Histories::default(),
            snap: Snapshot::default(),
        };
        c.refresh();
        c
    }

    pub fn refresh(&mut self) {
        let now = Instant::now();
        let secs = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.ticks += 1;

        // New users rarely appear; a refresh every minute or so is plenty.
        if self.ticks.is_multiple_of(60) {
            self.users.refresh();
        }

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
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
        self.nets.refresh(true);
        self.disks.refresh(true);

        let s = &mut self.snap;
        s.cpu_brand = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default();
        s.cpu_total = self.sys.global_cpu_usage();
        s.cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let la = System::load_average();
        s.load = (la.one, la.five, la.fifteen);
        s.uptime = System::uptime();
        s.mem_total = self.sys.total_memory();
        s.mem_used = self.sys.used_memory();
        s.swap_total = self.sys.total_swap();
        s.swap_used = self.sys.used_swap();

        s.procs = self
            .sys
            .processes()
            .values()
            // On Linux sysinfo also lists each thread as a process; keep only real processes.
            .filter(|p| p.thread_kind().is_none())
            .map(|p| ProcInfo {
                pid: p.pid().as_u32(),
                ppid: p.parent().map(|pp| pp.as_u32()),
                name: p.name().to_string_lossy().into_owned(),
                cmd: p
                    .cmd()
                    .iter()
                    .map(|a| a.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                user: p
                    .user_id()
                    .and_then(|u| self.users.get_user_by_id(u))
                    .map(|u| u.name().to_string())
                    .unwrap_or_else(|| "?".into()),
                cpu: p.cpu_usage(),
                mem: p.memory(),
                state: state_char(p.status()),
                threads: p.tasks().map(|t| t.len()).unwrap_or(1).max(1),
            })
            .collect();

        s.nets = self
            .nets
            .iter()
            .filter(|(name, _)| name.as_str() != "lo")
            .map(|(name, d)| NetIf {
                name: name.clone(),
                rx_rate: rate(d.received(), secs),
                tx_rate: rate(d.transmitted(), secs),
                rx_total: d.total_received(),
                tx_total: d.total_transmitted(),
            })
            .collect();
        s.nets.sort_by(|a, b| a.name.cmp(&b.name));

        // Btrfs subvolumes and bind mounts show the same device several times;
        // count each device once so its I/O isn't multiplied.
        let mut seen = HashSet::new();
        s.disks = self
            .disks
            .list()
            .iter()
            .filter(|d| seen.insert(d.name().to_os_string()))
            .map(|d| {
                let u = d.usage();
                DiskInfo {
                    device: d.name().to_string_lossy().into_owned(),
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    total: d.total_space(),
                    available: d.available_space(),
                    read_rate: rate(u.read_bytes, secs),
                    write_rate: rate(u.written_bytes, secs),
                }
            })
            .collect();

        s.temps = temps::read_temps(Path::new("/sys/class/hwmon"));

        s.gpus = gpu::read_gpus(Path::new("/sys/class/drm"));

        match hypr::clients() {
            Ok(w) => {
                s.windows = w;
                s.hypr_error = None;
            }
            Err(e) => {
                s.windows.clear();
                s.hypr_error = Some(e);
            }
        }

        let h = &mut self.hist;
        h.cpu.push(s.cpu_total.round() as u64);
        h.mem
            .push((s.mem_used * 100).checked_div(s.mem_total).unwrap_or(0));
        h.net_rx
            .push(s.nets.iter().map(|n| n.rx_rate).sum::<f64>() as u64);
        h.net_tx
            .push(s.nets.iter().map(|n| n.tx_rate).sum::<f64>() as u64);
        h.disk_read
            .push(s.disks.iter().map(|d| d.read_rate).sum::<f64>() as u64);
        h.disk_write
            .push(s.disks.iter().map(|d| d.write_rate).sum::<f64>() as u64);
        h.gpu
            .push(s.gpus.first().and_then(|g| g.busy).unwrap_or(0) as u64);
    }
}
