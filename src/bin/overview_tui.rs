//! Local board: same paint as the plugin, no Zellij.
//!
//! Replay a scene without a TTY:
//!   cargo run --bin overview-tui -- --replay e2e/scenes/pin-partial-open.scene
//!
//! Interactive (crossterm):
//!   cargo run --features tui --bin overview-tui
//!   cargo run --features tui --bin overview-tui -- e2e/scenes/pin-partial-open.scene

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use zellij_overview::{run_scene, Host};

const DEMO: &str = r#"
session ww current notes logs
session lp git feat
tabs notes* logs
"#;

const BOARD_ROWS: usize = 16;
const BOARD_COLS: usize = 80;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut rest = args.as_slice();
    match rest.first().map(String::as_str) {
        Some("-h" | "--help") => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some("--replay") => {
            rest = &rest[1..];
            let Some(path) = rest.first() else {
                eprintln!("overview-tui --replay FILE");
                return ExitCode::from(2);
            };
            let paint = rest.iter().any(|arg| arg == "--paint");
            replay(Path::new(path), paint)
        }
        Some("--scene") => {
            rest = &rest[1..];
            let Some(path) = rest.first() else {
                eprintln!("overview-tui --scene FILE pin TITLE");
                return ExitCode::from(2);
            };
            match load_scene(Path::new(path)) {
                Ok(host) => run_cli_commands(host, &rest[1..]),
                Err(code) => code,
            }
        }
        Some("pin" | "focus" | "jump") => run_cli_commands(load_demo(), rest),
        Some(path) => match load_scene(Path::new(path)) {
            Ok(host) => interactive(host),
            Err(code) => code,
        },
        None => interactive(load_demo()),
    }
}

fn print_usage() {
    eprintln!(
        "\
overview-tui — board and pin cache without Zellij

  overview-tui                         demo board
  overview-tui FILE                    load a scene, then play
  overview-tui pin TITLE               pin a tab (same as p)
  overview-tui focus TITLE             select a card
  overview-tui jump                    press e on the focused card
  overview-tui --scene FILE pin TITLE  load a scene, then pin / focus / jump
  overview-tui --replay FILE [--paint] run expects; no TTY
"
    );
}

fn run_cli_commands(mut host: Host, args: &[String]) -> ExitCode {
    let mut words = args.iter().map(String::as_str);
    while let Some(verb) = words.next() {
        match verb {
            "pin" => {
                let Some(spec) = words.next() else {
                    eprintln!("overview-tui pin TITLE");
                    return ExitCode::from(2);
                };
                if let Err(err) = host.pin(spec) {
                    eprintln!("{err}");
                    return ExitCode::FAILURE;
                }
            }
            "focus" => {
                let Some(spec) = words.next() else {
                    eprintln!("overview-tui focus TITLE");
                    return ExitCode::from(2);
                };
                if let Err(err) = host.focus(spec) {
                    eprintln!("{err}");
                    return ExitCode::FAILURE;
                }
            }
            "jump" => {
                host.jump();
            }
            other => {
                eprintln!("unknown {other}");
                return ExitCode::from(2);
            }
        }
    }
    print_report(&host);
    ExitCode::SUCCESS
}

fn replay(path: &Path, paint: bool) -> ExitCode {
    match load_scene(path) {
        Ok(host) => {
            println!("ok {}", path.display());
            print_report(&host);
            if paint {
                dump_paint(&host, BOARD_ROWS, BOARD_COLS);
            }
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

fn load_scene(path: &Path) -> Result<Host, ExitCode> {
    let source = fs::read_to_string(path).map_err(|err| {
        eprintln!("{}: {err}", path.display());
        ExitCode::from(2)
    })?;
    run_scene(&source).map_err(|err| {
        eprintln!("{}: {err}", path.display());
        ExitCode::FAILURE
    })
}

fn load_demo() -> Host {
    run_scene(DEMO).expect("demo scene")
}

fn print_report(host: &Host) {
    print!("pins:");
    if host.persisted_pins().is_empty() {
        print!(" none");
    } else {
        for pin in host.persisted_pins() {
            print!(" {}/{}", pin.session, pin.tab_name);
        }
    }
    println!();
    print!("focus:");
    match host.focused_title() {
        Some(title) => print!(" {title}"),
        None => print!(" none"),
    }
    println!();
    println!("action: {:?}", host.last_action());
    print!("titles:");
    for index in 0..host.overview.item_count() {
        if let Some(title) = host.overview.item_title(index) {
            print!(" {title}");
        }
    }
    println!();
}

fn dump_paint(host: &Host, rows: usize, cols: usize) {
    for line in host.overview.paint(rows, cols).lines {
        println!("{line}");
    }
}

fn interactive(mut host: Host) -> ExitCode {
    #[cfg(not(feature = "tui"))]
    {
        let _ = &mut host;
        eprintln!("rebuild with --features tui for the interactive board");
        eprintln!("  cargo run --features tui --bin overview-tui");
        print_report(&host);
        dump_paint(&host, BOARD_ROWS, BOARD_COLS);
        ExitCode::from(2)
    }

    #[cfg(feature = "tui")]
    match play(&mut host) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "tui")]
fn play(host: &mut Host) -> std::io::Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use crossterm::{cursor, execute, terminal};
    use zellij_overview::Action;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let result = (|| {
        let (mut cols, mut rows) = terminal::size()?;
        loop {
            draw(host, &mut stdout, rows, cols)?;
            match event::read()? {
                Event::Resize(next_cols, next_rows) => {
                    cols = next_cols;
                    rows = next_rows;
                }
                Event::Key(key) if key.kind != KeyEventKind::Repeat => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    let Some(mapped) = map_key(key, host.overview.is_hinting()) else {
                        continue;
                    };
                    match host.key(mapped) {
                        Action::Dismiss
                        | Action::Commit { .. }
                        | Action::SwitchSession { .. }
                        | Action::PreviousTab => break,
                        Action::None | Action::PersistPins => {}
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })();
    execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

#[cfg(feature = "tui")]
fn draw(
    host: &mut Host,
    stdout: &mut std::io::Stdout,
    rows: u16,
    cols: u16,
) -> std::io::Result<()> {
    use crossterm::{cursor, execute, terminal};
    use std::io::Write;

    host.overview.set_viewport(rows as usize, cols as usize);
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    for line in host.overview.paint(rows as usize, cols as usize).lines {
        writeln!(stdout, "{line}")?;
    }
    stdout.flush()
}

#[cfg(feature = "tui")]
fn map_key(key: crossterm::event::KeyEvent, hinting: bool) -> Option<zellij_overview::Key> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use zellij_overview::Key;

    if key.modifiers.contains(KeyModifiers::CONTROL) && !hinting {
        return match key.code {
            KeyCode::Char('d') => Some(Key::HalfPageDown),
            KeyCode::Char('u') => Some(Key::HalfPageUp),
            KeyCode::Char('f') => Some(Key::PageDown),
            KeyCode::Char('b') => Some(Key::PageUp),
            _ => None,
        };
    }
    if !key.modifiers.is_empty()
        && key.modifiers != KeyModifiers::NONE
        && key.modifiers != KeyModifiers::SHIFT
    {
        return None;
    }
    match key.code {
        KeyCode::Left => Some(Key::Left),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Right => Some(Key::Right),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::Enter if !hinting => Some(Key::Confirm),
        KeyCode::Esc => Some(Key::Dismiss),
        KeyCode::Backspace if hinting => Some(Key::Backspace),
        KeyCode::Char('s') if !hinting => Some(Key::StartHint),
        KeyCode::Char(c) if hinting => Some(Key::Input(c)),
        KeyCode::Char('z') => Some(Key::ZPrefix),
        KeyCode::Char('t') => Some(Key::AlignTop),
        KeyCode::Char('b') => Some(Key::AlignBottom),
        KeyCode::Char('g') => Some(Key::GoPrefix),
        KeyCode::Char('G') => Some(Key::Last),
        KeyCode::Char('?') => Some(Key::ToggleHelp),
        KeyCode::Char('q') => Some(Key::Dismiss),
        KeyCode::Char('h') => Some(Key::Left),
        KeyCode::Char('j') => Some(Key::Down),
        KeyCode::Char('k') => Some(Key::Up),
        KeyCode::Char('l') => Some(Key::Right),
        KeyCode::Char('e') => Some(Key::Confirm),
        KeyCode::Char('p') if !hinting => Some(Key::Pin),
        KeyCode::Char('-') => Some(Key::PreviousTab),
        _ => None,
    }
}
