//! Find in project: every text file under mwp/ (an open tab as edited), in the bottom panel.

use super::*;

/// One match: the file, its line (from 1), the line's text, the match in the line and in the file (chars).
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
    pub cols: (usize, usize),
    pub at: (usize, usize),
}

/// The project search: query, match case and the matches found for them.
#[derive(Default)]
pub struct Search {
    pub query: String,
    pub case: bool,
    /// Give the query field the focus on the next frame.
    pub focus: bool,
    /// The text Replace all puts in place of each match.
    pub replace: String,
    pub hits: Vec<Hit>,
    /// What `hits` were found for: query, case and the project read they come from.
    found_for: Option<(String, bool, Instant)>,
}

/// More are not listed; the query is too broad to read them all anyway.
pub const MAX_HITS: usize = 1000;

/// Every match in the project's text files (templates `_*` left out, like the explorer does).
/// An open tab is searched as it is in the editor, unsaved edits included.
pub fn find_all(mwp: &Path, tabs: &[Editor], query: &str, case: bool) -> Vec<Hit> {
    let mut out = Vec::new();
    for path in text_files(mwp) {
        let text = match tabs.iter().find(|t| t.path == path) {
            Some(t) => t.text.clone(),
            None => match fs::read_to_string(&path) {
                Ok(t) => t.replace("\r\n", "\n"),
                Err(_) => continue,
            },
        };
        let mut start = 0; // chars before this line
        for (n, line) in text.split('\n').enumerate() {
            for (a, b) in editor::matches(line, query, case) {
                if out.len() == MAX_HITS {
                    return out;
                }
                out.push(Hit { path: path.clone(), line: n + 1, text: line.to_string(), cols: (a, b), at: (start + a, start + b) });
            }
            start += line.chars().count() + 1;
        }
    }
    out
}

/// The text files the search reads, sorted: templates (`_*`) are left out, like the explorer does.
pub fn text_files(mwp: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(mwp, &mut files);
    files.retain(|f| TEXT_FILES.contains(&extension(f).as_str()) && !folder_name(f).starts_with('_'));
    files.sort();
    files
}

/// What Replace all would change: the matches, the files they are in, and how many of those
/// files are open in a tab. Unlike the list, it counts every match.
pub fn replace_plan(mwp: &Path, tabs: &[Editor], query: &str, case: bool) -> (usize, usize, usize) {
    let (mut hits, mut files, mut open) = (0, 0, 0);
    for path in text_files(mwp) {
        let tab = tabs.iter().find(|t| t.path == path);
        let text = match tab {
            Some(t) => t.text.clone(),
            None => fs::read_to_string(&path).unwrap_or_default(),
        };
        let n = editor::matches(&text, query, case).len();
        if n > 0 {
            (hits, files, open) = (hits + n, files + 1, open + tab.is_some() as usize);
        }
    }
    (hits, files, open)
}

impl App {
    /// Replace all in the project. A file open in a tab is changed in the tab and left unsaved,
    /// so nothing is written over an edit and a Ctrl+Z there undoes it; the others are written
    /// now, keeping their line endings. Returns the matches replaced and the files that failed.
    pub fn replace_in_project(&mut self, with: &str) -> (usize, Vec<String>) {
        let (query, case) = (self.search.query.clone(), self.search.case);
        let (mut n, mut failed) = (0, Vec::new());
        for path in text_files(&self.root.join("mwp")) {
            if let Some(t) = self.tabs.iter_mut().find(|t| t.path == path) {
                n += editor::replace_matches(&mut t.text, &query, case, with);
                continue;
            }
            // The query is one line, so matching the raw text (CRLF and all) finds the same matches.
            let Ok(mut text) = fs::read_to_string(&path) else { continue };
            let k = editor::replace_matches(&mut text, &query, case, with);
            if k > 0 {
                match fs::write(&path, text) {
                    Ok(()) => n += k,
                    Err(e) => failed.push(format!("{}: {e}", path.display())),
                }
            }
        }
        self.reload();
        (n, failed)
    }
}

impl Search {
    /// Opens the search with the focus in its field, seeded with `selected` when it is one line.
    pub fn open(&mut self, selected: Option<String>) {
        if let Some(s) = selected.filter(|s| !s.is_empty() && !s.contains('\n')) {
            self.query = s;
        }
        self.focus = true;
    }

    /// Searches again when the query, the case or the project read changed.
    // shortcut: the project is read again on every reload (2 s) while there is a query; index it if that gets slow.
    fn refresh(&mut self, mwp: &Path, tabs: &[Editor], loaded: Instant) {
        let key = (self.query.clone(), self.case, loaded);
        if self.found_for.as_ref() != Some(&key) {
            self.hits = if self.query.is_empty() { Vec::new() } else { find_all(mwp, tabs, &self.query, self.case) };
            self.found_for = Some(key);
        }
    }
}

/// The line of a hit with its match highlighted, cut around it when the line is long.
fn snippet(ui: &egui::Ui, h: &Hit) -> egui::text::LayoutJob {
    let chars: Vec<char> = h.text.chars().collect();
    let (a, b) = h.cols;
    let from = a.saturating_sub(40);
    let to = (b + 120).min(chars.len());
    let part = |r: std::ops::Range<usize>| chars[r].iter().collect::<String>();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let plain = egui::TextFormat::simple(font.clone(), ui.visuals().text_color());
    let mut job = egui::text::LayoutJob::default();
    job.append(&format!("{}{}", if from > 0 { "…" } else { "" }, part(from..a).trim_start()), 0.0, plain.clone());
    job.append(&part(a..b), 0.0, egui::TextFormat { background: Color32::from_rgba_unmultiplied(230, 180, 40, 90), ..plain.clone() });
    job.append(&format!("{}{}", part(b..to), if to < chars.len() { "…" } else { "" }), 0.0, plain);
    job
}

/// The Search tab of the bottom panel: query and match case, then the matches by file.
/// A click opens the file with the match selected.
pub fn search_panel(ui: &mut egui::Ui, v: &View, tabs: &[Editor], loaded: Instant, s: &mut Search, ask: &mut Asks) {
    ui.horizontal(|ui| {
        ui.label(ICON_SEARCH.rich_text().color(ui.visuals().weak_text_color()));
        let r = ui.add(egui::TextEdit::singleline(&mut s.query).hint_text(tr("Find in project")).desired_width(320.0));
        if std::mem::take(&mut s.focus) {
            r.request_focus();
        }
        ui.toggle_value(&mut s.case, ICON_MATCH_CASE.rich_text()).on_hover_text(tr("Match case"));
        s.refresh(&v.project.mwp, tabs, loaded);
        let files = s.hits.iter().map(|h| &h.path).collect::<std::collections::BTreeSet<_>>().len();
        let count = match s.hits.len() {
            _ if s.query.is_empty() => String::new(),
            0 => tr("No results").into(),
            MAX_HITS => trf("First {max} results in {files}", &[("max", &MAX_HITS.to_string()), ("files", &count(files, "file", "files"))]),
            n => trf("{results} in {files}", &[("results", &count(n, "result", "results")), ("files", &count(files, "file", "files"))]),
        };
        ui.weak(count);
    });
    ui.horizontal(|ui| {
        ui.label(ICON_FIND_REPLACE.rich_text().color(ui.visuals().weak_text_color()));
        ui.add(egui::TextEdit::singleline(&mut s.replace).hint_text(tr("Replace with")).desired_width(320.0));
        let all = egui::Button::new(tr("Replace all…"));
        ask.replace_project |= ui.add_enabled(!s.hits.is_empty(), all).on_hover_text(tr("Asks first, with the number of matches and files")).clicked();
    });
    egui::ScrollArea::vertical().id_salt("search").auto_shrink([false, false]).show(ui, |ui| {
        let mut last: Option<&Path> = None;
        for h in &s.hits {
            if last != Some(&h.path) {
                last = Some(&h.path);
                let (icon, color) = file_icon(ui, &v.project, &h.path);
                let n = s.hits.iter().filter(|x| x.path == h.path).count();
                ui.horizontal(|ui| {
                    ui.label(icon.rich_text().color(color));
                    ui.strong(v.project.rel(&h.path));
                    ui.weak(n.to_string());
                });
            }
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(RichText::new(format!("{:>4}", h.line)).monospace().weak());
                if ui.selectable_label(false, snippet(ui, h)).clicked() {
                    ask.action = Some(Action::Open(h.path.clone()));
                    ask.reveal = Some((h.path.clone(), h.at));
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_in_files_and_open_tabs() {
        let root = std::env::temp_dir().join(format!("mwp-search-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("mwp/modules")).unwrap();
        fs::write(root.join("mwp/modules/order.md"), "# Order\r\nUses T-0007.\r\nAlso t-0007 and T-0007\r\n").unwrap();
        fs::write(root.join("mwp/modules/_template.md"), "T-0007").unwrap();
        fs::write(root.join("mwp/notes.txt"), "é T-0007").unwrap();
        fs::write(root.join("mwp/logo.png"), "T-0007").unwrap();

        let hits = find_all(&root.join("mwp"), &[], "T-0007", true);
        let found: Vec<(String, usize, (usize, usize))> = hits.iter().map(|h| (folder_name(&h.path), h.line, h.at)).collect();
        // Chars from the start of the file, with \r\n read as one char.
        assert_eq!(found, [("order.md".into(), 2, (13, 19)), ("order.md".into(), 3, (37, 43)), ("notes.txt".into(), 1, (2, 8))]);
        assert_eq!(find_all(&root.join("mwp"), &[], "T-0007", false).len(), 4, "case off finds t-0007 too");

        // An open tab is searched with its edits.
        let mut tab = Editor::open(&root.join("mwp/notes.txt")).unwrap();
        tab.text = "nothing".into();
        assert_eq!(find_all(&root.join("mwp"), &[tab], "T-0007", true).len(), 2);
        let _ = fs::remove_dir_all(&root);
    }
}
