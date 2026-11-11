//! Application state and the actions the keys trigger.

use std::time::{Duration, Instant};

use ratatui::widgets::TableState;

use crate::actions::{self, Sig};
use crate::collect::Collector;
use crate::procs::{self, AppGroup, Row, Sort, SortKey};
use crate::theme::ThemeWatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Processes,
    Apps,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Overview, Tab::Processes, Tab::Apps];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Processes => "Processes",
            Tab::Apps => "Apps",
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    Signal {
        pids: Vec<u32>,
        sig: Sig,
        what: String,
        warning: Option<String>,
    },
    CloseWindows {
        addresses: Vec<String>,
        what: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Filter,
    Help,
    Confirm(Pending),
}

pub struct Status {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

pub const MIN_TICK: Duration = Duration::from_millis(250);
pub const MAX_TICK: Duration = Duration::from_secs(5);

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub sort: Sort,
    pub tree: bool,
    pub filter: String,
    pub tick: Duration,
    pub quit: bool,
    /// Set after an action so the list refreshes soon instead of at the next tick.
    pub refresh_soon: bool,
    pub status: Option<Status>,
    pub collector: Collector,
    pub theme: ThemeWatcher,
    pub rows: Vec<Row>,
    pub groups: Vec<AppGroup>,
    pub proc_state: TableState,
    pub app_state: TableState,
    /// The selected process, so the selection follows it when the list re-sorts.
    selected_pid: Option<u32>,
    selected_class: Option<String>,
    /// System Monitor's own ancestors (shell, terminal…). Stopping one of them closes System Monitor too.
    self_ancestors: Vec<u32>,
}

impl App {
    pub fn new(collector: Collector, theme: ThemeWatcher) -> Self {
        let mut app = App {
            tab: Tab::Overview,
            mode: Mode::Normal,
            sort: Sort {
                key: SortKey::Cpu,
                reverse: false,
            },
            tree: false,
            filter: String::new(),
            tick: Duration::from_secs(1),
            quit: false,
            refresh_soon: false,
            status: None,
            collector,
            theme,
            rows: Vec::new(),
            groups: Vec::new(),
            proc_state: TableState::default().with_selected(Some(0)),
            app_state: TableState::default().with_selected(Some(0)),
            selected_pid: None,
            selected_class: None,
            self_ancestors: Vec::new(),
        };
        app.rebuild();
        app
    }

    pub fn refresh(&mut self) {
        self.collector.refresh();
        self.theme.poll();
        self.rebuild();
    }

    /// Recomputes the visible rows and keeps the selection on the same process or app.
    pub fn rebuild(&mut self) {
        let snap = &self.collector.snap;
        self.self_ancestors = procs::ancestors(&snap.procs, std::process::id());
        self.rows = if self.tree {
            procs::tree_rows(&snap.procs, &self.filter, self.sort)
        } else {
            procs::flat_rows(&snap.procs, &self.filter, self.sort)
        };
        self.groups = procs::group_apps(&snap.windows, &snap.procs);

        let pid_pos = self
            .selected_pid
            .and_then(|pid| self.rows.iter().position(|r| snap.procs[r.idx].pid == pid));
        let sel = pid_pos
            .or(self.proc_state.selected())
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
        self.proc_state.select(Some(sel));
        self.selected_pid = self.rows.get(sel).map(|r| snap.procs[r.idx].pid);

        let class_pos = self
            .selected_class
            .as_ref()
            .and_then(|c| self.groups.iter().position(|g| &g.class == c));
        let sel = class_pos
            .or(self.app_state.selected())
            .unwrap_or(0)
            .min(self.groups.len().saturating_sub(1));
        self.app_state.select(Some(sel));
        self.selected_class = self.groups.get(sel).map(|g| g.class.clone());
    }

    pub fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(Status {
            text: text.into(),
            error,
            at: Instant::now(),
        });
    }

    /// Moves the selection on the current tab by `delta` rows (clamped).
    pub fn move_selection(&mut self, delta: isize) {
        let (state, len) = match self.tab {
            Tab::Processes => (&mut self.proc_state, self.rows.len()),
            Tab::Apps => (&mut self.app_state, self.groups.len()),
            Tab::Overview => return,
        };
        if len == 0 {
            return;
        }
        let cur = state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, len as isize - 1) as usize;
        state.select(Some(next));
        match self.tab {
            Tab::Processes => {
                self.selected_pid = Some(self.collector.snap.procs[self.rows[next].idx].pid)
            }
            Tab::Apps => self.selected_class = Some(self.groups[next].class.clone()),
            Tab::Overview => {}
        }
    }

    pub fn selected_proc(&self) -> Option<&crate::collect::ProcInfo> {
        let row = self.rows.get(self.proc_state.selected()?)?;
        self.collector.snap.procs.get(row.idx)
    }

    pub fn selected_group(&self) -> Option<&AppGroup> {
        self.groups.get(self.app_state.selected()?)
    }

    /// Opens the confirmation for stopping whatever is selected.
    pub fn request_stop(&mut self, sig: Sig) {
        match self.tab {
            Tab::Overview => {}
            Tab::Processes => {
                let Some(p) = self.selected_proc() else {
                    return;
                };
                if let Some(why) = actions::refusal(p.pid) {
                    return self.set_status(why, true);
                }
                let what = format!("{} (PID {}, {})", p.name, p.pid, p.user);
                let warning = self.self_ancestors.contains(&p.pid).then(|| {
                    "System Monitor runs inside this process, so System Monitor will close too."
                        .to_string()
                });
                self.mode = Mode::Confirm(Pending::Signal {
                    pids: vec![p.pid],
                    sig,
                    what,
                    warning,
                });
            }
            Tab::Apps => {
                let Some(g) = self.selected_group() else {
                    return;
                };
                let me = std::process::id();
                if g.pids.contains(&me) {
                    return self.set_status(
                        format!(
                            "{} is the terminal System Monitor runs in; close it from Hyprland instead",
                            g.class
                        ),
                        true,
                    );
                }
                let what = format!(
                    "{} ({} windows, {} processes)",
                    g.class,
                    g.windows.len(),
                    g.pids.len()
                );
                self.mode = Mode::Confirm(match sig {
                    Sig::Term => Pending::CloseWindows {
                        addresses: g.windows.iter().map(|w| w.address.clone()).collect(),
                        what,
                    },
                    Sig::Kill => Pending::Signal {
                        pids: g.pids.clone(),
                        sig,
                        what,
                        warning: None,
                    },
                });
            }
        }
    }

    pub fn confirm(&mut self) {
        let Mode::Confirm(pending) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        let (errors, done, what) = match pending {
            Pending::Signal {
                pids, sig, what, ..
            } => {
                let errors: Vec<String> = pids
                    .iter()
                    .filter_map(|&pid| actions::signal(pid, sig).err())
                    .collect();
                (errors, format!("Sent {} to", sig.name()), what)
            }
            Pending::CloseWindows { addresses, what } => {
                let errors: Vec<String> = addresses
                    .iter()
                    .filter_map(|a| actions::close_window(a).err())
                    .collect();
                (errors, "Asked to close".to_string(), what)
            }
        };
        match errors.first() {
            None => self.set_status(format!("{done} {what}"), false),
            Some(e) if errors.len() == 1 => self.set_status(e.clone(), true),
            Some(e) => self.set_status(format!("{e} (and {} more errors)", errors.len() - 1), true),
        }
        self.refresh_soon = true;
    }

    pub fn cycle_sort(&mut self) {
        self.sort.key = self.sort.key.next();
        self.sort.reverse = false;
        self.rebuild();
    }

    pub fn reverse_sort(&mut self) {
        self.sort.reverse = !self.sort.reverse;
        self.rebuild();
    }

    pub fn toggle_tree(&mut self) {
        self.tree = !self.tree;
        self.rebuild();
    }

    pub fn faster(&mut self) {
        self.tick = (self.tick / 2).max(MIN_TICK);
        self.set_status(
            format!("Refreshing every {} ms", self.tick.as_millis()),
            false,
        );
    }

    pub fn slower(&mut self) {
        self.tick = (self.tick * 2).min(MAX_TICK);
        self.set_status(
            format!("Refreshing every {} ms", self.tick.as_millis()),
            false,
        );
    }
}
