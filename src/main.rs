mod actions;
mod app;
mod collect;
mod input;
mod procs;
mod theme;
mod ui;

use std::time::Instant;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use app::App;
use collect::Collector;
use theme::ThemeWatcher;

const USAGE: &str = "System Monitor: a terminal task manager for Omarchy

Usage: system-monitor [--help | --version]

Press ? inside System Monitor to see every key.";

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("-h" | "--help") => {
            println!("{USAGE}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("system-monitor {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(other) => {
            eprintln!("system-monitor: unknown argument '{other}'\n\n{USAGE}");
            std::process::exit(2);
        }
    }

    let mut app = App::new(
        Collector::new(),
        ThemeWatcher::new(ThemeWatcher::default_path()),
    );

    // ratatui::init switches to the alternate screen and installs a panic
    // hook that restores the terminal, so a crash never leaves it garbled.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    let mut next_tick = Instant::now() + app.tick;
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = next_tick.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => input::handle(app, key),
                _ => {}
            }
        }

        if app.refresh_soon {
            app.refresh_soon = false;
            next_tick = next_tick.min(Instant::now() + std::time::Duration::from_millis(200));
        }
        if Instant::now() >= next_tick {
            app.refresh();
            next_tick = Instant::now() + app.tick;
        }
    }
    Ok(())
}
