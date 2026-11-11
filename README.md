# System Monitor

A terminal task manager for [Omarchy](https://omarchy.org). It shows running processes, open Hyprland apps and live system metrics, and lets you stop any process or app from the keyboard. Colors follow the active Omarchy theme and update live when you switch themes.

## What it shows

- **Overview**: total CPU history and a bar for each core, RAM and swap, AMD GPU busy % and VRAM, temperature sensors, network throughput for each interface, and disk usage plus read/write rates.
- **Processes**: a sortable, filterable process table with an optional tree view.
- **Apps**: Hyprland windows grouped by app, with the combined CPU and memory of every process each app owns.

## Install

```sh
omarchy-pkg-add rust                          # once, if cargo isn't installed
cargo install --path . --root ~/.local        # puts system-monitor in ~/.local/bin (already on PATH)
omarchy-tui-install "System Monitor" ~/.local/bin/system-monitor tile utilities-system-monitor   # adds it to the app launcher
```

Re-run the `cargo install` line after changing the code.

Then open it with Super+Space → System Monitor, or run `system-monitor` in a terminal.

## Keys

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab`, `1` `2` `3` | switch tab |
| `j` `k` / `↓` `↑`, `PgDn` `PgUp`, `g` `G` | move the selection |
| `/` | filter processes by name, command, user or PID (`Enter` keeps it, `Esc` clears it) |
| `s` / `r` | cycle the sort column (cpu, mem, pid, name) / reverse it |
| `t` | toggle the process tree |
| `x` / `Del` | stop the process with SIGTERM; in Apps, close the app's windows the way Super+W does |
| `X` | force kill with SIGKILL; in Apps, kill every process the app owns |
| `+` / `-` | refresh faster / slower (250 ms to 5 s) |
| `?` | show every key |
| `q` / `Ctrl+C` | quit |

Every stop or kill asks for confirmation first. System Monitor won't signal PID 1 or itself, and it warns you when the target is one of its own parent processes, such as the terminal it runs in. Processes owned by other users need `sudo system-monitor`; otherwise you get a permission error in the status line.

## Development

```sh
cargo test
cargo clippy -- -D warnings
cargo run
```
