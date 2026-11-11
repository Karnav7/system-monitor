//! Stopping processes and closing windows.

use std::process::Command;

use nix::errno::Errno;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    Term,
    Kill,
}

impl Sig {
    pub fn name(self) -> &'static str {
        match self {
            Sig::Term => "SIGTERM",
            Sig::Kill => "SIGKILL",
        }
    }
}

/// Why a PID is never signalled, or None when it may be.
pub fn refusal(pid: u32) -> Option<&'static str> {
    if pid <= 1 {
        Some("refusing to signal PID 1 (init); that would take down the whole system")
    } else if pid == std::process::id() {
        Some("refusing to signal System Monitor itself; press q to quit")
    } else {
        None
    }
}

pub fn signal(pid: u32, sig: Sig) -> Result<(), String> {
    if let Some(why) = refusal(pid) {
        return Err(why.into());
    }
    let s = match sig {
        Sig::Term => Signal::SIGTERM,
        Sig::Kill => Signal::SIGKILL,
    };
    kill(Pid::from_raw(pid as i32), s).map_err(|e| match e {
        Errno::EPERM => format!("permission denied for PID {pid}; it belongs to another user (run system-monitor with sudo to stop it)"),
        Errno::ESRCH => format!("PID {pid} has already exited"),
        e => format!("could not signal PID {pid}: {e}"),
    })
}

/// Asks Hyprland to close a window the same way Super+W does, so the app can
/// save its work or ask before quitting.
pub fn close_window(address: &str) -> Result<(), String> {
    // Hyprland 0.55+ (Omarchy 4) takes Lua for dispatch; older releases take
    // the classic "closewindow address:…" form. Try the new one first.
    let lua = format!("hl.dsp.window.close({{ window = \"address:{address}\" }})");
    let legacy = format!("address:{address}");
    let err = match dispatch(&[&lua]) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    dispatch(&["closewindow", &legacy])
        .map_err(|_| format!("hyprctl could not close the window: {err}"))
}

fn dispatch(args: &[&str]) -> Result<(), String> {
    let out = Command::new("hyprctl")
        .arg("dispatch")
        .args(args)
        .output()
        .map_err(|e| format!("could not run hyprctl: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && text == "ok" {
        Ok(())
    } else {
        Err(text.lines().next().unwrap_or("no output").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_init_and_self() {
        assert!(refusal(0).is_some());
        assert!(refusal(1).is_some());
        assert!(refusal(std::process::id()).is_some());
        assert!(signal(1, Sig::Term).is_err());
    }

    #[test]
    fn terminates_a_child() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        signal(child.id(), Sig::Term).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    #[test]
    fn reports_missing_process() {
        let mut child = Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        let err = signal(pid, Sig::Term).unwrap_err();
        assert!(err.contains("already exited"), "{err}");
    }
}
