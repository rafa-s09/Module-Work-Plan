//! What the window remembers between runs besides what eframe keeps itself (window size and
//! place, panel sizes, collapsed sections): the editor mode, the board and panel switches, and
//! the open tabs and the board filters of each project. Kept as plain lines in eframe's storage; a line that does not
//! read is skipped, so a damaged file loses only that line.

use super::*;
use std::collections::BTreeMap;

/// The key in eframe's storage.
pub const KEY: &str = "mwp-gui";

pub struct Layout {
    pub mode: Mode,
    pub show_done: bool,
    pub show_problems: bool,
    /// By project folder (absolute): the open tab and the tabs, as paths relative to the project.
    pub tabs: BTreeMap<PathBuf, (Option<String>, Vec<String>)>,
    /// By project folder: the board filters (text, module, agent, size).
    pub filters: BTreeMap<PathBuf, [String; 4]>,
}

impl Default for Layout {
    fn default() -> Layout {
        Layout { mode: Mode::Split, show_done: false, show_problems: true, tabs: BTreeMap::new(), filters: BTreeMap::new() }
    }
}

const MODES: [(Mode, &str); 3] = [(Mode::Edit, "source"), (Mode::Split, "split"), (Mode::Preview, "preview")];

impl Layout {
    pub fn parse(text: &str) -> Layout {
        let mut l = Layout::default();
        for line in text.lines() {
            let mut f = line.split('\t');
            match (f.next(), f.next()) {
                (Some("mode"), Some(m)) => l.mode = MODES.iter().find(|x| x.1 == m).map_or(l.mode, |x| x.0),
                (Some("show_done"), Some(b)) => l.show_done = b == "1",
                (Some("show_problems"), Some(b)) => l.show_problems = b != "0",
                (Some("tabs"), Some(root)) if !root.is_empty() => {
                    let active = f.next().filter(|a| !a.is_empty()).map(str::to_string);
                    l.tabs.insert(PathBuf::from(root), (active, f.filter(|p| !p.is_empty()).map(str::to_string).collect()));
                }
                (Some("filter"), Some(root)) if !root.is_empty() => {
                    let mut v: [String; 4] = Default::default();
                    for (slot, field) in v.iter_mut().zip(f) {
                        *slot = field.to_string();
                    }
                    l.filters.insert(PathBuf::from(root), v);
                }
                _ => {}
            }
        }
        l
    }

    pub fn text(&self) -> String {
        let mode = MODES.iter().find(|x| x.0 == self.mode).map_or("split", |x| x.1);
        let mut out = format!("mode\t{mode}\nshow_done\t{}\nshow_problems\t{}\n", self.show_done as u8, self.show_problems as u8);
        for (root, (active, tabs)) in &self.tabs {
            out += &format!("tabs\t{}\t{}\t{}\n", root.display(), active.as_deref().unwrap_or(""), tabs.join("\t"));
        }
        for (root, v) in self.filters.iter().filter(|f| f.1.iter().any(|x| !x.is_empty())) {
            // A tab or a line break typed in the filter would break the line format.
            let v: Vec<String> = v.iter().map(|x| x.replace(['\t', '\n', '\r'], " ")).collect();
            out += &format!("filter\t{}\t{}\n", root.display(), v.join("\t"));
        }
        out
    }
}

/// The folder as the key of its tabs: absolute, so the same project matches from any working folder.
pub fn project_key(root: &Path) -> PathBuf {
    std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf())
}

/// A path inside the project as it is stored: relative, with `/`.
pub fn rel_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trips_and_survives_damage() {
        let mut l = Layout { mode: Mode::Preview, show_done: true, show_problems: false, ..Layout::default() };
        l.tabs.insert(PathBuf::from("/p/one"), (Some("mwp/map.md".into()), vec!["mwp/PROJECT.md".into(), "mwp/map.md".into()]));
        l.tabs.insert(PathBuf::from("/p/two"), (None, vec![]));
        l.filters.insert(PathBuf::from("/p/one"), ["check\tout".into(), "order".into(), String::new(), "M".into()]);
        l.filters.insert(PathBuf::from("/p/two"), Default::default());
        let back = Layout::parse(&l.text());
        assert!(back.mode == Mode::Preview && back.show_done && !back.show_problems);
        assert_eq!(back.tabs, l.tabs);
        // A tab typed in the filter is kept as a space; an empty filter is not written at all.
        assert_eq!(back.filters.get(Path::new("/p/one")), Some(&["check out".to_string(), "order".into(), String::new(), "M".into()]));
        assert_eq!(back.filters.len(), 1);

        // Garbage lines are skipped; what still reads is kept.
        let back = Layout::parse("mode\tzoom\n\u{0}\u{1}junk\ntabs\t\nshow_done\t1\ntabs\t/p/one\t\tmwp/a.md\n");
        assert!(back.mode == Mode::Split && back.show_done && back.show_problems);
        assert_eq!(back.tabs.get(Path::new("/p/one")), Some(&(None, vec!["mwp/a.md".to_string()])));
        assert_eq!(back.tabs.len(), 1);
    }
}
