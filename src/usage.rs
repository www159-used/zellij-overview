//! Local usage log. No titles, no upload.

use crate::Key;

pub const USAGE_CAP: usize = 400;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Usage {
    keys: u32,
    flash: bool,
    hjkl: bool,
    dash: bool,
    drill: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageEnd {
    Dismiss,
    Toggle,
    Tab,
    Switch,
    Prev,
}

impl UsageEnd {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dismiss => "dismiss",
            Self::Toggle => "toggle",
            Self::Tab => "tab",
            Self::Switch => "switch",
            Self::Prev => "prev",
        }
    }
}

impl Usage {
    pub fn note(&mut self, key: Key) {
        self.keys = self.keys.saturating_add(1);
        match key {
            Key::StartHint | Key::Input(_) | Key::Backspace => self.flash = true,
            Key::PreviousTab => self.dash = true,
            _ => {}
        }
    }

    pub fn note_hjkl(&mut self) {
        self.hjkl = true;
    }

    pub fn note_drill(&mut self) {
        self.drill = true;
    }

    pub fn encode(&self, end: UsageEnd, cross: bool) -> String {
        format!(
            "{{\"keys\":{},\"flash\":{},\"hjkl\":{},\"dash\":{},\"drill\":{},\"cross\":{},\"end\":\"{}\"}}",
            self.keys,
            json_bool(self.flash),
            json_bool(self.hjkl),
            json_bool(self.dash),
            json_bool(self.drill),
            json_bool(cross),
            end.as_str()
        )
    }
}

pub fn append_usage_log(existing: &str, line: &str, max_lines: usize) -> String {
    let mut lines: Vec<&str> = existing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let line = line.trim();
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.len() > max_lines {
        lines.drain(0..lines.len() - max_lines);
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn json_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_flags_without_titles() {
        let mut usage = Usage::default();
        usage.note(Key::StartHint);
        usage.note(Key::Input('g'));
        usage.note_hjkl();
        usage.note(Key::Down);
        usage.note(Key::PreviousTab);
        usage.note_drill();
        let line = usage.encode(UsageEnd::Switch, true);
        assert_eq!(
            line,
            r#"{"keys":4,"flash":true,"hjkl":true,"dash":true,"drill":true,"cross":true,"end":"switch"}"#
        );
        assert!(!line.contains("lp"));
        assert!(!line.contains("ww"));
        assert!(!line.contains("name"));
        assert!(!line.contains("title"));
    }

    #[test]
    fn append_drops_the_oldest_lines_when_capped() {
        let existing = "{\"keys\":1}\n{\"keys\":2}\n";
        let out = append_usage_log(existing, "{\"keys\":3}", 2);
        assert_eq!(out, "{\"keys\":2}\n{\"keys\":3}\n");
    }
}
