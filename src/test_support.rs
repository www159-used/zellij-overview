use crate::{Pin, SessionFact, TabFact};

pub fn tab(id: usize, position: usize, name: &str, active: bool) -> TabFact {
    TabFact {
        id,
        position,
        name: name.to_owned(),
        active,
    }
}

pub fn session(name: &str, current: bool, tab_count: usize) -> SessionFact {
    SessionFact {
        name: name.to_owned(),
        current,
        tab_count,
        tabs: (0..tab_count)
            .map(|position| {
                tab(
                    100 + position,
                    position,
                    &format!("{name}-{position}"),
                    false,
                )
            })
            .collect(),
    }
}

pub fn numbered_tabs(count: usize) -> Vec<TabFact> {
    (0..count)
        .map(|position| {
            tab(
                position,
                position,
                &format!("tab-{position}"),
                position == 0,
            )
        })
        .collect()
}

pub fn pin(session: &str, tab_name: &str) -> Pin {
    Pin {
        session: session.to_owned(),
        tab_name: tab_name.to_owned(),
    }
}
