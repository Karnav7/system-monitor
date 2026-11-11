//! Open windows, read from Hyprland.

use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub address: String,
    pub pid: u32,
    pub class: String,
    pub title: String,
    pub workspace: String,
}

#[derive(Deserialize)]
struct Workspace {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Client {
    address: String,
    pid: i64,
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
    workspace: Option<Workspace>,
    #[serde(default = "yes")]
    mapped: bool,
}

fn yes() -> bool {
    true
}

pub fn parse_clients(json: &str) -> Result<Vec<Window>, String> {
    let clients: Vec<Client> =
        serde_json::from_str(json).map_err(|e| format!("bad hyprctl output: {e}"))?;
    Ok(clients
        .into_iter()
        .filter(|c| c.mapped && c.pid > 0)
        .map(|c| Window {
            address: c.address,
            pid: c.pid as u32,
            class: if c.class.is_empty() {
                "(unknown)".into()
            } else {
                c.class
            },
            title: c.title,
            workspace: c.workspace.map(|w| w.name).unwrap_or_default(),
        })
        .collect())
}

/// Lists Hyprland's windows. Errors are short messages for the status line.
pub fn clients() -> Result<Vec<Window>, String> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return Err("Hyprland is not running, so the Apps view is empty".into());
    }
    let out = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .map_err(|e| format!("could not run hyprctl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "hyprctl failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_clients(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture() {
        let json = include_str!("../../tests/fixtures/hyprctl_clients.json");
        let w = parse_clients(json).unwrap();
        assert_eq!(w.len(), 2, "unmapped window is skipped");
        assert_eq!(w[0].class, "brave-browser");
        assert_eq!(w[0].pid, 5425);
        assert_eq!(w[0].workspace, "11");
        assert_eq!(w[1].class, "(unknown)");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_clients("not json").is_err());
    }
}
