//! Sorting, filtering, the process tree, and grouping processes into apps.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::collect::{ProcInfo, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Cpu,
    Mem,
    Pid,
    Name,
}

impl SortKey {
    pub fn next(self) -> SortKey {
        match self {
            SortKey::Cpu => SortKey::Mem,
            SortKey::Mem => SortKey::Pid,
            SortKey::Pid => SortKey::Name,
            SortKey::Name => SortKey::Cpu,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Cpu => "cpu",
            SortKey::Mem => "mem",
            SortKey::Pid => "pid",
            SortKey::Name => "name",
        }
    }

    /// CPU and memory read best biggest-first; PID and name smallest-first.
    fn compare(self, a: &ProcInfo, b: &ProcInfo) -> Ordering {
        match self {
            SortKey::Cpu => b.cpu.total_cmp(&a.cpu).then(b.mem.cmp(&a.mem)),
            SortKey::Mem => b.mem.cmp(&a.mem).then(b.cpu.total_cmp(&a.cpu)),
            SortKey::Pid => a.pid.cmp(&b.pid),
            SortKey::Name => a
                .name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.pid.cmp(&b.pid)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sort {
    pub key: SortKey,
    pub reverse: bool,
}

impl Sort {
    fn cmp(&self, a: &ProcInfo, b: &ProcInfo) -> Ordering {
        let o = self.key.compare(a, b);
        if self.reverse {
            o.reverse()
        } else {
            o
        }
    }
}

/// Case-insensitive match on the name, the command line, the user, or an exact PID.
pub fn matches(p: &ProcInfo, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    p.name.to_lowercase().contains(&f)
        || p.cmd.to_lowercase().contains(&f)
        || p.user.to_lowercase() == f
        || p.pid.to_string() == f
}

/// A row of the process table: an index into the process list and its tree depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub idx: usize,
    pub depth: usize,
}

pub fn flat_rows(procs: &[ProcInfo], filter: &str, sort: Sort) -> Vec<Row> {
    let mut idx: Vec<usize> = (0..procs.len())
        .filter(|&i| matches(&procs[i], filter))
        .collect();
    idx.sort_by(|&a, &b| sort.cmp(&procs[a], &procs[b]));
    idx.into_iter().map(|idx| Row { idx, depth: 0 }).collect()
}

/// Rows in parent/child order. With a filter, each match is shown together
/// with its ancestors so it stays clear where it lives in the tree.
pub fn tree_rows(procs: &[ProcInfo], filter: &str, sort: Sort) -> Vec<Row> {
    let by_pid: HashMap<u32, usize> = procs.iter().enumerate().map(|(i, p)| (p.pid, i)).collect();

    let mut visible: HashSet<usize> = HashSet::new();
    for (i, p) in procs.iter().enumerate() {
        if !matches(p, filter) {
            continue;
        }
        let mut cur = Some(i);
        while let Some(c) = cur {
            if !visible.insert(c) {
                break;
            }
            cur = procs[c].ppid.and_then(|pp| by_pid.get(&pp).copied());
        }
    }

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut roots = Vec::new();
    for &i in &visible {
        match procs[i].ppid.and_then(|pp| by_pid.get(&pp).copied()) {
            Some(parent) if parent != i && visible.contains(&parent) => {
                children.entry(parent).or_default().push(i)
            }
            _ => roots.push(i),
        }
    }
    let by_sort = |v: &mut Vec<usize>| v.sort_by(|&a, &b| sort.cmp(&procs[a], &procs[b]));
    by_sort(&mut roots);
    for v in children.values_mut() {
        by_sort(v);
    }

    let mut out = Vec::with_capacity(visible.len());
    let mut stack: Vec<Row> = roots
        .into_iter()
        .rev()
        .map(|idx| Row { idx, depth: 0 })
        .collect();
    let mut done = HashSet::new();
    while let Some(row) = stack.pop() {
        if !done.insert(row.idx) {
            continue;
        }
        out.push(row);
        if let Some(kids) = children.get(&row.idx) {
            stack.extend(kids.iter().rev().map(|&idx| Row {
                idx,
                depth: row.depth + 1,
            }));
        }
    }
    out
}

/// `root` and every process below it.
pub fn descendants(procs: &[ProcInfo], root: u32) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in procs {
        if let Some(pp) = p.ppid {
            kids.entry(pp).or_default().push(p.pid);
        }
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = vec![root];
    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        out.push(pid);
        if let Some(k) = kids.get(&pid) {
            stack.extend(k);
        }
    }
    out
}

/// PIDs from `pid` up to the root, starting with `pid`'s parent.
pub fn ancestors(procs: &[ProcInfo], pid: u32) -> Vec<u32> {
    let parent: HashMap<u32, u32> = procs
        .iter()
        .filter_map(|p| Some((p.pid, p.ppid?)))
        .collect();
    let mut out = Vec::new();
    let mut cur = pid;
    while let Some(&pp) = parent.get(&cur) {
        if out.contains(&pp) || pp == 0 {
            break;
        }
        out.push(pp);
        cur = pp;
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppGroup {
    pub class: String,
    pub windows: Vec<Window>,
    /// Every process that belongs to the app's windows, including children.
    pub pids: Vec<u32>,
    pub cpu: f32,
    pub mem: u64,
}

/// Groups Hyprland windows by class and sums the resources of their processes.
pub fn group_apps(windows: &[Window], procs: &[ProcInfo]) -> Vec<AppGroup> {
    let by_pid: HashMap<u32, &ProcInfo> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut groups: Vec<AppGroup> = Vec::new();
    for w in windows {
        let g = match groups.iter_mut().position(|g| g.class == w.class) {
            Some(i) => &mut groups[i],
            None => {
                groups.push(AppGroup {
                    class: w.class.clone(),
                    windows: vec![],
                    pids: vec![],
                    cpu: 0.0,
                    mem: 0,
                });
                groups.last_mut().unwrap()
            }
        };
        g.windows.push(w.clone());
        for pid in descendants(procs, w.pid) {
            if !g.pids.contains(&pid) {
                g.pids.push(pid);
            }
        }
    }
    for g in &mut groups {
        g.pids.retain(|pid| by_pid.contains_key(pid));
        g.cpu = g.pids.iter().map(|p| by_pid[p].cpu).sum();
        g.mem = g.pids.iter().map(|p| by_pid[p].mem).sum();
    }
    groups.sort_by(|a, b| b.mem.cmp(&a.mem).then(a.class.cmp(&b.class)));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: Option<u32>, name: &str, cpu: f32, mem: u64) -> ProcInfo {
        ProcInfo {
            pid,
            ppid,
            name: name.into(),
            cmd: format!("/usr/bin/{name}"),
            user: "me".into(),
            cpu,
            mem,
            state: 'S',
            threads: 1,
        }
    }

    fn sample() -> Vec<ProcInfo> {
        vec![
            p(1, None, "systemd", 0.1, 10),
            p(100, Some(1), "foot", 1.0, 50),
            p(101, Some(100), "bash", 0.0, 5),
            p(102, Some(101), "vim", 5.0, 30),
            p(200, Some(1), "brave", 20.0, 900),
            p(201, Some(200), "brave-renderer", 30.0, 400),
        ]
    }

    const CPU: Sort = Sort {
        key: SortKey::Cpu,
        reverse: false,
    };

    #[test]
    fn flat_sorts_and_filters() {
        let procs = sample();
        let pids = |rows: Vec<Row>| rows.iter().map(|r| procs[r.idx].pid).collect::<Vec<_>>();
        assert_eq!(
            pids(flat_rows(&procs, "", CPU)),
            vec![201, 200, 102, 100, 1, 101]
        );
        assert_eq!(
            pids(flat_rows(
                &procs,
                "",
                Sort {
                    key: SortKey::Mem,
                    reverse: true
                }
            )),
            vec![101, 1, 102, 100, 201, 200]
        );
        assert_eq!(pids(flat_rows(&procs, "BRAVE", CPU)), vec![201, 200]);
        assert_eq!(pids(flat_rows(&procs, "102", CPU)), vec![102]);
    }

    #[test]
    fn tree_orders_children_under_parents() {
        let procs = sample();
        let rows: Vec<(u32, usize)> = tree_rows(
            &procs,
            "",
            Sort {
                key: SortKey::Pid,
                reverse: false,
            },
        )
        .iter()
        .map(|r| (procs[r.idx].pid, r.depth))
        .collect();
        assert_eq!(
            rows,
            vec![(1, 0), (100, 1), (101, 2), (102, 3), (200, 1), (201, 2)]
        );
    }

    #[test]
    fn tree_filter_keeps_ancestors() {
        let procs = sample();
        let rows: Vec<u32> = tree_rows(&procs, "vim", CPU)
            .iter()
            .map(|r| procs[r.idx].pid)
            .collect();
        assert_eq!(rows, vec![1, 100, 101, 102]);
    }

    #[test]
    fn descendants_and_ancestors() {
        let procs = sample();
        let mut d = descendants(&procs, 100);
        d.sort();
        assert_eq!(d, vec![100, 101, 102]);
        assert_eq!(ancestors(&procs, 102), vec![101, 100, 1]);
    }

    #[test]
    fn groups_apps_by_class() {
        let procs = sample();
        let w = |pid, class: &str| Window {
            address: format!("0x{pid}"),
            pid,
            class: class.into(),
            title: "t".into(),
            workspace: "1".into(),
        };
        let groups = group_apps(&[w(100, "foot"), w(200, "brave"), w(200, "brave")], &procs);
        assert_eq!(groups[0].class, "brave");
        assert_eq!(groups[0].windows.len(), 2);
        assert_eq!(groups[0].mem, 1300);
        assert_eq!(groups[0].cpu, 50.0);
        assert_eq!(groups[1].class, "foot");
        let mut pids = groups[1].pids.clone();
        pids.sort();
        assert_eq!(pids, vec![100, 101, 102]);
    }
}
