//! Maps keys to actions, depending on the current mode.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Sig;
use crate::app::{App, Mode, Tab};

const PAGE: isize = 10;

pub fn handle(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }
    match &app.mode {
        Mode::Help => app.mode = Mode::Normal,
        Mode::Confirm(_) => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm(),
            _ => {
                app.mode = Mode::Normal;
                app.set_status("Cancelled", false);
            }
        },
        Mode::Filter => filter_key(app, key),
        Mode::Normal => normal_key(app, key),
    }
}

fn filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.filter.clear();
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => app.mode = Mode::Normal,
        KeyCode::Backspace => {
            app.filter.pop();
        }
        KeyCode::Up => app.move_selection(-1),
        KeyCode::Down => app.move_selection(1),
        KeyCode::Char(c) => app.filter.push(c),
        _ => return,
    }
    app.rebuild();
}

fn normal_key(app: &mut App, key: KeyEvent) {
    // Alt+key chords belong to the terminal or Hyprland, not to System Monitor.
    if key.modifiers.contains(KeyModifiers::ALT) {
        return;
    }
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Tab => app.tab = Tab::ALL[(app.tab.index() + 1) % Tab::ALL.len()],
        KeyCode::BackTab => {
            app.tab = Tab::ALL[(app.tab.index() + Tab::ALL.len() - 1) % Tab::ALL.len()]
        }
        KeyCode::Char('1') => app.tab = Tab::Overview,
        KeyCode::Char('2') => app.tab = Tab::Processes,
        KeyCode::Char('3') => app.tab = Tab::Apps,
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::PageDown => app.move_selection(PAGE),
        KeyCode::PageUp => app.move_selection(-PAGE),
        KeyCode::Char('g') | KeyCode::Home => app.move_selection(isize::MIN / 2),
        KeyCode::Char('G') | KeyCode::End => app.move_selection(isize::MAX / 2),
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('r') => app.reverse_sort(),
        KeyCode::Char('t') => app.toggle_tree(),
        KeyCode::Char('/') => {
            if app.tab == Tab::Overview {
                app.tab = Tab::Processes;
            }
            app.mode = Mode::Filter;
        }
        KeyCode::Esc if !app.filter.is_empty() => {
            app.filter.clear();
            app.rebuild();
        }
        KeyCode::Char('x') | KeyCode::Delete => app.request_stop(Sig::Term),
        KeyCode::Char('X') => app.request_stop(Sig::Kill),
        KeyCode::Char('+') | KeyCode::Char('=') => app.faster(),
        KeyCode::Char('-') => app.slower(),
        _ => {}
    }
}
