//! The Markdown editor: one open file, its source view (highlighted, with line numbers and a
//! formatting toolbar) and its rendered preview.

use eframe::egui::{self, text::CCursor, text::LayoutJob, text_selection::CCursorRange, Color32, FontId, RichText, TextFormat};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_material_icons::{icons::*, MaterialIcon};
use crate::i18n::{count, tr, trf};
use mwp_core::doc::split_front_matter;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Edit,
    Split,
    Preview,
}

/// An open file. Kept with `\n` endings while editing, written back with the file's own.
pub struct Editor {
    pub path: PathBuf,
    pub text: String,
    saved: String,
    crlf: bool,
    /// Last selection in the source view, in characters.
    cursor: Option<CCursorRange>,
    cache: CommonMarkCache,
    /// The file as it is on disk, when someone else changed it while this tab has unsaved edits.
    /// Saving is refused until the user picks a version.
    pub disk: Option<String>,
    /// The changes against the disk version are shown under the conflict bar.
    show_disk: bool,
    /// The file is no longer on disk (deleted, or moved by someone else). Saving is refused
    /// until the user saves it again on purpose (`recreate`) or closes the tab.
    pub gone: bool,
    /// The find bar, while it is open.
    pub find: Option<Find>,
}

/// The find bar of a tab.
pub struct Find {
    pub query: String,
    case: bool,
    /// Give the query field the focus on the next frame.
    focus: bool,
    /// Scroll the source to the current match on the next frame.
    reveal: bool,
    /// The current match, as a char range. Kept here, not as the text box's selection: egui drops
    /// the selection of a text box without focus, and the focus stays in the query field.
    pub current: Option<(usize, usize)>,
    /// The replacement text, while the Replace row is shown.
    pub replace: Option<String>,
}

/// Every match of `query` in `text`, as char ranges (the text box counts in chars).
pub fn matches(text: &str, query: &str, case: bool) -> Vec<(usize, usize)> {
    // shortcut: case folding is per char, so a letter that folds to two chars (ß) only matches itself.
    let fold = |c: char| if case { c } else { c.to_lowercase().next().unwrap_or(c) };
    let (t, q): (Vec<char>, Vec<char>) = (text.chars().map(fold).collect(), query.chars().map(fold).collect());
    let mut out = Vec::new();
    let mut i = 0;
    while !q.is_empty() && i + q.len() <= t.len() {
        if t[i..i + q.len()] == q[..] {
            out.push((i, i + q.len()));
            i += q.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Replaces every match of `query` in `text` with `with`; returns how many.
pub fn replace_matches(text: &mut String, query: &str, case: bool, with: &str) -> usize {
    let m = matches(text, query, case);
    let bytes: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
    // From the end, so the offsets still to replace stay valid.
    for &(a, b) in m.iter().rev() {
        text.replace_range(bytes[a]..bytes[b], with);
    }
    m.len()
}

/// How the user settled a conflict between the tab's edits and the file on disk.
pub enum Resolve {
    /// Write the tab's text over the disk version.
    Mine,
    /// Drop the edits and load the disk version.
    Disk,
    /// The file was deleted on disk: write it again from the tab.
    Recreate,
    /// The file was deleted on disk: close the tab (asking first when it has edits).
    Close,
}

/// The lines of `mine` and `disk` in order, each marked ' ' (in both), '-' (only in the tab)
/// or '+' (only on disk).
pub fn line_diff<'a>(mine: &'a str, disk: &'a str) -> Vec<(char, &'a str)> {
    let (a, b): (Vec<&str>, Vec<&str>) = (mine.lines().collect(), disk.lines().collect());
    // The same start and end are kept out of the table, which then only holds the changed middle.
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..].iter().rev().zip(b[pre..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut out: Vec<(char, &str)> = a[..pre].iter().map(|l| (' ', *l)).collect();
    // shortcut: a full longest-common-subsequence table, fine for module-sized files; use a
    // Myers diff if conflicts on files with thousands of changed lines become common.
    let (n, m) = (am.len(), bm.len());
    let at = |i: usize, j: usize| i * (m + 1) + j;
    let mut t = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[at(i, j)] = if am[i] == bm[j] { t[at(i + 1, j + 1)] + 1 } else { t[at(i + 1, j)].max(t[at(i, j + 1)]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && am[i] == bm[j] {
            out.push((' ', am[i]));
            (i, j) = (i + 1, j + 1);
        } else if i < n && (j == m || t[at(i + 1, j)] >= t[at(i, j + 1)]) {
            out.push(('-', am[i]));
            i += 1;
        } else {
            out.push(('+', bm[j]));
            j += 1;
        }
    }
    out.extend(a[a.len() - suf..].iter().map(|l| (' ', *l)));
    out
}

impl Editor {
    pub fn open(path: &Path) -> Result<Editor, String> {
        let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = raw.replace("\r\n", "\n");
        Ok(Editor { path: path.to_path_buf(), saved: text.clone(), text, crlf: raw.contains("\r\n"), cursor: None, cache: CommonMarkCache::default(), disk: None, show_disk: false, gone: false, find: None })
    }

    /// Opens the find bar (or focuses it), with the selection as the query when it is on one line.
    /// `replace` also shows the Replace row.
    pub fn open_find(&mut self, replace: bool) {
        let (a, b) = sel(self);
        let picked = self.selected();
        let f = self.find.get_or_insert_with(|| Find { query: String::new(), case: false, focus: true, reveal: false, current: None, replace: None });
        if !picked.is_empty() && !picked.contains('\n') {
            f.query = picked;
            f.current = Some((a, b));
        }
        if replace && f.replace.is_none() {
            f.replace = Some(String::new());
        }
        f.focus = true;
    }

    /// The text selected in the source (empty when nothing is).
    pub fn selected(&self) -> String {
        let (a, b) = sel(self);
        self.text.chars().skip(a).take(b - a).collect()
    }

    /// Opens the find bar on a match found elsewhere (the project search), scrolled to it.
    pub fn show_match(&mut self, query: &str, case: bool, at: (usize, usize)) {
        let replace = self.find.take().and_then(|f| f.replace);
        self.find = Some(Find { query: query.to_string(), case, focus: false, reveal: true, current: Some(at), replace });
    }

    /// Replaces the current match and goes to the next one. When the current match is not a match
    /// (none yet, or the text changed), it only goes to the next one, so the user sees what is replaced.
    fn replace_current(&mut self) {
        let Some(f) = self.find.as_ref() else { return };
        let (Some(with), Some((a, b))) = (f.replace.clone(), f.current.filter(|c| matches(&self.text, &f.query, f.case).contains(c))) else {
            let next = self.find_step(true, false);
            if let Some(f) = self.find.as_mut() {
                (f.current, f.reveal) = (next, next.is_some());
            }
            return;
        };
        let (ba, bb) = (byte(&self.text, a), byte(&self.text, b));
        self.text.replace_range(ba..bb, &with);
        let end = a + with.chars().count();
        self.find.as_mut().unwrap().current = Some((a, end));
        let next = self.find_step(true, false);
        let f = self.find.as_mut().unwrap();
        (f.current, f.reveal) = (next, next.is_some());
    }

    /// Replaces every match at once; returns how many. One Ctrl+Z in the source undoes it.
    fn replace_all(&mut self) -> usize {
        let Some(f) = self.find.as_ref() else { return 0 };
        let Some(with) = f.replace.clone() else { return 0 };
        let n = replace_matches(&mut self.text, &f.query.clone(), f.case, &with);
        self.find.as_mut().unwrap().current = None;
        n
    }

    /// The next (or previous) match after the current one (or the cursor), wrapping around.
    /// `from_start` searches from where the current match starts, so it can be found again.
    fn find_step(&self, forward: bool, from_start: bool) -> Option<(usize, usize)> {
        let f = self.find.as_ref()?;
        let m = matches(&self.text, &f.query, f.case);
        let (a, b) = f.current.unwrap_or_else(|| sel(self));
        let after = if from_start { a } else { b };
        if forward { m.iter().find(|r| r.0 >= after).or(m.first()).copied() } else { m.iter().rev().find(|r| r.1 <= a).or(m.last()).copied() }
    }

    /// The file on disk with `\n` endings, and whether it uses CRLF.
    fn read_disk(&self) -> Option<(String, bool)> {
        fs::read_to_string(&self.path).ok().map(|raw| (raw.replace("\r\n", "\n"), raw.contains("\r\n")))
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn name(&self) -> String {
        self.path.file_name().unwrap().to_string_lossy().into_owned()
    }

    /// Writes the tab, unless the file changed on disk since it was loaded or saved: then the
    /// change is kept in `disk` and nothing is written, so neither version is lost.
    pub fn save(&mut self) -> Result<(), String> {
        // Writing now would bring back a file someone deleted: only the bar's Save again does that.
        if self.gone || !self.path.exists() {
            self.gone = true;
            return Err(trf("{name} was deleted on disk: save it again or close the tab", &[("name", &self.name())]));
        }
        // Read again here: an agent may have written since the last refresh.
        if let Some((disk, _)) = self.read_disk().filter(|(d, _)| *d != self.saved && *d != self.text) {
            self.disk = Some(disk);
            return Err(trf("{name} changed on disk: choose which version to keep", &[("name", &self.name())]));
        }
        self.write()
    }

    /// Writes the tab to its file, with the file's own line endings.
    fn write(&mut self) -> Result<(), String> {
        let text = if self.crlf { self.text.replace('\n', "\r\n") } else { self.text.clone() };
        fs::write(&self.path, text).map_err(|e| trf("cannot save {path}: {error}", &[("path", &self.path.display().to_string()), ("error", &e.to_string())]))?;
        (self.saved, self.disk) = (self.text.clone(), None);
        Ok(())
    }

    /// Save again: writes a file that was deleted on disk back from the tab, its folder too.
    pub fn recreate(&mut self) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| trf("cannot save {path}: {error}", &[("path", &self.path.display().to_string()), ("error", &e.to_string())]))?;
        }
        self.write()?;
        self.gone = false;
        Ok(())
    }

    /// Follows the file on disk (an agent may have changed it). With unsaved edits the tab keeps
    /// them and records the disk version as a conflict instead.
    pub fn refresh(&mut self) {
        // A file that cannot be read but still exists (locked, mid-write) is left for the next refresh.
        self.gone = !self.path.exists();
        let Some((disk, crlf)) = self.read_disk() else { return };
        if !self.dirty() {
            if disk != self.text {
                (self.text, self.saved, self.crlf) = (disk.clone(), disk, crlf);
            }
            self.disk = None;
        } else if disk != self.saved && disk != self.text {
            self.disk = Some(disk);
        } else {
            self.disk = None;
        }
    }

    /// Settles a conflict. `Mine` takes the disk version as the base, so the next save writes over it.
    pub fn resolve(&mut self, how: Resolve) {
        match (how, self.read_disk()) {
            (Resolve::Mine, Some((disk, _))) => self.saved = disk,
            (Resolve::Disk, Some((disk, crlf))) => (self.text, self.saved, self.crlf) = (disk.clone(), disk, crlf),
            _ => {} // Recreate and Close are done by the app; Mine / Disk with no file left change nothing
        }
        (self.disk, self.show_disk) = (None, false);
    }

    /// The id of the source text box, so the Edit menu can give it focus back.
    pub fn id(&self) -> egui::Id {
        egui::Id::unique(("source", &self.path))
    }
}

/// Byte offset of the character at `ci`.
fn byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map_or(s.len(), |(b, _)| b)
}

/// Wraps the selection in `open`/`close` (e.g. `**`), or inserts them around the cursor.
fn wrap(ed: &mut Editor, open: &str, close: &str) -> CCursorRange {
    let (a, b) = sel(ed);
    let (ba, bb) = (byte(&ed.text, a), byte(&ed.text, b));
    ed.text.insert_str(bb, close);
    ed.text.insert_str(ba, open);
    let n = open.chars().count();
    CCursorRange::two(CCursor::new(a + n), CCursor::new(b + n))
}

/// Toggles `prefix` (e.g. `- `) at the start of every selected line. Headings replace each other.
fn prefix(ed: &mut Editor, prefix: &str) -> CCursorRange {
    let (a, b) = sel(ed);
    let (ba, bb) = (byte(&ed.text, a), byte(&ed.text, b));
    let start = ed.text[..ba].rfind('\n').map_or(0, |i| i + 1);
    let end = ed.text[bb..].find('\n').map_or(ed.text.len(), |i| bb + i);
    let lines: Vec<String> = ed.text[start..end].split('\n').map(|l| {
        let bare = if prefix.starts_with('#') { l.trim_start_matches('#').trim_start() } else { l };
        if l.starts_with(prefix) { l[prefix.len()..].to_string() } else { format!("{prefix}{bare}") }
    }).collect();
    let new = lines.join("\n");
    let cstart = ed.text[..start].chars().count();
    let len = new.chars().count();
    ed.text.replace_range(start..end, &new);
    CCursorRange::two(CCursor::new(cstart), CCursor::new(cstart + len))
}

fn sel(ed: &Editor) -> (usize, usize) {
    let len = ed.text.chars().count();
    let (a, b) = ed.cursor.map_or((len, len), |r| (r.primary.index.0, r.secondary.index.0));
    (a.min(b).min(len), a.max(b).min(len))
}

struct Palette {
    text: Color32,
    weak: Color32,
    heading: Color32,
    key: Color32,
    value: Color32,
    marker: Color32,
    code: Color32,
    placeholder: Color32,
}

fn palette(v: &egui::Visuals) -> Palette {
    let c = |dark: (u8, u8, u8), light: (u8, u8, u8)| {
        let (r, g, b) = if v.dark_mode { dark } else { light };
        Color32::from_rgb(r, g, b)
    };
    Palette {
        text: v.text_color(),
        weak: v.weak_text_color(),
        heading: c((97, 175, 239), (0, 92, 197)),
        key: c((224, 108, 117), (207, 34, 46)),
        value: c((152, 195, 121), (17, 99, 41)),
        marker: c((209, 154, 102), (149, 56, 0)),
        code: c((86, 182, 194), (5, 80, 174)),
        placeholder: c((229, 192, 123), (130, 80, 0)),
    }
}

/// Markdown highlighting for the source view, line by line.
fn highlight(text: &str, font: FontId, p: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut put = |s: &str, color: Color32| job.append(s, 0.0, TextFormat::simple(font.clone(), color));
    let front_matter = split_front_matter(text).map(|(fm, _)| fm.lines().count() + 2).unwrap_or(0);
    let mut in_code = false;
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let body = line.trim_end_matches('\n');
        let nl = &line[body.len()..];
        if n < front_matter {
            match body.split_once(':') {
                Some((k, v)) if n > 0 => {
                    put(k, p.key);
                    put(":", p.weak);
                    put(v, p.value);
                }
                _ => put(body, p.weak),
            }
        } else if body.trim_start().starts_with("```") {
            in_code = !in_code;
            put(body, p.weak);
        } else if in_code {
            put(body, p.code);
        } else if body.starts_with('#') {
            put(body, p.heading);
        } else if body.starts_with('>') || body.starts_with("---") {
            put(body, p.weak);
        } else {
            let indent = body.len() - body.trim_start().len();
            let rest = &body[indent..];
            let marker = ["- [ ] ", "- [x] ", "- ", "* ", "+ "].into_iter().find(|m| rest.starts_with(m)).map(str::len)
                .or_else(|| rest.find(". ").filter(|&i| i > 0 && rest[..i].bytes().all(|b| b.is_ascii_digit())).map(|i| i + 2))
                .unwrap_or(0);
            put(&body[..indent + marker], p.marker);
            inline(&body[indent + marker..], &mut put, p);
        }
        put(nl, p.text);
    }
    job
}

/// `code`, **bold** and <placeholder> spans inside a line.
fn inline(s: &str, put: &mut impl FnMut(&str, Color32), p: &Palette) {
    let mut rest = s;
    while !rest.is_empty() {
        let next = ["`", "**", "<"].iter().filter_map(|t| rest.find(t).map(|i| (i, *t))).min();
        let Some((i, t)) = next else { break };
        let close = if t == "<" { ">" } else { t };
        let Some(j) = rest[i + t.len()..].find(close).map(|j| i + t.len() + j + close.len()) else { break };
        put(&rest[..i], p.text);
        put(&rest[i..j], match t { "`" => p.code, "**" => p.text, _ => p.placeholder });
        rest = &rest[j..];
    }
    put(rest, p.text);
}

/// The editor of one tab: toolbar, source and preview. Returns how a conflict with the disk was settled, and the file of an
/// ID that was clicked (in the preview, or Ctrl+click in the source).
pub fn show(ui: &mut egui::Ui, ed: &mut Editor, mode: &mut Mode, find_id: FindId) -> (Option<Resolve>, Option<PathBuf>) {
    let resolve = conflict_bar(ui, ed);
    let mut change: Option<CCursorRange> = None;
    ui.horizontal(|ui| {
        ui.add_enabled_ui(*mode != Mode::Preview, |ui| {
            let tools: [(MaterialIcon, &str, fn(&mut Editor) -> CCursorRange); 10] = [
                (ICON_FORMAT_H1, "Heading 1", |e| prefix(e, "# ")),
                (ICON_FORMAT_H2, "Heading 2", |e| prefix(e, "## ")),
                (ICON_FORMAT_H3, "Heading 3", |e| prefix(e, "### ")),
                (ICON_FORMAT_BOLD, "Bold", |e| wrap(e, "**", "**")),
                (ICON_FORMAT_ITALIC, "Italic", |e| wrap(e, "*", "*")),
                (ICON_CODE, "Inline code", |e| wrap(e, "`", "`")),
                (ICON_FORMAT_LIST_BULLETED, "Bulleted list", |e| prefix(e, "- ")),
                (ICON_CHECKLIST, "Checklist", |e| prefix(e, "- [ ] ")),
                (ICON_FORMAT_QUOTE, "Quote", |e| prefix(e, "> ")),
                (ICON_LINK, "Link", |e| wrap(e, "[", "](url)")),
            ];
            for (i, (icon, tip, f)) in tools.into_iter().enumerate() {
                if i == 3 || i == 6 {
                    ui.separator();
                }
                let button = egui::Button::new(icon.rich_text().size(18.0)).frame(false).min_size(egui::vec2(26.0, 24.0));
                if ui.add(button).on_hover_text(tr(tip)).clicked() {
                    change = Some(f(ed));
                }
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (m, icon, label) in [(Mode::Preview, ICON_VISIBILITY, "Preview"), (Mode::Split, ICON_VERTICAL_SPLIT, "Split"), (Mode::Edit, ICON_CODE, "Source")] {
                ui.selectable_value(mode, m, format!("{} {}", icon.codepoint, tr(label)));
            }
        });
    });
    if ed.find.is_some() {
        // Matches are shown in the source, so a search from Preview brings it back.
        if *mode == Mode::Preview {
            *mode = Mode::Split;
        }
        find_bar(ui, ed);
    }
    ui.separator();

    // Status of the source: position, words, line endings.
    let line_info = {
        let (a, _) = sel(ed);
        let before: String = ed.text.chars().take(a).collect();
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        trf("Ln {line}, Col {col}   ·   {words} words   ·   {eol}   ·   Markdown", &[("line", &line.to_string()), ("col", &col.to_string()), ("words", &ed.text.split_whitespace().count().to_string()), ("eol", if ed.crlf { "CRLF" } else { "LF" })])
    };
    // The views get the height minus one status row, so the scrolled text never runs under it.
    let id = ed.id();
    let status_h = ui.spacing().interact_size.y;
    let body = egui::vec2(ui.available_width(), (ui.available_height() - status_h - ui.spacing().item_spacing.y).max(0.0));
    let open = ui.allocate_ui(body, |ui| match *mode {
        Mode::Edit => source(ui, ed, id, find_id),
        Mode::Preview => preview(ui, ed, find_id),
        Mode::Split => {
            let area = ui.max_rect();
            let open = ui.columns(2, |cols| {
                let a = source(&mut cols[0], ed, id, find_id);
                preview(&mut cols[1], ed, find_id).or(a)
            });
            // A line where the source ends and the preview starts, in the gap between the columns.
            ui.painter().vline(area.center().x, area.y_range(), ui.visuals().widgets.noninteractive.bg_stroke);
            open
        }
    }).inner;
    // Align::Min: a centred right-to-left layout would take all the height it is offered.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| ui.weak(line_info));

    if let Some(range) = change {
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(ui.ctx(), id);
        ui.ctx().memory_mut(|m| m.request_focus(id));
        ed.cursor = Some(range);
    }
    (resolve, open)
}

/// The find bar: query, match case, "n of m", previous / next, close. Enter and F3 go to the next
/// match, Shift with them to the previous one, Esc closes. The query keeps the focus; the match is
/// selected in the source and scrolled into view. The chevron (or Ctrl+H) shows the Replace row,
/// where Enter replaces the current match.
fn find_bar(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(f) = ed.find.as_ref() else { return };
    let m = matches(&ed.text, &f.query, f.case);
    let here = f.current.and_then(|c| m.iter().position(|r| *r == c));
    let (mut step, mut close, mut typed) = (None, false, false);
    let (mut replace_one, mut replace_all) = (false, false);
    let field_id = egui::Id::unique(("find", &ed.path));
    let f = ed.find.as_mut().unwrap();
    let arrow = |icon: MaterialIcon| egui::Button::new(icon.rich_text()).frame(false);
    ui.horizontal(|ui| {
        let chevron = if f.replace.is_some() { ICON_EXPAND_MORE } else { ICON_CHEVRON_RIGHT };
        if ui.add(arrow(chevron)).on_hover_text(tr("Replace (Ctrl+H)")).clicked() {
            f.replace = if f.replace.is_some() { None } else { Some(String::new()) };
        }
        ui.label(ICON_SEARCH.rich_text().size(18.0).color(ui.visuals().weak_text_color()));
        let r = ui.add(egui::TextEdit::singleline(&mut f.query).id(field_id).hint_text(tr("Find")).desired_width(260.0));
        if std::mem::take(&mut f.focus) {
            r.request_focus();
        }
        let (enter, shift, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.modifiers.shift, i.key_pressed(egui::Key::Escape)));
        if r.lost_focus() && enter {
            step = Some(!shift);
            r.request_focus(); // stay in the field for the next Enter
        }
        typed = r.changed();
        if (r.has_focus() || r.lost_focus()) && esc {
            close = true;
        }
        ui.toggle_value(&mut f.case, ICON_MATCH_CASE.rich_text()).on_hover_text(tr("Match case"));
        let count = match (m.len(), here) {
            (0, _) if f.query.is_empty() => String::new(),
            (0, _) => tr("No results").into(),
            (n, Some(i)) => trf("{i} of {n}", &[("i", &(i + 1).to_string()), ("n", &n.to_string())]),
            (n, None) => trf("{n} found", &[("n", &n.to_string())]),
        };
        ui.label(egui::RichText::new(count).color(if m.is_empty() && !f.query.is_empty() { ui.visuals().error_fg_color } else { ui.visuals().weak_text_color() }));
        if ui.add_enabled(!m.is_empty(), arrow(ICON_ARROW_UPWARD)).on_hover_text(tr("Previous match (Shift+Enter, Shift+F3)")).clicked() {
            step = Some(false);
        }
        if ui.add_enabled(!m.is_empty(), arrow(ICON_ARROW_DOWNWARD)).on_hover_text(tr("Next match (Enter, F3)")).clicked() {
            step = Some(true);
        }
        close |= ui.add(arrow(ICON_CLOSE)).on_hover_text(tr("Close (Esc)")).clicked();
    });
    if let Some(with) = f.replace.as_mut() {
        ui.horizontal(|ui| {
            // Lined up under the query field: the chevron's width, then the search icon's.
            ui.add_space(ui.spacing().interact_size.y + ui.spacing().item_spacing.x);
            ui.label(ICON_FIND_REPLACE.rich_text().size(18.0).color(ui.visuals().weak_text_color()));
            let r = ui.add(egui::TextEdit::singleline(with).hint_text(tr("Replace")).desired_width(260.0));
            let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
            if r.lost_focus() && enter {
                replace_one = true;
                r.request_focus();
            }
            if (r.has_focus() || r.lost_focus()) && esc {
                close = true;
            }
            let any = !m.is_empty();
            replace_one |= ui.add_enabled(any, egui::Button::new(tr("Replace"))).on_hover_text(tr("Replace this match and go to the next (Enter)")).clicked();
            replace_all = ui.add_enabled(any, egui::Button::new(tr("Replace all"))).clicked();
        });
    }
    if replace_all {
        ed.replace_all();
        return;
    }
    if replace_one {
        ed.replace_current();
        return;
    }
    if let Some(back) = ui.input(|i| i.key_pressed(egui::Key::F3).then_some(i.modifiers.shift)) {
        step = Some(!back);
    }
    if close {
        // Closing leaves the current match selected in the source, with the focus there.
        if let Some((a, b)) = ed.find.take().and_then(|f| f.current).filter(|c| m.contains(c)) {
            let (id, range) = (ed.id(), CCursorRange::two(CCursor::new(a), CCursor::new(b)));
            let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            state.cursor.set_char_range(Some(range));
            state.store(ui.ctx(), id);
            ed.cursor = Some(range);
            ui.ctx().memory_mut(|mem| mem.request_focus(id));
        }
        return;
    }
    // Typing searches again from where the current match starts, so it stays while it still matches.
    let found = if typed { ed.find_step(true, true) } else { step.and_then(|forward| ed.find_step(forward, false)) };
    if let Some(f) = ed.find.as_mut() {
        if typed || found.is_some() {
            (f.current, f.reveal) = (found, found.is_some());
        }
    }
}

/// The warning shown while the file on disk and the tab's edits disagree, with the two ways out
/// and, on request, the lines that differ. Or, when the file was deleted on disk, the bar that
/// offers to save it again or close the tab.
fn conflict_bar(ui: &mut egui::Ui, ed: &mut Editor) -> Option<Resolve> {
    if ed.gone {
        return gone_bar(ui);
    }
    let disk = ed.disk.as_deref()?;
    let warn = ui.visuals().warn_fg_color;
    let mut resolve = None;
    egui::Frame::new().fill(warn.gamma_multiply(0.12)).stroke(egui::Stroke::new(1.0, warn.gamma_multiply(0.5))).corner_radius(4).inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(ICON_WARNING.rich_text().size(18.0).color(warn));
            ui.vertical(|ui| {
                ui.strong(tr("This file changed on disk while you have unsaved edits."));
                ui.weak(tr("Saving is paused until you choose which version to keep."));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Load disk version")).on_hover_text(tr("Drop your edits")).clicked() {
                    resolve = Some(Resolve::Disk);
                }
                if ui.button(tr("Overwrite with mine")).on_hover_text(tr("Save your edits over the disk version")).clicked() {
                    resolve = Some(Resolve::Mine);
                }
                ui.toggle_value(&mut ed.show_disk, (ICON_DIFFERENCE.rich_text(), tr("Show changes")));
            });
        });
        if ed.show_disk {
            ui.add_space(4.0);
            diff_view(ui, &ed.text, disk, &ed.path);
        }
    });
    ui.add_space(4.0);
    resolve
}

/// The lines that differ between the tab and the disk, with two lines around each change; longer
/// runs of equal lines fold into one row. The text can be selected and copied.
fn diff_view(ui: &mut egui::Ui, mine: &str, disk: &str, path: &Path) {
    let diff = line_diff(mine, disk);
    let near = |k: usize| diff[k.saturating_sub(2)..(k + 3).min(diff.len())].iter().any(|d| d.0 != ' ');
    let v = ui.visuals().clone();
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("− only in your edits")).monospace().color(v.error_fg_color));
        ui.label(RichText::new(tr("+ only on disk")).monospace().color(Color32::from_rgb(90, 170, 100)));
    });
    egui::ScrollArea::vertical().id_salt(("diff", path)).max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut k = 0;
        while k < diff.len() {
            if !near(k) {
                let run = (k..diff.len()).take_while(|&x| !near(x)).count();
                ui.label(RichText::new(format!("  ⋯ {}", count(run, "unchanged line", "unchanged lines"))).monospace().weak());
                k += run;
                continue;
            }
            let (mark, line) = diff[k];
            let text = RichText::new(format!("{mark} {line}")).monospace();
            ui.label(match mark {
                '-' => text.color(v.error_fg_color).background_color(v.error_fg_color.gamma_multiply(0.12)),
                '+' => text.color(Color32::from_rgb(90, 170, 100)).background_color(Color32::from_rgba_unmultiplied(90, 170, 100, 30)),
                _ => text.color(v.weak_text_color()),
            });
            k += 1;
        }
    });
}

/// The file of the tab is no longer on disk: save it again, or close the tab.
fn gone_bar(ui: &mut egui::Ui) -> Option<Resolve> {
    let err = ui.visuals().error_fg_color;
    let mut resolve = None;
    egui::Frame::new().fill(err.gamma_multiply(0.12)).stroke(egui::Stroke::new(1.0, err.gamma_multiply(0.5))).corner_radius(4).inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(ICON_DELETE.rich_text().size(18.0).color(err));
            ui.vertical(|ui| {
                ui.strong(tr("This file was deleted on disk."));
                ui.weak(tr("Saving is paused: save it again to bring it back, or close the tab."));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Close tab")).clicked() {
                    resolve = Some(Resolve::Close);
                }
                if ui.button(tr("Save again")).on_hover_text(tr("Write the file again from this tab")).clicked() {
                    resolve = Some(Resolve::Recreate);
                }
            });
        });
    });
    ui.add_space(4.0);
    resolve
}

/// The source view; returns the file of an ID that was Ctrl+clicked.
fn source(ui: &mut egui::Ui, ed: &mut Editor, id: egui::Id, find_id: FindId) -> Option<PathBuf> {
    let mut open = None;
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let p = palette(ui.visuals());
    let digits = ed.text.lines().count().max(1).to_string().len().max(3) as f32;
    let gutter = digits * ui.fonts_mut(|f| f.glyph_width(&font, '0')) + 18.0;
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
        let mut job = highlight(buf.as_str(), font.clone(), &p);
        job.wrap.max_width = wrap;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    egui::ScrollArea::vertical().id_salt(("source-scroll", id)).auto_shrink([false, false]).show(ui, |ui| {
        let out = egui::TextEdit::multiline(&mut ed.text)
            .id(id)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            // The margin goes in the frame: with a custom frame, TextEdit::margin is ignored.
            .frame(egui::Frame::NONE.inner_margin(egui::Margin { left: gutter.min(120.0) as i8, right: 8, top: 6, bottom: 6 }))
            .desired_width(f32::INFINITY)
            .min_size(ui.available_size())
            .layouter(&mut layouter)
            .show(ui);
        if out.cursor_range.is_some() {
            ed.cursor = out.cursor_range;
        }
        // Ctrl+click on an ID opens its file; the click has put the cursor on it.
        if out.response.clicked() && ui.input(|i| i.modifiers.command) {
            if let Some(r) = out.cursor_range {
                open = find_id(&id_at(&ed.text, r.primary.index.0));
            }
        }
        let to_galley = out.galley_pos.to_vec2();
        if let Some(f) = ed.find.as_mut().filter(|f| f.reveal) {
            f.reveal = false;
            if let Some((a, _)) = f.current {
                ui.scroll_to_rect(out.galley.pos_from_cursor(CCursor::new(a)).translate(to_galley).expand(24.0), Some(egui::Align::Center));
            }
        }
        // Current line and line numbers, painted in the left margin.
        let painter = ui.painter_at(out.response.rect);
        let left = out.response.rect.left();
        let current = ed.cursor.map(|r| out.galley.pos_from_cursor(r.primary).translate(out.galley_pos.to_vec2()));
        if let (Some(r), true) = (current, out.response.has_focus()) {
            let band = egui::Rect::from_x_y_ranges(out.response.rect.x_range(), r.y_range());
            painter.rect_filled(band, 0.0, ui.visuals().faint_bg_color);
        }
        let mut number = 1;
        let mut starts_line = true;
        for row in &out.galley.rows {
            if starts_line {
                let y = out.galley_pos.y + row.pos.y;
                let color = if current.is_some_and(|c| (c.top() - y).abs() < 1.0) { p.text } else { p.weak };
                painter.text(egui::pos2(left + gutter - 10.0, y), egui::Align2::RIGHT_TOP, number, font.clone(), color);
                number += 1;
            }
            starts_line = row.ends_with_newline;
        }
        // Find matches: all of them lightly, the selected one stronger. The highlight goes over the
        // text, so it is translucent.
        if let Some(f) = &ed.find {
            let picked = f.current.unwrap_or_default();
            for (a, b) in matches(&ed.text, &f.query, f.case).into_iter().take(2000) {
                let (pa, pb) = (out.galley.pos_from_cursor(CCursor::new(a)), out.galley.pos_from_cursor(CCursor::new(b)));
                if (pa.top() - pb.top()).abs() < 1.0 {
                    let rect = egui::Rect::from_min_max(pa.left_top(), egui::pos2(pb.left(), pa.bottom())).translate(to_galley);
                    let alpha = if (a, b) == picked { 110 } else { 45 };
                    painter.rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(230, 180, 40, alpha));
                }
            }
        }
    });
    open
}

/// Finds the file of a project or module ID (`T-0007`, `SP-0001`, `ADR-0002`, `order`).
pub type FindId<'a> = &'a dyn Fn(&str) -> Option<PathBuf>;

/// The project IDs (`T-`, `SP-`, `ADR-`) a link is made for.
const LINKED: [&str; 3] = ["T-", "SP-", "ADR-"];

/// The Markdown body with every ID that `known` accepts made a `mwp:<id>` link, and the IDs linked:
/// project IDs in the text, and module or project IDs written as code (`` `order` ``, `` `order#FR-01` ``).
/// Code blocks, file names (`T-0007-slug.md`) and link targets are left alone.
// shortcut: a line scan, not a Markdown parse: an ID in an indented code block or inside a link's text is linked too.
pub fn link_ids(body: &str, known: &dyn Fn(&str) -> bool) -> (String, Vec<String>) {
    let (mut out, mut ids, mut fence) = (String::with_capacity(body.len()), Vec::new(), false);
    for line in body.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        }
        let parts: Vec<&str> = line.split('`').collect();
        // Inside a fence, or a backtick left open: as it is.
        if fence || t.starts_with("```") || t.starts_with("~~~") || parts.len().is_multiple_of(2) {
            out.push_str(line);
            continue;
        }
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 0 {
                link_text(part, known, &mut out, &mut ids);
                continue;
            }
            let id = part.split('#').next().unwrap_or("");
            if !id.is_empty() && known(id) {
                out.push_str(&format!("[`{part}`](mwp:{id})"));
                ids.push(id.to_string());
            } else {
                out.push_str(&format!("`{part}`"));
            }
        }
    }
    (out, ids)
}

/// The project IDs in plain text made links; see `link_ids`.
fn link_text(s: &str, known: &dyn Fn(&str) -> bool, out: &mut String, ids: &mut Vec<String>) {
    let b = s.as_bytes();
    let joined = |c: u8| c.is_ascii_alphanumeric() || c == b'-' || c == b'_';
    let (mut i, mut last) = (0, 0);
    while i < b.len() {
        // An ASCII capital first, so `s[i..]` is on a char boundary.
        let starts = b[i].is_ascii_uppercase() && (i == 0 || !(joined(b[i - 1]) || matches!(b[i - 1], b'/' | b'[' | b'(')));
        if let Some(p) = LINKED.iter().find(|p| starts && s[i..].starts_with(*p)) {
            let digits = s[i + p.len()..].bytes().take_while(u8::is_ascii_digit).count();
            let end = i + p.len() + digits;
            // Bytes, but only cut at ASCII positions, so always on a char boundary.
            if digits > 0 && (end == b.len() || !(joined(b[end]) || b[end] == b']')) && known(&s[i..end]) {
                let id = &s[i..end];
                out.push_str(&s[last..i]);
                out.push_str(&format!("[{id}](mwp:{id})"));
                ids.push(id.to_string());
                (i, last) = (end, end);
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&s[last..]);
}

/// The word under a char position, as an ID to look up: `T-0007`, `order` (`order#FR-01` gives `order`).
fn id_at(text: &str, at: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let part = |c: &char| c.is_alphanumeric() || matches!(c, '-' | '_' | '#');
    let start = chars[..at.min(chars.len())].iter().rposition(|c| !part(c)).map_or(0, |i| i + 1);
    let end = chars[start..].iter().position(|c| !part(c)).map_or(chars.len(), |i| start + i);
    let word: String = chars[start..end].iter().collect();
    word.split('#').next().unwrap_or("").to_string()
}

/// The rendered file; returns the file of an ID link that was clicked.
fn preview(ui: &mut egui::Ui, ed: &mut Editor, find_id: FindId) -> Option<PathBuf> {
    let mut open = None;
    egui::Frame::NONE.inner_margin(egui::Margin::symmetric(16, 8)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt(("preview", &ed.path)).auto_shrink([false, false]).show(ui, |ui| {
            let (fm, body) = split_front_matter(&ed.text).unwrap_or(("", &ed.text));
            if !fm.is_empty() {
                // Front matter as a property table instead of raw YAML.
                egui::Frame::group(ui.style()).inner_margin(10).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::Grid::new(("fm", &ed.path)).num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                        for (k, v) in fm.lines().filter_map(|l| l.split_once(':')).filter(|(k, _)| !k.trim_start().starts_with('#')) {
                            let v = v.split(" #").next().unwrap_or("").trim();
                            let v = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')).unwrap_or(v).trim();
                            if !v.is_empty() {
                                ui.weak(k.trim());
                                // Values that name other docs (modules, refs, owner...) link to them; not the doc's own id.
                                let items: Vec<(&str, Option<PathBuf>)> = v.split(',').map(|x| x.trim().trim_matches(['"', '\''])).map(|x| (x, if k.trim() == "id" { None } else { find_id(x) })).collect();
                                if items.iter().any(|x| x.1.is_some()) {
                                    let count = items.len();
                                    ui.horizontal_wrapped(|ui| {
                                        for (n, (item, path)) in items.into_iter().enumerate() {
                                            let last = if n + 1 < count { "," } else { "" };
                                            match path {
                                                Some(p) if ui.link(format!("{item}{last}")).on_hover_text(tr("Open file")).clicked() => open = Some(p),
                                                Some(_) => {}
                                                None => {
                                                    ui.label(format!("{item}{last}"));
                                                }
                                            }
                                        }
                                    });
                                } else {
                                    ui.label(v);
                                }
                                ui.end_row();
                            }
                        }
                    });
                });
                ui.add_space(8.0);
            }
            let (body, ids) = link_ids(body, &|id| find_id(id).is_some());
            ed.cache.link_hooks_clear();
            for id in &ids {
                ed.cache.add_link_hook(format!("mwp:{id}"));
            }
            CommonMarkViewer::new().show(ui, &mut ed.cache, &body);
            if let Some(id) = ids.iter().find(|id| ed.cache.get_link_hook(&format!("mwp:{id}")) == Some(true)) {
                open = find_id(id);
            }
        });
    });
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(text: &str, a: usize, b: usize) -> Editor {
        Editor { path: "x.md".into(), text: text.into(), saved: String::new(), crlf: false, cursor: Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))), cache: CommonMarkCache::default(), disk: None, show_disk: false, gone: false, find: None }
    }

    #[test]
    fn formatting_wraps_and_toggles() {
        let mut e = editor("make it bold", 8, 12);
        wrap(&mut e, "**", "**");
        assert_eq!(e.text, "make it **bold**");

        let mut e = editor("one\ntwo\nthree", 1, 5); // touches lines 1 and 2
        prefix(&mut e, "- ");
        assert_eq!(e.text, "- one\n- two\nthree");
        e.cursor = Some(CCursorRange::two(CCursor::new(0), CCursor::new(7)));
        prefix(&mut e, "- ");
        assert_eq!(e.text, "one\ntwo\nthree");

        let mut e = editor("## Título", 3, 3);
        prefix(&mut e, "# ");
        assert_eq!(e.text, "# Título"); // headings replace each other, accents survive
    }

    #[test]
    fn disk_changes_never_lose_edits() {
        let f = std::env::temp_dir().join(format!("mwp-conflict-{}.md", std::process::id()));
        fs::write(&f, "base\n").unwrap();
        let mut ed = Editor::open(&f).unwrap();

        // Clean tab: follows the disk.
        fs::write(&f, "agent 1\n").unwrap();
        ed.refresh();
        assert_eq!((ed.text.as_str(), ed.disk.is_none()), ("agent 1\n", true));

        // Edits plus a change on disk, even one the refresh has not seen yet: the save is refused.
        ed.text.push_str("mine\n");
        fs::write(&f, "agent 2\n").unwrap();
        assert!(ed.save().is_err());
        assert_eq!(fs::read_to_string(&f).unwrap(), "agent 2\n");
        assert_eq!(ed.disk.as_deref(), Some("agent 2\n"));
        ed.refresh();
        assert!(ed.dirty() && ed.disk.is_some(), "the refresh keeps the edits and the conflict");

        // Overwrite with mine.
        ed.resolve(Resolve::Mine);
        ed.save().unwrap();
        assert_eq!(fs::read_to_string(&f).unwrap(), "agent 1\nmine\n");

        // Load the disk version.
        ed.text.push_str("more\n");
        fs::write(&f, "agent 3\r\n").unwrap();
        ed.refresh();
        ed.resolve(Resolve::Disk);
        assert!(!ed.dirty() && ed.disk.is_none() && ed.text == "agent 3\n" && ed.crlf);

        // Deleted on disk: the tab says so and saving never brings the file back by itself.
        fs::remove_file(&f).unwrap();
        ed.refresh();
        assert!(ed.gone);
        ed.text.push_str("kept\n");
        assert!(ed.save().is_err() && !f.exists());
        // Save again writes it from the tab.
        ed.recreate().unwrap();
        assert!(!ed.gone && fs::read_to_string(&f).unwrap() == "agent 3\r\nkept\r\n");
        let _ = fs::remove_file(&f);
    }

    #[test]
    fn line_diff_marks_each_side() {
        let (mine, disk) = ("a\nb\nmine\nc\nd\n", "a\nb\ndisk\nc\nd\nnew\n");
        assert_eq!(line_diff(mine, disk), [(' ', "a"), (' ', "b"), ('-', "mine"), ('+', "disk"), (' ', "c"), (' ', "d"), ('+', "new")]);
        assert_eq!(line_diff("x\n", "x\n"), [(' ', "x")]);
        assert_eq!(line_diff("", "y\n"), [('+', "y")]);
        assert_eq!(line_diff("y\nz\n", ""), [('-', "y"), ('-', "z")]);
    }

    #[test]
    fn find_matches_and_wraps() {
        let text = "Pão alpha ALPHA\nalphaalpha é";
        assert_eq!(matches(text, "alpha", false), [(4, 9), (10, 15), (16, 21), (21, 26)]);
        assert_eq!(matches(text, "alpha", true), [(4, 9), (16, 21), (21, 26)]);
        assert_eq!(matches(text, "ÃO", false), [(1, 3)]);
        assert!(matches(text, "", false).is_empty() && matches(text, "zzz", false).is_empty());

        let mut ed = editor(text, 0, 0);
        ed.open_find(false);
        ed.find.as_mut().unwrap().query = "alpha".into();
        assert_eq!(ed.find_step(true, false), Some((4, 9)));
        ed.find.as_mut().unwrap().current = Some((4, 9));
        assert_eq!(ed.find_step(true, false), Some((10, 15)));
        assert_eq!(ed.find_step(true, true), Some((4, 9)), "from the start, the current match is found again");
        assert_eq!(ed.find_step(false, false), Some((21, 26)), "back from the first wraps to the last");

        // The selection on one line seeds the query and the current match.
        let mut ed = editor(text, 4, 9);
        ed.open_find(false);
        let f = ed.find.as_ref().unwrap();
        assert_eq!((f.query.as_str(), f.current, f.replace.is_none()), ("alpha", Some((4, 9)), true));
    }

    #[test]
    fn ids_become_links_only_when_they_exist() {
        let known = |id: &str| ["T-0007", "SP-0001", "order"].contains(&id);
        let body = "See — T-0007, T-0099 and SP-0001.\nNot TEST-0007, T-0007-slug.md, [T-0007](x) or mwp/T-0007.\nModule `order`, `order#FR-01`, `nope`.\n```\nT-0007 `order`\n```\nUnclosed `T-0007\n";
        let (out, ids) = link_ids(body, &known);
        assert_eq!(out, "See — [T-0007](mwp:T-0007), T-0099 and [SP-0001](mwp:SP-0001).\nNot TEST-0007, T-0007-slug.md, [T-0007](x) or mwp/T-0007.\nModule [`order`](mwp:order), [`order#FR-01`](mwp:order), `nope`.\n```\nT-0007 `order`\n```\nUnclosed `T-0007\n");
        assert_eq!(ids, ["T-0007", "SP-0001", "order", "order"]);
        // Ctrl+click: the word under the cursor.
        let text = "uses order#FR-01 and  T-0007. é";
        assert_eq!((id_at(text, 6), id_at(text, 24), id_at(text, 21)), ("order".into(), "T-0007".into(), "".into()));
    }

    #[test]
    fn replace_one_then_all() {
        let mut ed = editor("é cat, Cat; cat", 0, 0);
        ed.open_find(true);
        let f = ed.find.as_mut().unwrap();
        (f.query, f.replace) = ("cat".into(), Some("lion cat".into()));
        // Nothing current yet: the first press only goes to the first match.
        ed.replace_current();
        assert_eq!((ed.text.as_str(), ed.find.as_ref().unwrap().current), ("é cat, Cat; cat", Some((2, 5))));
        // Replaced, and the next match is after the new text (which itself contains "cat").
        ed.replace_current();
        assert_eq!((ed.text.as_str(), ed.find.as_ref().unwrap().current), ("é lion cat, Cat; cat", Some((12, 15))));
        ed.find.as_mut().unwrap().case = true;
        assert_eq!(ed.replace_all(), 2, "Match case leaves Cat");
        assert_eq!(ed.text, "é lion lion cat, Cat; lion cat");
    }

    #[test]
    fn highlight_keeps_every_character() {
        let text = "---\nkind: task\n---\n# T — Title\n- [ ] item with `code` and <placeholder>\n```\nfn x() {}\n```\n";
        let p = palette(&egui::Visuals::dark());
        assert_eq!(highlight(text, FontId::monospace(12.0), &p).text, text);
        // A BOM does not hide the front matter.
        let bom = format!("\u{feff}{text}");
        assert_eq!(highlight(&bom, FontId::monospace(12.0), &p).text, bom);
        assert_eq!(split_front_matter(&bom).map(|f| f.0), Some("kind: task"));
    }
}
