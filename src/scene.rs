use crate::host::Host;
use crate::{Action, Key, Pin, SessionFact, TabFact};

#[derive(Debug)]
pub struct SceneError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for SceneError {}

/// Line-oriented board scene. You write the expected board and pin file.
pub fn run_scene(source: &str) -> Result<Host, SceneError> {
    let mut host = Host::new();
    let mut rows = 12;
    let mut cols = 80;
    let mut last_action = Action::None;
    let mut pending_sessions: Vec<SessionFact> = Vec::new();
    let mut in_snapshot = false;

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let verb = words.next().unwrap_or("");
        let args: Vec<&str> = words.collect();
        if verb != "session" {
            flush_sessions(&mut host, &mut pending_sessions, in_snapshot);
            in_snapshot = false;
        }
        match verb {
            "size" => {
                rows = parse_usize(args.first(), line_no, "rows")?;
                cols = parse_usize(args.get(1), line_no, "cols")?;
            }
            "snapshot" => {
                in_snapshot = true;
            }
            "session" => {
                pending_sessions.push(parse_session(&args, line_no)?);
            }
            "pin" => {
                let spec = card_spec(&args, line_no, "pin")?;
                last_action = host.pin(&spec).map_err(|message| SceneError {
                    line: line_no,
                    message,
                })?;
            }
            "focus" => {
                let spec = card_spec(&args, line_no, "focus")?;
                host.focus(&spec).map_err(|message| SceneError {
                    line: line_no,
                    message,
                })?;
            }
            "jump" => {
                last_action = host.jump();
            }
            "tabs" => {
                host.apply_tabs(parse_tabs(&args, line_no)?);
            }
            "key" => {
                last_action = host.key(parse_key(&args, line_no)?);
            }
            "previous-session" => {
                let name = args.first().copied().ok_or_else(|| SceneError {
                    line: line_no,
                    message: "previous-session needs a name".into(),
                })?;
                host.overview
                    .set_previous_session_name(Some((*name).to_owned()));
            }
            "session-last" => {
                let session = args.first().copied().ok_or_else(|| SceneError {
                    line: line_no,
                    message: "session-last needs session position".into(),
                })?;
                let position = parse_usize(args.get(1), line_no, "position")?;
                host.overview
                    .set_session_last_tab((*session).to_owned(), position);
            }
            "previous-tab" => {
                let id = parse_usize(args.first(), line_no, "tab id")?;
                host.overview.set_previous_tab_id(Some(id));
            }
            "expect" => {}
            "pins" => expect_pins(&host, &args, line_no)?,
            "titles" => expect_titles(&host, &args, line_no)?,
            "focused" => expect_focused(&host, &args, line_no)?,
            "viewing" => expect_viewing(&host, &args, line_no)?,
            "previous" => expect_previous(&host, &args, line_no)?,
            "action" => expect_action(&last_action, &args, line_no)?,
            "screen" => {
                host.overview.set_viewport(rows, cols);
                let joined = host.overview.paint(rows, cols).lines.join("\n");
                let needle = args.join(" ");
                if !joined.contains(&needle) {
                    return Err(SceneError {
                        line: line_no,
                        message: format!("screen missing {needle:?}"),
                    });
                }
            }
            other => {
                return Err(SceneError {
                    line: line_no,
                    message: format!("unknown {other}"),
                });
            }
        }
    }
    flush_sessions(&mut host, &mut pending_sessions, in_snapshot);
    host.overview.set_viewport(rows, cols);
    Ok(host)
}

fn flush_sessions(host: &mut Host, pending: &mut Vec<SessionFact>, in_snapshot: bool) {
    if pending.is_empty() {
        return;
    }
    let sessions = std::mem::take(pending);
    if in_snapshot {
        host.load_from_snapshot(sessions);
    } else {
        host.apply_sessions(sessions);
    }
}

fn parse_session(args: &[&str], line: usize) -> Result<SessionFact, SceneError> {
    let name = args
        .first()
        .copied()
        .ok_or_else(|| SceneError {
            line,
            message: "session needs a name".into(),
        })?
        .to_owned();
    let current = args.contains(&"current");
    let tab_count = args
        .iter()
        .find_map(|word| word.strip_prefix("count=")?.parse().ok());
    let tabs = args
        .iter()
        .skip(1)
        .filter(|word| **word != "current" && !word.starts_with("count="))
        .enumerate()
        .map(|(position, name)| tab_from_token(100 + position, position, name))
        .collect::<Vec<_>>();
    Ok(SessionFact {
        name,
        current,
        tab_count: tab_count.unwrap_or(tabs.len()),
        tabs,
    })
}

fn parse_tabs(args: &[&str], line: usize) -> Result<Vec<TabFact>, SceneError> {
    if args.is_empty() {
        return Err(SceneError {
            line,
            message: "tabs needs at least one name".into(),
        });
    }
    Ok(args
        .iter()
        .enumerate()
        .map(|(position, name)| tab_from_token(position, position, name))
        .collect())
}

fn tab_from_token(default_id: usize, position: usize, token: &str) -> TabFact {
    let active = token.ends_with('*');
    let token = token.trim_end_matches('*');
    let (id, name) = match token.strip_prefix("id=") {
        Some(rest) => match rest.split_once(':') {
            Some((id, name)) => (id.parse().unwrap_or(default_id), name),
            None => (default_id, rest),
        },
        None => (default_id, token),
    };
    TabFact {
        id,
        position,
        name: name.to_owned(),
        active,
    }
}

fn card_spec(args: &[&str], line: usize, verb: &str) -> Result<String, SceneError> {
    match args {
        [raw] => Ok((*raw).to_owned()),
        [session, tab] => Ok(format!("{session}/{tab}")),
        _ => Err(SceneError {
            line,
            message: format!("{verb} wants a title (or session/tab)"),
        }),
    }
}

fn parse_key(args: &[&str], line: usize) -> Result<Key, SceneError> {
    match args.first().copied() {
        Some("s") => Ok(Key::StartHint),
        Some("e") | Some("enter") => Ok(Key::Confirm),
        Some("esc") | Some("q") => Ok(Key::Dismiss),
        Some("p") => Ok(Key::Pin),
        Some("-") => Ok(Key::PreviousTab),
        Some("h") | Some("left") => Ok(Key::Left),
        Some("j") | Some("down") => Ok(Key::Down),
        Some("k") | Some("up") => Ok(Key::Up),
        Some("l") | Some("right") => Ok(Key::Right),
        Some("g") => Ok(Key::GoPrefix),
        Some("G") => Ok(Key::Last),
        Some("?") => Ok(Key::ToggleHelp),
        Some("backspace") => Ok(Key::Backspace),
        Some("input") => {
            let ch = args
                .get(1)
                .and_then(|word| word.chars().next())
                .ok_or_else(|| SceneError {
                    line,
                    message: "key input needs a character".into(),
                })?;
            Ok(Key::Input(ch))
        }
        Some(other) => Err(SceneError {
            line,
            message: format!("unknown key {other}"),
        }),
        None => Err(SceneError {
            line,
            message: "key needs a name".into(),
        }),
    }
}

fn expect_pins(host: &Host, args: &[&str], line: usize) -> Result<(), SceneError> {
    let expected: Vec<Pin> = if args.first() == Some(&"none") {
        Vec::new()
    } else {
        args.iter()
            .map(|raw| {
                let (session, tab_name) = raw.split_once('/').ok_or_else(|| SceneError {
                    line,
                    message: "pins want session/tab".into(),
                })?;
                Ok(Pin {
                    session: session.to_owned(),
                    tab_name: tab_name.to_owned(),
                })
            })
            .collect::<Result<_, _>>()?
    };
    let actual = host.persisted_pins();
    if actual != expected.as_slice() {
        return Err(SceneError {
            line,
            message: format!("pins {actual:?} != {expected:?}"),
        });
    }
    Ok(())
}

fn expect_focused(host: &Host, args: &[&str], line: usize) -> Result<(), SceneError> {
    let spec = card_spec(args, line, "focused")?;
    let (session, title) = spec
        .split_once('/')
        .map(|(session, title)| (Some(session), title))
        .unwrap_or((None, spec.as_str()));
    let actual = host.focused_title().unwrap_or("");
    if actual != title {
        return Err(SceneError {
            line,
            message: format!("focused {actual:?} != {title:?}"),
        });
    }
    if let Some(session) = session {
        let got = host.overview.item_session_name(host.overview.cursor());
        if got != Some(session) {
            return Err(SceneError {
                line,
                message: format!("focused session {got:?} != {session:?}"),
            });
        }
    }
    Ok(())
}

fn expect_viewing(host: &Host, args: &[&str], line: usize) -> Result<(), SceneError> {
    let expected = args.first().copied().unwrap_or("none");
    let actual = host.overview.viewing_session().unwrap_or("none");
    if actual != expected {
        return Err(SceneError {
            line,
            message: format!("viewing {actual:?} != {expected:?}"),
        });
    }
    Ok(())
}

fn expect_previous(host: &Host, args: &[&str], line: usize) -> Result<(), SceneError> {
    let marks: Vec<String> = (0..host.overview.item_count())
        .filter(|&index| host.overview.is_previous_item(index))
        .filter_map(|index| host.overview.item_title(index).map(str::to_owned))
        .collect();
    if args.first() == Some(&"none") {
        if !marks.is_empty() {
            return Err(SceneError {
                line,
                message: format!("previous {marks:?} != none"),
            });
        }
        return Ok(());
    }
    let expected: Vec<&str> = args
        .iter()
        .flat_map(|word| word.split(','))
        .filter(|word| !word.is_empty())
        .collect();
    if marks != expected {
        return Err(SceneError {
            line,
            message: format!("previous {marks:?} != {expected:?}"),
        });
    }
    Ok(())
}

fn expect_titles(host: &Host, args: &[&str], line: usize) -> Result<(), SceneError> {
    let expected: Vec<&str> = args
        .iter()
        .flat_map(|word| word.split(','))
        .filter(|word| !word.is_empty())
        .collect();
    let actual: Vec<String> = (0..host.overview.item_count())
        .filter_map(|index| host.overview.item_title(index).map(str::to_owned))
        .collect();
    if actual != expected {
        return Err(SceneError {
            line,
            message: format!("titles {actual:?} != {expected:?}"),
        });
    }
    Ok(())
}

fn expect_action(actual: &Action, args: &[&str], line: usize) -> Result<(), SceneError> {
    let expected = match args.first().copied() {
        Some("none") => Action::None,
        Some("dismiss") => Action::Dismiss,
        Some("commit") => Action::Commit {
            tab_index: parse_usize(args.get(1), line, "tab")? as u32,
        },
        Some("persist-pins") => Action::PersistPins,
        Some("switch") => Action::SwitchSession {
            name: args
                .get(1)
                .copied()
                .ok_or_else(|| SceneError {
                    line,
                    message: "action switch needs a session".into(),
                })?
                .to_owned(),
            tab_position: args.get(2).and_then(|word| word.parse().ok()),
        },
        Some(other) => {
            return Err(SceneError {
                line,
                message: format!("unknown action {other}"),
            });
        }
        None => {
            return Err(SceneError {
                line,
                message: "action needs a name".into(),
            });
        }
    };
    if actual != &expected {
        return Err(SceneError {
            line,
            message: format!("action {actual:?} != {expected:?}"),
        });
    }
    Ok(())
}

fn parse_usize(raw: Option<&&str>, line: usize, what: &str) -> Result<usize, SceneError> {
    raw.and_then(|word| word.parse().ok())
        .ok_or_else(|| SceneError {
            line,
            message: format!("need {what}"),
        })
}

#[cfg(test)]
mod tests {
    use super::run_scene;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn scenes_in_e2e_pass() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("e2e/scenes");
        let mut files: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("{}: {err}", dir.display()))
            .map(|entry| entry.expect("scene entry").path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("scene"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no .scene files in {}", dir.display());
        for path in files {
            let source = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("{}: {err}", path.display());
            });
            if let Err(err) = run_scene(&source) {
                panic!("{}: {err}", path.display());
            }
        }
    }
}
