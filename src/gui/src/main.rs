//! `mwp-gui`: an editor for MWP projects. Explorer of the mwp/ files, a Markdown editor with
//! tabs and preview, the task board, lint problems and a dialog to create a project.
//! All the MWP logic is in `mwp_core`.

mod board;
mod dialogs;
mod editor;
mod explorer;
mod layout;
mod search;
mod tabs;

use board::*;
use dialogs::*;
use editor::{Editor, Mode};
use explorer::*;
use layout::{project_key, rel_to, Layout};
use search::*;
use std::collections::BTreeMap;
use tabs::*;
use eframe::egui::{self, Color32, RichText};
use egui_material_icons::{icons::*, MaterialIcon};
use mwp_core::doc::{id_num, slug, walk, Doc, Project};
use mwp_core::lint::{lint, Report};
use mwp_core::ops;
use mwp_core::scaffold::{self, AGENTS};
use mwp_core::state::{self, Claim};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// shortcut: the project is reread on a timer, not on file events; add a file watcher
// if rereading every 2 s gets slow on a very large project.
const REFRESH: Duration = Duration::from_secs(2);

/// How lint reports a stale map.md; that problem gets an "Update map.md" button.
const MAP_STALE: &str = "mwp/map.md: out of date";
const fn key(modifiers: egui::Modifiers, key: egui::Key) -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(modifiers, key)
}
const CMD: egui::Modifiers = egui::Modifiers::COMMAND;
const SAVE: egui::KeyboardShortcut = key(CMD, egui::Key::S);
const SAVE_ALL: egui::KeyboardShortcut = key(CMD.plus(egui::Modifiers::SHIFT), egui::Key::S);
const FIND: egui::KeyboardShortcut = key(CMD, egui::Key::F);
const REPLACE: egui::KeyboardShortcut = key(CMD, egui::Key::H);
const SEARCH: egui::KeyboardShortcut = key(CMD.plus(egui::Modifiers::SHIFT), egui::Key::F);
const CLOSE_TAB: egui::KeyboardShortcut = key(CMD, egui::Key::W);
/// Next / previous tab. The Shift ones come first: Ctrl+Tab would also match Ctrl+Shift+Tab.
const TAB_STEPS: [(egui::KeyboardShortcut, bool); 4] = [
    (key(CMD.plus(egui::Modifiers::SHIFT), egui::Key::Tab), false),
    (key(CMD, egui::Key::Tab), true),
    (key(CMD, egui::Key::PageUp), false),
    (key(CMD, egui::Key::PageDown), true),
];

/// The Edit menu. Each command is replayed into the source view as the key or clipboard
/// event the text box already handles.
#[derive(Clone, Copy)]
enum EditCmd {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

const EDIT_MENU: [(&str, MaterialIcon, EditCmd, egui::KeyboardShortcut); 6] = [
    ("Undo", ICON_UNDO, EditCmd::Undo, key(CMD, egui::Key::Z)),
    ("Redo", ICON_REDO, EditCmd::Redo, key(CMD, egui::Key::Y)),
    ("Cut", ICON_CONTENT_CUT, EditCmd::Cut, key(CMD, egui::Key::X)),
    ("Copy", ICON_CONTENT_COPY, EditCmd::Copy, key(CMD, egui::Key::C)),
    ("Paste", ICON_CONTENT_PASTE, EditCmd::Paste, key(CMD, egui::Key::V)),
    ("Select all", ICON_SELECT_ALL, EditCmd::SelectAll, key(CMD, egui::Key::A)),
];

/// A button text with its icon in front.
fn ic(icon: MaterialIcon, text: &str) -> (RichText, &str) {
    (icon.rich_text(), text)
}

/// The file a lint line is about: lines are `mwp/path.md: message`.
fn problem_path(root: &Path, line: &str) -> Option<PathBuf> {
    line.split_once(": ").map(|(path, _)| root.join(path)).filter(|p| p.is_file())
}

/// A change of what is open; asked first when it would drop unsaved edits.
enum Action {
    Open(PathBuf),
    Board,
    CloseTab(usize),
    /// Close several tabs (Close others, Close to the right); unsaved ones are asked about once.
    CloseTabs(Vec<usize>),
    /// Move the tab at the first index to the second; the open tab stays open.
    MoveTab(usize, usize),
    /// Switch to the project in this folder; `true` also opens its PROJECT.md.
    Project(PathBuf, bool),
    Quit,
}

/// What the panels asked for in one frame; run once they are all drawn.
#[derive(Default)]
struct Asks {
    action: Option<Action>,
    new_task: bool,
    new_module: bool,
    new_spec: bool,
    new_adr: bool,
    ask_delete: Option<PathBuf>,
    folder_cmd: Option<FolderCmd>,
    task_cmd: Option<TaskCmd>,
    open_error: Option<String>,
    shown_tab: Option<PathBuf>,
    /// A project search match to select once its file is open (chars in the file).
    reveal: Option<(PathBuf, (usize, usize))>,
}

struct App {
    root: PathBuf,
    view: Result<View, String>,
    loaded: Instant,
    tabs: Vec<Editor>,
    /// The tab shown; `None` shows the board.
    active: Option<usize>,
    /// The open tab the strip last scrolled to.
    shown_tab: Option<PathBuf>,
    mode: Mode,
    /// An Edit menu command and how many frames until it runs (focus first, then the event).
    edit_cmd: Option<(EditCmd, u8)>,
    pending: Option<Action>,
    dialog: Option<Dialog>,
    /// The last projects opened, newest first.
    recent: Vec<PathBuf>,
    message: Option<String>,
    show_done: bool,
    /// The bottom panel is open; `show_search` picks its Search tab over Problems.
    show_problems: bool,
    show_search: bool,
    search: Search,
    title: String,
    /// The tabs each project had open, kept between runs (see `layout`).
    saved_tabs: BTreeMap<PathBuf, (Option<String>, Vec<String>)>,
}

impl App {
    fn new(root: PathBuf) -> App {
        let mut app = App {
            view: load(&root),
            recent: recent_file().and_then(|f| fs::read_to_string(f).ok()).map_or_else(Vec::new, |t| t.trim_start_matches('\u{feff}').lines().map(PathBuf::from).filter(|p| p.join("mwp").is_dir()).collect()),
            root,
            loaded: Instant::now(),
            tabs: Vec::new(),
            active: None,
            shown_tab: None,
            mode: Mode::Split,
            edit_cmd: None,
            pending: None,
            dialog: None,
            message: None,
            show_done: false,
            show_problems: true,
            show_search: false,
            search: Search::default(),
            title: String::new(),
            saved_tabs: BTreeMap::new(),
        };
        app.remember();
        app
    }

    /// Puts the open project first in the recent list and saves the list.
    fn remember(&mut self) {
        if self.view.is_ok() {
            self.recent = remember(&self.recent, &self.root);
            if let Some(f) = recent_file() {
                let text: String = self.recent.iter().map(|p| format!("{}\n", p.display())).collect();
                // Losing the list is harmless: it is rebuilt as projects are opened.
                let _ = fs::create_dir_all(f.parent().unwrap()).and_then(|_| fs::write(f, text));
            }
        }
    }

    /// The app with its icon font and a little more room than egui's default spacing, set up
    /// as it was left last time.
    fn start(cc: &eframe::CreationContext, root: PathBuf) -> App {
        egui_material_icons::initialize(&cc.egui_ctx);
        cc.egui_ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 5.0);
            s.spacing.button_padding = egui::vec2(8.0, 3.0);
        });
        let mut app = App::new(root);
        if let Some(text) = cc.storage.and_then(|s| s.get_string(layout::KEY)) {
            let l = Layout::parse(&text);
            (app.mode, app.show_done, app.show_problems, app.saved_tabs) = (l.mode, l.show_done, l.show_problems, l.tabs);
            app.restore_tabs();
        }
        app
    }

    /// Records the open tabs of this project in `saved_tabs`.
    fn keep_tabs(&mut self) {
        if self.view.is_ok() {
            let tabs = self.tabs.iter().map(|e| rel_to(&self.root, &e.path)).collect();
            let active = self.current().map(|e| rel_to(&self.root, &e.path));
            self.saved_tabs.insert(project_key(&self.root), (active, tabs));
        }
    }

    /// Opens the tabs this project had last time, those whose file is still there.
    fn restore_tabs(&mut self) {
        let Some((active, tabs)) = self.saved_tabs.get(&project_key(&self.root)).cloned() else { return };
        self.tabs = tabs.iter().filter_map(|r| Editor::open(&self.root.join(r)).ok()).collect();
        self.active = active.and_then(|a| self.tabs.iter().position(|e| rel_to(&self.root, &e.path) == a));
    }

    fn reload(&mut self) {
        self.view = load(&self.root);
        self.loaded = Instant::now();
        self.tabs.iter_mut().for_each(Editor::refresh);
    }

    fn current(&self) -> Option<&Editor> {
        self.active.and_then(|i| self.tabs.get(i))
    }

    /// Shows the Search tab of the bottom panel, seeded with the editor's selection.
    fn open_search(&mut self) {
        if self.view.is_ok() {
            self.search.open(self.current().map(Editor::selected));
            (self.show_problems, self.show_search) = (true, true);
        }
    }

    fn save_tab(&mut self, i: usize) -> bool {
        let Some(ed) = self.tabs.get_mut(i) else { return true };
        match ed.save() {
            Ok(()) => {
                self.message = None;
                self.reload();
                true
            }
            Err(e) => {
                self.message = Some(e);
                false
            }
        }
    }

    fn save_all(&mut self) -> bool {
        (0..self.tabs.len()).filter(|&i| self.tabs[i].dirty()).collect::<Vec<_>>().into_iter().all(|i| self.save_tab(i))
    }

    /// Runs the action, or asks first when it would drop unsaved edits.
    fn request(&mut self, ctx: &egui::Context, a: Action) {
        let blocked = match &a {
            Action::CloseTab(i) => self.tabs.get(*i).is_some_and(Editor::dirty),
            Action::CloseTabs(list) => list.iter().any(|&i| self.tabs.get(i).is_some_and(Editor::dirty)),
            Action::Project(..) | Action::Quit => self.tabs.iter().any(Editor::dirty),
            Action::Open(_) | Action::Board | Action::MoveTab(..) => false,
        };
        if blocked {
            self.pending = Some(a);
        } else {
            self.apply(ctx, a);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::Open(path) => match self.tabs.iter().position(|t| t.path == path) {
                Some(i) => self.active = Some(i),
                None => match Editor::open(&path) {
                    Ok(ed) => {
                        self.tabs.push(ed);
                        self.active = Some(self.tabs.len() - 1);
                    }
                    Err(e) => self.message = Some(e),
                },
            },
            Action::Board => self.active = None,
            Action::CloseTab(i) => self.apply(ctx, Action::CloseTabs(vec![i])),
            Action::CloseTabs(mut list) => {
                let open = self.current().map(|e| e.path.clone());
                let was = self.active;
                list.retain(|&i| i < self.tabs.len());
                list.sort_unstable();
                list.dedup();
                for &i in list.iter().rev() {
                    self.tabs.remove(i);
                }
                // The open tab stays open; when it was closed, the one now in its place opens.
                self.active = match open.and_then(|p| self.tabs.iter().position(|e| e.path == p)) {
                    Some(i) => Some(i),
                    None => was.filter(|_| !self.tabs.is_empty()).map(|a| a.min(self.tabs.len() - 1)),
                };
            }
            Action::MoveTab(from, to) if from < self.tabs.len() && to < self.tabs.len() => {
                let open = self.current().map(|e| e.path.clone());
                let tab = self.tabs.remove(from);
                self.tabs.insert(to, tab);
                self.active = open.and_then(|p| self.tabs.iter().position(|e| e.path == p));
            }
            Action::MoveTab(..) => {}
            Action::Project(dir, open_project) => {
                self.keep_tabs();
                self.root = dir;
                (self.tabs, self.active) = (Vec::new(), None);
                self.reload();
                self.remember();
                self.restore_tabs();
                if open_project {
                    self.apply(ctx, Action::Open(self.root.join("mwp/PROJECT.md")));
                }
            }
            Action::Quit => {
                self.tabs.clear(); // nothing left to guard
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Replays an Edit menu command: the menu took the focus, so give it back first.
    fn run_edit_cmd(&mut self, ctx: &egui::Context) {
        let Some((cmd, wait)) = self.edit_cmd.take() else { return };
        let Some(id) = self.current().map(Editor::id) else { return };
        if wait > 0 {
            ctx.memory_mut(|m| m.request_focus(id));
            self.edit_cmd = Some((cmd, wait - 1));
            return;
        }
        match cmd {
            EditCmd::Cut => ctx.send_viewport_cmd(egui::ViewportCommand::RequestCut),
            EditCmd::Copy => ctx.send_viewport_cmd(egui::ViewportCommand::RequestCopy),
            EditCmd::Paste => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
            EditCmd::Undo | EditCmd::Redo | EditCmd::SelectAll => {
                let sc = EDIT_MENU.iter().find(|m| std::mem::discriminant(&m.2) == std::mem::discriminant(&cmd)).unwrap().3;
                let event = egui::Event::Key { key: sc.logical_key, physical_key: None, pressed: true, repeat: false, modifiers: sc.modifiers };
                ctx.input_mut(|i| i.events.push(event));
            }
        }
    }


    /// Copies files into mwp/content/<folder>; names already there are kept and listed in a notice.
    fn add_content(&mut self, folder: &str, files: &[PathBuf]) {
        match Project::open(&self.root).and_then(|p| ops::content_add(&p, folder, files)) {
            Ok(lines) => {
                let skipped: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("skip ")).collect();
                if !skipped.is_empty() {
                    let text = format!("Existing files are never replaced. Not added:\n{}", skipped.join("\n"));
                    self.dialog = Some(Dialog::Notice { title: format!("{} file(s) not added", skipped.len()), text });
                }
            }
            Err(e) => self.dialog = Some(Dialog::Notice { title: "Files not added".into(), text: e }),
        }
        self.reload();
    }
}

impl eframe::App for App {
    /// Called by eframe now and then and on exit; the window and panels it keeps itself.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.keep_tabs();
        let l = Layout { mode: self.mode, show_done: self.show_done, show_problems: self.show_problems, tabs: self.saved_tabs.clone() };
        storage.set_string(layout::KEY, l.text());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.loaded.elapsed() >= REFRESH {
            self.reload();
        }
        ctx.request_repaint_after(REFRESH);
        if ctx.input(|i| i.viewport().close_requested()) && self.tabs.iter().any(Editor::dirty) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Action::Quit);
        }
        // Save all first: Ctrl+S would also match Ctrl+Shift+S.
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE_ALL)) {
            self.save_all();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE)) {
            if let Some(i) = self.active {
                self.save_tab(i);
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&CLOSE_TAB)) {
            if let Some(i) = self.active {
                self.request(&ctx, Action::CloseTab(i));
            }
        }
        for (sc, forward) in TAB_STEPS {
            if ctx.input_mut(|i| i.consume_shortcut(&sc)) && !self.tabs.is_empty() {
                // From the board, the first (or last) tab.
                let n = self.tabs.len();
                self.active = Some(match self.active {
                    Some(i) if forward => (i + 1) % n,
                    Some(i) => (i + n - 1) % n,
                    None if forward => 0,
                    None => n - 1,
                });
            }
        }
        // Find in project first: Ctrl+F would also match Ctrl+Shift+F.
        if ctx.input_mut(|i| i.consume_shortcut(&SEARCH)) {
            self.open_search();
        }
        for (sc, replace) in [(FIND, false), (REPLACE, true)] {
            if ctx.input_mut(|i| i.consume_shortcut(&sc)) {
                if let Some(i) = self.active {
                    self.tabs[i].open_find(replace);
                }
            }
        }
        self.run_edit_cmd(&ctx);

        let title = match self.current() {
            Some(ed) => format!("{}{} — MWP", if ed.dirty() { "● " } else { "" }, ed.name()),
            None => format!("{} — MWP", self.root.display()),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        let mut ask = Asks::default();
        let (mut save, mut save_all) = (false, false);
        let mut reload = false;
        let (mut new_dialog, mut open_dialog) = (false, false);
        let mut update_map = false;
        let mut resolve: Option<(usize, editor::Resolve)> = None;
        // The open file, when it is one the editor may delete.
        let deletable = match (&self.view, self.current()) {
            (Ok(v), Some(ed)) if ops::deletable(&v.project, &ed.path) => Some(ed.path.clone()),
            _ => None,
        };

        egui::Panel::top("menu").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(ic(ICON_CREATE_NEW_FOLDER, "New project…")).clicked() {
                        self.dialog = Some(self.new_project_dialog());
                    }
                    if ui.button(ic(ICON_FOLDER_OPEN, "Open project…")).clicked() {
                        self.dialog = Some(Dialog::Open { dir: self.root.display().to_string(), error: String::new() });
                    }
                    ui.separator();
                    let project = self.view.is_ok();
                    ask.new_task = ui.add_enabled(project, egui::Button::new(ic(ICON_ADD_TASK, "New task…"))).clicked();
                    ask.new_module = ui.add_enabled(project, egui::Button::new(ic(ICON_EXTENSION, "New module…"))).clicked();
                    ask.new_spec = ui.add_enabled(project, egui::Button::new(ic(ICON_ARTICLE, "New SPEC…"))).clicked();
                    ask.new_adr = ui.add_enabled(project, egui::Button::new(ic(ICON_GAVEL, "New ADR…"))).clicked();
                    ui.separator();
                    let dirty = self.current().is_some_and(Editor::dirty);
                    save = ui.add_enabled(dirty, egui::Button::new(ic(ICON_SAVE, "Save")).shortcut_text(ctx.format_shortcut(&SAVE))).clicked();
                    let any = self.tabs.iter().any(Editor::dirty);
                    save_all = ui.add_enabled(any, egui::Button::new(ic(ICON_SAVE_AS, "Save all")).shortcut_text(ctx.format_shortcut(&SAVE_ALL))).clicked();
                    if ui.add_enabled(self.active.is_some(), egui::Button::new(ic(ICON_CLOSE, "Close file")).shortcut_text(ctx.format_shortcut(&CLOSE_TAB))).clicked() {
                        ask.action = self.active.map(Action::CloseTab);
                    }
                    if ui.add_enabled(deletable.is_some(), egui::Button::new(ic(ICON_DELETE, "Delete file…"))).on_disabled_hover_text("Only modules, specs and open tasks can be deleted").clicked() {
                        ask.ask_delete = deletable.clone();
                    }
                    ui.separator();
                    reload = ui.button(ic(ICON_REFRESH, "Reload")).clicked();
                    ui.separator();
                    if ui.button(ic(ICON_LOGOUT, "Quit")).clicked() {
                        ask.action = Some(Action::Quit);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let on = self.active.is_some() && self.mode != Mode::Preview;
                    for (i, (label, icon, cmd, sc)) in EDIT_MENU.iter().enumerate() {
                        if i == 2 || i == 5 {
                            ui.separator();
                        }
                        if ui.add_enabled(on, egui::Button::new(ic(*icon, label)).shortcut_text(ctx.format_shortcut(sc))).clicked() {
                            self.edit_cmd = Some((*cmd, 1));
                        }
                    }
                    ui.separator();
                    for (label, icon, sc, replace) in [("Find…", ICON_SEARCH, FIND, false), ("Replace…", ICON_FIND_REPLACE, REPLACE, true)] {
                        let item = egui::Button::new(ic(icon, label)).shortcut_text(ctx.format_shortcut(&sc));
                        if ui.add_enabled(self.active.is_some(), item).clicked() {
                            if let Some(i) = self.active {
                                self.tabs[i].open_find(replace);
                            }
                        }
                    }
                    let search = egui::Button::new(ic(ICON_MANAGE_SEARCH, "Find in project…")).shortcut_text(ctx.format_shortcut(&SEARCH));
                    if ui.add_enabled(self.view.is_ok(), search).clicked() {
                        self.open_search();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button(ic(ICON_VIEW_KANBAN, "Board")).clicked() {
                        ask.action = Some(Action::Board);
                    }
                    ui.separator();
                    for (m, label) in [(Mode::Edit, "Source only"), (Mode::Split, "Source and preview"), (Mode::Preview, "Preview only")] {
                        ui.radio_value(&mut self.mode, m, label);
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_done, "Done tasks on the board");
                    ui.checkbox(&mut self.show_problems, "Bottom panel (Problems, Search)");
                });
            });
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Ok(v) = &self.view {
                    let (e, w) = (v.report.errors.len(), v.report.warnings.len());
                    let tint = |n: usize, c: Color32| if n > 0 { c } else { ui.visuals().weak_text_color() };
                    let (ec, wc) = (tint(e, ui.visuals().error_fg_color), tint(w, ui.visuals().warn_fg_color));
                    let problems = (ICON_ERROR.rich_text().color(ec), RichText::new(e.to_string()).color(ec), ICON_WARNING.rich_text().color(wc), RichText::new(w.to_string()).color(wc));
                    let on = self.show_problems && !self.show_search;
                    if ui.selectable_label(on, problems).on_hover_text("Errors and warnings from mwp lint").clicked() {
                        (self.show_problems, self.show_search) = (!on, false);
                    }
                    ui.separator();
                    ui.label(ICON_SMART_TOY.rich_text());
                    ui.label(format!("{} in progress", v.claims.len()));
                    ui.separator();
                }
                match &self.message {
                    Some(m) => {
                        ui.colored_label(ui.visuals().error_fg_color, m);
                    }
                    None => dim(ui, self.root.display().to_string()),
                }
            });
        });

        match &self.view {
            Err(e) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(ICON_HUB.rich_text().size(56.0).color(ui.visuals().weak_text_color()));
                        ui.heading("No MWP project open");
                        ui.colored_label(ui.visuals().error_fg_color, e);
                        ui.add_space(12.0);
                        new_dialog = ui.button(ic(ICON_CREATE_NEW_FOLDER, "New project…")).clicked();
                        open_dialog = ui.button(ic(ICON_FOLDER_OPEN, "Open project…")).clicked();
                        ui.add_space(24.0);
                    });
                    if !self.recent.is_empty() {
                        // A centred card; rows inside are left-aligned.
                        let w = ui.available_width().min(560.0);
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - w) / 2.0);
                            ui.vertical(|ui| {
                                ui.set_width(w);
                                section(ui, "Recent", |ui| {
                                    if let Some(dir) = recent_list(ui, &self.recent) {
                                        ask.action = Some(Action::Project(dir, false));
                                    }
                                });
                            });
                        });
                    }
                });
            }
            Ok(v) => {
                self.explorer(ui, v, &mut ask);

                if self.show_problems {
                    egui::Panel::bottom("problems").resizable(true).default_size(110.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.show_search, false, RichText::new("Problems").strong());
                            ui.selectable_value(&mut self.show_search, true, RichText::new("Search").strong());
                            if !self.show_search {
                                ui.weak(format!("{} errors, {} warnings", v.report.errors.len(), v.report.warnings.len()));
                            }
                        });
                        if self.show_search {
                            search_panel(ui, v, &self.tabs, self.loaded, &mut self.search, &mut ask);
                            return;
                        }
                        egui::ScrollArea::vertical().id_salt("problems").auto_shrink([false, false]).show(ui, |ui| {
                            let errors = v.report.errors.iter().map(|l| (l, ui.visuals().error_fg_color, ICON_ERROR));
                            let warnings = v.report.warnings.iter().map(|l| (l, ui.visuals().warn_fg_color, ICON_WARNING));
                            for (line, color, icon) in errors.chain(warnings).collect::<Vec<_>>() {
                                ui.horizontal(|ui| {
                                    let r = ui.selectable_label(false, (icon.rich_text().color(color), RichText::new(line).monospace()));
                                    if r.clicked() {
                                        ask.action = problem_path(&self.root, line).map(Action::Open);
                                    }
                                    if line.starts_with(MAP_STALE) {
                                        update_map |= ui.small_button(ic(ICON_REFRESH, "Update map.md")).clicked();
                                    }
                                });
                            }
                            if v.report.errors.is_empty() && v.report.warnings.is_empty() {
                                ui.weak("No errors or warnings.");
                            }
                        });
                    });
                }

                egui::CentralPanel::default().show(ui, |ui| {
                    // The tab strip (the board is in the explorer and the View menu).
                    tab_strip(ui, v, &self.tabs, self.active, self.shown_tab.as_ref(), &mut ask);
                    match self.active.and_then(|i| self.tabs.get_mut(i)) {
                    Some(ed) => {
                        // File header: icon, title, path and what the front matter says about it.
                        let doc = v.project.docs.iter().find(|d| d.path == ed.path);
                        ui.horizontal(|ui| {
                            let (icon, color) = file_icon(ui, &v.project, &ed.path);
                            ui.label(icon.rich_text().size(28.0).color(color));
                            ui.vertical(|ui| {
                                let title = match doc {
                                    Some(d) if !d.id().is_empty() => format!("{}  {}", d.id(), d.title()).trim_end().to_string(),
                                    _ => ed.name(),
                                };
                                ui.label(RichText::new(title).heading().strong());
                                let mut meta = vec![v.project.rel(&ed.path)];
                                meta.extend(["status", "size", "type", "agent"].iter().filter_map(|k| doc.map(|d| d.get(k)).filter(|s| !s.is_empty()).map(|s| format!("{k}: {s}"))));
                                dim(ui, meta.join("   ·   "));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if deletable.is_some() && ui.button(ic(ICON_DELETE, "Delete")).on_hover_text("Delete this file").clicked() {
                                    ask.ask_delete = deletable.clone();
                                }
                                if let Some(t) = doc.filter(|d| d.kind() == "task") {
                                    task_buttons(ui, t, &mut ask.task_cmd);
                                    context_button(ui, &v.project, t, &mut ask);
                                }
                                let stale = v.report.errors.iter().any(|e| e.starts_with(MAP_STALE));
                                if stale && ed.path == v.project.mwp.join("map.md") {
                                    update_map |= ui.button(ic(ICON_REFRESH, "Update map.md")).on_hover_text("Rebuild the module table from the modules, like mwp remap").clicked();
                                }
                            });
                        });
                        ui.separator();
                        let find_id = |id: &str| v.project.docs.iter().find(|d| !d.id().is_empty() && d.id() == id).map(|d| d.path.clone());
                        let (how, open) = editor::show(ui, ed, &mut self.mode, &find_id);
                        if let Some(how) = how {
                            resolve = self.active.map(|i| (i, how));
                        }
                        if let Some(path) = open {
                            ask.action = Some(Action::Open(path));
                        }
                    }
                    None => {
                        board(ui, v, &mut self.show_done, &mut ask);
                    }
                    }
                });
            }
        }

        if new_dialog {
            self.dialog = Some(self.new_project_dialog());
        }
        if open_dialog {
            self.dialog = Some(Dialog::Open { dir: self.root.display().to_string(), error: String::new() });
        }
        if ask.new_task {
            self.dialog = self.new_task_dialog();
        }
        if ask.new_module && self.view.is_ok() {
            ask.folder_cmd = Some(FolderCmd::NewModule(String::new()));
        }
        if let Some(cmd) = ask.task_cmd {
            self.run_task_cmd(cmd);
        }
        if let Some((i, how)) = resolve {
            let mine = matches!(how, editor::Resolve::Mine);
            self.tabs[i].resolve(how);
            if mine {
                self.save_tab(i);
            }
        }
        if let Some(e) = ask.open_error {
            self.message = Some(e);
        }
        if ask.shown_tab.is_some() {
            self.shown_tab = ask.shown_tab;
        }
        // Files dropped on the window go to mwp/content/.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() && self.view.is_ok() {
            self.add_content("", &dropped);
        }
        if update_map {
            let map = self.root.join("mwp/map.md");
            if self.tabs.iter().any(|e| e.path == map && e.dirty()) {
                self.dialog = Some(Dialog::Notice { title: "Save map.md first".into(), text: "map.md has unsaved changes. Save them before it is rebuilt.".into() });
            } else {
                let done = ops::refresh_map(&self.root);
                self.message = done.starts_with("warning").then_some(done);
                self.reload();
            }
        }
        match ask.folder_cmd {
            Some(FolderCmd::NewModule(folder)) => {
                self.dialog = Some(Dialog::NewModule { folder, id: String::new(), kind: "domain", summary: String::new(), error: String::new() });
            }
            Some(FolderCmd::NewFolder(area, parent)) => self.dialog = Some(Dialog::NewFolder { area, parent, name: String::new(), error: String::new() }),
            Some(FolderCmd::AddFiles(folder)) => {
                let start = self.root.join("mwp/content").join(&folder);
                if let Some(files) = rfd::FileDialog::new().set_title("Add to content").set_directory(&start).pick_files() {
                    self.add_content(&folder, &files);
                }
            }
            Some(FolderCmd::Remove(dir)) => {
                // remove_dir fails on a folder that is not empty: nothing in it can be lost.
                if let Err(e) = fs::remove_dir(&dir) {
                    self.message = Some(format!("{}: {e}", dir.display()));
                }
                self.reload();
            }
            None => {}
        }
        if ask.new_spec {
            self.dialog = self.new_spec_dialog();
        }
        if ask.new_adr {
            self.dialog = self.new_adr_dialog();
        }
        if let Some(path) = ask.ask_delete {
            self.dialog = self.delete_dialog(path);
        }
        if let Some(a) = self.dialogs(&ctx) {
            ask.action = Some(a);
        }

        if let Some(pending) = &self.pending {
            let targets: Vec<usize> = match pending {
                Action::CloseTab(i) => vec![*i],
                Action::CloseTabs(list) => list.iter().copied().filter(|&i| self.tabs.get(i).is_some_and(Editor::dirty)).collect(),
                _ => (0..self.tabs.len()).filter(|&i| self.tabs[i].dirty()).collect(),
            };
            let names: Vec<String> = targets.iter().map(|&i| self.tabs[i].name()).collect();
            let mut choice = None;
            egui::Modal::new(egui::Id::unique("unsaved")).show(&ctx, |ui| {
                ui.set_width(380.0);
                ui.heading("Unsaved changes");
                match names.as_slice() {
                    [one] => ui.label(format!("Save the changes to {one}?")),
                    many => ui.label(format!("Save the changes to {} files?\n{}", many.len(), many.join("\n"))),
                };
                ui.weak("Changes you don't save are lost.");
                ui.separator();
                right_row(ui, |ui| {
                    for (label, c) in [("Save", "save"), ("Don't save", "discard"), ("Cancel", "cancel")] {
                        if ui.button(label).clicked() {
                            choice = Some(c);
                        }
                    }
                });
            });
            match choice {
                Some("save") if targets.iter().all(|&i| self.save_tab(i)) => {
                    let a = self.pending.take().unwrap();
                    self.apply(&ctx, a);
                }
                Some("discard") => {
                    let a = self.pending.take().unwrap();
                    self.apply(&ctx, a);
                }
                Some("cancel") => self.pending = None,
                _ => {}
            }
        }

        if save {
            if let Some(i) = self.active {
                self.save_tab(i);
            }
        }
        if save_all {
            self.save_all();
        }
        if reload {
            self.reload();
        }
        if let Some(a) = ask.action {
            self.request(&ctx, a);
        }
        // Opening a file is never held back, so its tab is there now.
        if let Some((path, at)) = ask.reveal {
            if let Some(ed) = self.tabs.iter_mut().find(|e| e.path == path) {
                ed.show_match(&self.search.query, self.search.case, at);
            }
        }
    }
}

/// The window and taskbar icon: the largest PNG stored in assets/icone/icon.ico, the file
/// build.rs also puts in the exe.
fn window_icon() -> Option<egui::IconData> {
    let ico: &[u8] = include_bytes!("../../../assets/icone/icon.ico");
    let u32_at = |i: usize| ico.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let count = u16::from_le_bytes([*ico.get(4)?, *ico.get(5)?]) as usize;
    // Each 16-byte directory entry ends with the image size and offset.
    (0..count)
        .filter_map(|i| {
            let (size, offset) = (u32_at(6 + 16 * i + 8)?, u32_at(6 + 16 * i + 12)?);
            ico.get(offset..offset + size).filter(|img| img.starts_with(b"\x89PNG"))
        })
        .max_by_key(|png| png.len())
        .and_then(|png| eframe::icon_data::from_png_bytes(png).ok())
}

fn main() -> eframe::Result {
    // The project is the folder given, or the nearest one with mwp/ from here upward.
    let root = match std::env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => Project::find().map(|p| p.root).unwrap_or_else(|_| std::env::current_dir().unwrap_or_default()),
    };
    // The app id also names the folder of eframe's saved state: <config>/mwp-gui/, next to recent.txt.
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([1280.0, 760.0]).with_app_id("mwp-gui");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("MWP", options, Box::new(|cc| Ok(Box::new(App::start(cc, root)))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn explorer_lists_docs_and_flags_broken_ones() {
        let root = std::env::temp_dir().join(format!("mwp-gui-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        scaffold::init(&root, Some("Demo"), &[&AGENTS[0]], false).unwrap();
        fs::write(root.join("mwp/tasks/open/T-0001-a.md"), "---\nkind: task\nid: T-0001\nstatus: open\nsize: S\nmodules: []\n---\n# T-0001 — Add login\n").unwrap();
        fs::write(root.join("mwp/tasks/open/T-0002-b.md"), "# no front matter\n").unwrap();

        let v = load(&root).unwrap();
        let open = &v.tree.iter().find(|s| s.0 == "Open tasks").unwrap().2;
        assert_eq!(open.iter().map(|e| (e.id.as_str(), e.title.as_str(), e.ok)).collect::<Vec<_>>(), [("T-0001", "Add login", true), ("", "T-0002-b.md", false)]);
        assert!(v.tree[0].2.iter().any(|e| e.title == "constitution.md"));

        let line = v.report.errors.iter().find(|l| l.contains("T-0002")).unwrap();
        assert_eq!(problem_path(&root, line), Some(root.join("mwp/tasks/open/T-0002-b.md")));

        // CRLF files keep their endings after an edit.
        let f = root.join("mwp/glossary.md");
        fs::write(&f, "a\r\nb\r\n").unwrap();
        let mut app = App::new(root.clone());
        app.tabs.push(Editor::open(&f).unwrap());
        app.tabs[0].text.push_str("c\n");
        assert!(app.save_tab(0));
        assert_eq!(fs::read_to_string(&f).unwrap(), "a\r\nb\r\nc\r\n");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn module_types_match_the_core() {
        assert_eq!(MODULE_TYPES.map(|t| t.0), mwp_core::lint::MODULE_TYPES);
    }

    #[test]
    fn window_icon_comes_from_the_ico() {
        let icon = window_icon().expect("icon.ico holds a PNG image");
        assert!(icon.width == icon.height && icon.width >= 256, "{}x{}", icon.width, icon.height);
    }

    #[test]
    fn recent_projects_newest_first_at_most_five() {
        let base = std::env::temp_dir().join(format!("mwp-gui-recent-{}", std::process::id()));
        let dirs: Vec<PathBuf> = (0..7).map(|i| base.join(format!("p{i}"))).collect();
        dirs.iter().for_each(|d| fs::create_dir_all(d.join("mwp")).unwrap());
        let mut list = Vec::new();
        for d in &dirs {
            list = remember(&list, d);
        }
        assert_eq!(list, dirs[2..].iter().rev().cloned().collect::<Vec<_>>());
        list = remember(&list, &dirs[4]); // reopened: moves to the front, no duplicate
        assert_eq!(list[..2], [dirs[4].clone(), dirs[6].clone()]);
        assert_eq!(list.len(), 5);
        fs::remove_dir_all(dirs[6].join("mwp")).unwrap(); // a folder that lost its project drops out
        assert!(!remember(&list, &dirs[0]).contains(&dirs[6]));
        let _ = fs::remove_dir_all(&base);
    }

    /// Drives the real UI headless: tabs, edit, save, save all, Edit menu, the unsaved guard, New project.
    #[test]
    fn edit_save_guard_and_new_project() {
        use egui::accesskit::Role;
        use egui_kittest::{kittest::{NodeT, Queryable}, Harness};

        let base = std::env::temp_dir().join(format!("mwp-gui-ui-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("app");
        scaffold::init(&root, Some("Demo"), &[&AGENTS[0]], false).unwrap();
        let task = root.join("mwp/tasks/open/T-0001-a.md");
        fs::write(&task, "---\nkind: task\nid: T-0001\nstatus: open\nsize: S\nmodules: []\n---\n# T-0001 — Add login\n").unwrap();

        let start = root.clone();
        let mut h = Harness::builder().with_size([1280.0, 760.0]).build_eframe(move |cc| App::start(cc, start));
        h.run_steps(3);

        // Open from the explorer, type, save with Ctrl+S. Labels start with the file's icon.
        h.get_by_label_contains("T-0001 Add login").click();
        h.run_steps(3);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(task.clone()));
        h.get_by_role(Role::MultilineTextInput).focus();
        h.run_steps(2);
        h.get_by_role(Role::MultilineTextInput).type_text("edited");
        h.run_steps(3);
        assert!(h.state().current().unwrap().dirty());
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::S]);
        h.run_steps(3);
        assert!(fs::read_to_string(&task).unwrap().contains("edited"));
        assert!(!h.state().current().unwrap().dirty());

        // Edit > Select all reaches the source view through the menu.
        h.get_by_label("Edit").click();
        h.run_steps(3);
        h.get_by_label_contains("Select all").click(); // the label also holds the shortcut
        h.run_steps(5);
        let id = h.state().current().unwrap().id();
        let len = h.state().current().unwrap().text.chars().count();
        let range = egui::TextEdit::load_state(&h.ctx, id).and_then(|s| s.cursor.char_range()).unwrap();
        let (a, b) = (range.primary.index.0, range.secondary.index.0);
        assert_eq!((a.min(b), a.max(b)), (0, len));

        // A second tab; File > Save all writes both.
        h.get_by_label_contains("constitution.md").click();
        h.run_steps(3);
        assert_eq!(h.state().tabs.len(), 2);
        h.state_mut().tabs.iter_mut().for_each(|t| t.text.push_str("\nall"));
        h.run_steps(2);
        h.get_all_by_label("File").next().unwrap().click(); // the menu; the module template has a "File" column
        h.run_steps(3);
        h.get_by_label_contains("Save all").click();
        h.run_steps(3);
        assert!(h.state().tabs.iter().all(|t| !t.dirty()));
        assert!(fs::read_to_string(&task).unwrap().ends_with("all") && fs::read_to_string(root.join("mwp/constitution.md")).unwrap().contains("\nall"));

        // Closing a tab with unsaved edits asks first; "Don't save" drops them.
        h.state_mut().tabs[1].text.push_str("lost");
        h.run_steps(2);
        h.get_all_by_label(ICON_CLOSE.codepoint).nth(1).unwrap().click();
        h.run_steps(3);
        assert!(h.state().pending.is_some() && h.state().tabs.len() == 2);
        h.run_steps(30); // the modal ignores clicks while it fades in
        h.get_by_label("Don't save").click();
        h.run_steps(3);
        assert_eq!(h.state().tabs.len(), 1);
        assert!(!fs::read_to_string(root.join("mwp/constitution.md")).unwrap().contains("lost"));

        // File > New module creates it from the template, adds it to map.md and opens it.
        h.get_all_by_label("File").next().unwrap().click(); // the menu; the module template has a "File" column
        h.run_steps(3);
        h.get_by_label_contains("New module…").click();
        h.run_steps(30);
        if let Some(Dialog::NewModule { id, summary, .. }) = h.state_mut().dialog.as_mut() {
            (*id, *summary) = ("User Auth".into(), "Sign in and sessions".into());
        } else {
            panic!("the New module dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        let module = root.join("mwp/modules/user-auth.md");
        assert!(h.state().dialog.is_none() && module.exists());
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(module.clone()));
        assert!(fs::read_to_string(root.join("mwp/map.md")).unwrap().contains("| user-auth | domain | Sign in and sessions |"));

        // File > New task starts with the open module checked.
        h.get_all_by_label("File").next().unwrap().click(); // the menu; the module template has a "File" column
        h.run_steps(3);
        h.get_by_label_contains("New task…").click();
        h.run_steps(30);
        if let Some(Dialog::NewTask { title, modules, .. }) = h.state_mut().dialog.as_mut() {
            assert_eq!(modules, &[("user-auth".to_string(), true)]);
            *title = "Remember me".into();
        } else {
            panic!("the New task dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        let new_task = root.join("mwp/tasks/open/T-0002-remember-me.md");
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(new_task.clone()));
        assert!(fs::read_to_string(&new_task).unwrap().contains("modules: [user-auth]"));

        // The file header's Delete asks first, then removes the file and closes its tab.
        h.get_by_label_contains("Delete").click();
        h.run_steps(30);
        assert!(matches!(h.state().dialog, Some(Dialog::Delete { .. })));
        h.get_by_label("Delete").click(); // the dialog's button; the header's has an icon
        h.run_steps(3);
        assert!(!new_task.exists() && h.state().dialog.is_none());
        assert!(h.state().tabs.iter().all(|t| t.path != new_task));

        // File > New SPEC: owned by the module open in the editor.
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(module.clone()));
        h.get_all_by_label("File").next().unwrap().click();
        h.run_steps(3);
        h.get_by_label_contains("New SPEC…").click();
        h.run_steps(30);
        if let Some(Dialog::NewSpec { title, owner, .. }) = h.state_mut().dialog.as_mut() {
            assert_eq!(owner, "user-auth");
            *title = "Session rules".into();
        } else {
            panic!("the New SPEC dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        let spec = root.join("mwp/specs/SP-0001-session-rules.md");
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(spec.clone()));
        assert!(fs::read_to_string(&spec).unwrap().contains("owner: user-auth"));

        // Right-click Modules > New folder, then right-click the folder > New module here.
        h.get_by_label_contains("Modules").click_secondary();
        h.run_steps(3);
        h.get_by_label_contains("New folder…").click();
        h.run_steps(30);
        if let Some(Dialog::NewFolder { parent, name, .. }) = h.state_mut().dialog.as_mut() {
            assert_eq!(parent, "");
            *name = "Billing".into();
        } else {
            panic!("the New folder dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        assert!(root.join("mwp/modules/billing").is_dir() && h.state().dialog.is_none());
        h.get_all_by_label_contains("billing").next().unwrap().click_secondary(); // the folder; the file header also shows the path
        h.run_steps(3);
        assert!(!h.get_by_label_contains("Delete folder").accesskit_node().is_disabled(), "an empty folder can be deleted");
        h.get_by_label_contains("New module here…").click();
        h.run_steps(30);
        if let Some(Dialog::NewModule { folder, id, summary, .. }) = h.state_mut().dialog.as_mut() {
            assert_eq!(folder, "billing");
            (*id, *summary) = ("invoice".into(), "Invoices".into());
        } else {
            panic!("the New module dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        let invoice = root.join("mwp/modules/billing/invoice.md");
        assert!(invoice.exists() && h.state().current().is_some_and(|e| e.path == invoice));
        h.get_all_by_label_contains("billing").next().unwrap().click_secondary(); // the folder; the file header also shows the path
        h.run_steps(3);
        assert!(h.get_by_label_contains("Delete folder").accesskit_node().is_disabled(), "a folder with modules cannot be deleted");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);

        // Task lifecycle from the file header: Start, Status > review, Close (refused, then done).
        let life = root.join("mwp/tasks/open/T-0009-life.md");
        fs::write(&life, "---\nkind: task\nid: T-0009\nstatus: open\nsize: S\nmodules: [user-auth]\n---\n# T-0009 — Life\n").unwrap();
        h.state_mut().reload();
        h.run_steps(3);
        h.get_by_label_contains("T-0009 Life").click();
        h.run_steps(3);
        let header = |icon: MaterialIcon, text: &str| format!("{} {text}", icon.codepoint);
        h.get_by_label(&header(ICON_PLAY_ARROW, "Start")).click();
        h.run_steps(30);
        if let Some(Dialog::StartTask { agent, modules, .. }) = h.state_mut().dialog.as_mut() {
            assert_eq!(modules, &["user-auth"]);
            *agent = "claude".into();
        } else {
            panic!("the Start dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Start").click(); // the dialog's button
        h.run_steps(3);
        let task = || Project::open(&root).unwrap().by_id("task", "T-0009").map(|t| (t.get("status").to_string(), t.get("agent").to_string(), t.path.clone()));
        assert_eq!(task().map(|t| (t.0, t.1)), Some(("in_progress".into(), "claude".into())));
        assert_eq!(state::locks(&root.join(".mwp")).len(), 1);

        h.get_by_label(&header(ICON_SWAP_HORIZ, "Status")).click();
        h.run_steps(3);
        h.get_by_label_contains("Ready for review").click();
        h.run_steps(3);
        assert_eq!(task().unwrap().0, "review");
        assert!(state::locks(&root.join(".mwp")).is_empty(), "releasing frees the locks");

        h.get_by_label(&header(ICON_DONE_ALL, "Close")).click();
        h.run_steps(30);
        assert!(matches!(&h.state().dialog, Some(Dialog::Notice { title, .. }) if title == "Cannot close T-0009"));
        h.get_by_label("OK").click();
        h.run_steps(3);
        fs::write(&life, fs::read_to_string(&life).unwrap() + "\n## Closure\n- Checked by hand.\n").unwrap();
        h.state_mut().reload();
        h.run_steps(3);
        h.get_by_label(&header(ICON_DONE_ALL, "Close")).click();
        h.run_steps(3);
        let done = root.join("mwp/tasks/done/T-0009-life.md");
        assert_eq!(task(), Some(("done".into(), "claude".into(), done.clone())));
        assert!(!life.exists() && h.state().current().is_some_and(|e| e.path == done), "the tab follows the file to tasks/done/");

        // An agent writes the open file while it has unsaved edits: the bar shows, Ctrl+S is refused,
        // "Overwrite with mine" saves the edits.
        let i = h.state().active.unwrap();
        h.state_mut().tabs[i].text.push_str("mine\n");
        fs::write(&done, "agent wrote this\n").unwrap();
        h.state_mut().reload();
        h.run_steps(3);
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::S]);
        h.run_steps(3);
        assert_eq!(fs::read_to_string(&done).unwrap(), "agent wrote this\n");
        h.get_by_label_contains("Show disk version").click();
        h.run_steps(3);
        h.get_by_label("Overwrite with mine").click();
        h.run_steps(3);
        assert!(fs::read_to_string(&done).unwrap().ends_with("mine\n") && !h.state().tabs[i].dirty() && h.state().tabs[i].disk.is_none());

        // A module summary edited on disk leaves map.md stale; the problem's button rebuilds it.
        let stale = |h: &Harness<App>| h.state().view.as_ref().unwrap().report.errors.iter().any(|e| e.starts_with(MAP_STALE));
        fs::write(&module, fs::read_to_string(&module).unwrap().replace("Sign in and sessions", "Sign in")).unwrap();
        h.state_mut().reload();
        h.run_steps(3);
        assert!(stale(&h));
        h.get_by_label_contains("Update map.md").click();
        h.run_steps(3);
        assert!(!stale(&h) && fs::read_to_string(root.join("mwp/map.md")).unwrap().contains("| user-auth | domain | Sign in |"));

        // File > New ADR, then a second one that replaces it.
        for (title, replaces) in [("Store money in cents", ""), ("Store money as decimal", "ADR-0001")] {
            h.get_all_by_label("File").next().unwrap().click();
            h.run_steps(3);
            h.get_by_label_contains("New ADR…").click();
            h.run_steps(30);
            if let Some(Dialog::NewAdr { title: t, modules, supersedes, adrs, .. }) = h.state_mut().dialog.as_mut() {
                assert_eq!(adrs.len(), if replaces.is_empty() { 0 } else { 1 });
                *t = title.into();
                *supersedes = replaces.into();
                modules.iter_mut().filter(|m| m.0 == "user-auth").for_each(|m| m.1 = true);
            } else {
                panic!("the New ADR dialog did not open");
            }
            h.run_steps(3);
            h.get_by_label("Create").click();
            h.run_steps(3);
        }
        let p = Project::open(&root).unwrap();
        assert_eq!(p.by_id("adr", "ADR-0001").unwrap().get("status"), "superseded");
        assert_eq!(p.by_id("adr", "ADR-0002").unwrap().list("modules"), ["user-auth"]);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(p.by_id("adr", "ADR-0002").unwrap().path.clone()));

        // Content: right-click > New folder, add a file (twice: never replaced), then delete it.
        h.get_all_by_label_contains("Content").next().unwrap().click_secondary();
        h.run_steps(3);
        h.get_by_label_contains("New folder…").click();
        h.run_steps(30);
        if let Some(Dialog::NewFolder { area, name, .. }) = h.state_mut().dialog.as_mut() {
            assert!(*area == Area::Content);
            *name = "icons".into();
        } else {
            panic!("the New folder dialog did not open");
        }
        h.run_steps(3);
        h.get_by_label("Create").click();
        h.run_steps(3);
        assert!(root.join("mwp/content/icons").is_dir());
        let logo = base.join("logo.png");
        fs::write(&logo, [137u8, 80, 78, 71]).unwrap();
        h.state_mut().add_content("icons", std::slice::from_ref(&logo)); // what the picker and a drop call
        h.run_steps(3);
        assert!(h.state().dialog.is_none() && root.join("mwp/content/icons/logo.png").exists());
        h.state_mut().add_content("icons", std::slice::from_ref(&logo));
        h.run_steps(30);
        assert!(matches!(&h.state().dialog, Some(Dialog::Notice { title, .. }) if title == "1 file(s) not added"));
        h.get_by_label("OK").click();
        h.run_steps(3);
        h.get_by_label_contains("logo.png").click_secondary();
        h.run_steps(3);
        h.get_by_label_contains("Delete…").click();
        h.run_steps(30);
        h.get_by_label("Delete").click();
        h.run_steps(3);
        assert!(!root.join("mwp/content/icons/logo.png").exists());

        // Find: Ctrl+F, type, Enter for the next match, Match case, Esc.
        let notes = root.join("mwp/content/notes.md");
        fs::write(&notes, "alpha beta Alpha\ngamma alpha\n").unwrap();
        h.state_mut().reload();
        let ctx = h.ctx.clone();
        h.state_mut().apply(&ctx, Action::Open(notes.clone()));
        h.run_steps(3);
        let i = h.state().active.unwrap();
        let picked = |h: &Harness<App>| h.state().tabs[i].find.as_ref().and_then(|f| f.current).unwrap();
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::F]);
        h.run_steps(3);
        assert!(h.state().tabs[i].find.is_some());
        h.get_by_role(Role::TextInput).focus(); // kittest types into the focused widget
        h.run_steps(2);
        h.get_by_role(Role::TextInput).type_text("alpha");
        h.run_steps(3);
        assert_eq!(picked(&h), (0, 5));
        h.get_by_label("1 of 3"); // panics when the count is not shown
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        assert_eq!(picked(&h), (11, 16), "Enter goes to the next match, case-insensitive");
        h.get_by_label(ICON_MATCH_CASE.codepoint).click();
        h.run_steps(3);
        h.get_by_role(Role::TextInput).focus();
        h.run_steps(2);
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        assert_eq!(picked(&h), (23, 28), "with Match case, Alpha is skipped");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        assert!(h.state().tabs[i].find.is_none());
        // Esc leaves the match selected in the source, which now has the focus.
        let r = egui::TextEdit::load_state(&h.ctx, h.state().tabs[i].id()).and_then(|s| s.cursor.char_range()).unwrap();
        let (a, b) = (r.primary.index.0, r.secondary.index.0);
        assert_eq!((a.min(b), a.max(b)), (23, 28));
        assert!(h.ctx.memory(|m| m.has_focus(h.state().tabs[i].id())));

        // Replace: Ctrl+H shows the Replace row; Replace all, then one Ctrl+Z in the source undoes it.
        let before = h.state().tabs[i].text.clone();
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::H]);
        h.run_steps(3);
        assert!(h.state().tabs[i].find.as_ref().is_some_and(|f| f.query == "alpha" && f.replace.is_some()), "seeded from the selection");
        h.get_all_by_role(Role::TextInput).nth(1).unwrap().focus();
        h.run_steps(2);
        h.get_all_by_role(Role::TextInput).nth(1).unwrap().type_text("omega");
        h.run_steps(3);
        h.get_by_label("Replace all").click();
        h.run_steps(3);
        assert_eq!(h.state().tabs[i].text, "omega beta omega\ngamma omega\n");
        let source = h.state().tabs[i].id();
        h.ctx.memory_mut(|m| m.request_focus(source));
        h.run_steps(2);
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::Z]);
        h.run_steps(3);
        assert_eq!(h.state().tabs[i].text, before);
        h.state_mut().tabs[i].find = None;

        // Find in project: Ctrl+Shift+F, type, click a match: its file opens with the match selected.
        let ctx = h.ctx.clone();
        h.state_mut().apply(&ctx, Action::Board);
        h.key_combination_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, &[egui::Key::F]);
        h.run_steps(3);
        assert!(h.state().show_search);
        h.get_by_role(Role::TextInput).focus();
        h.run_steps(2);
        h.get_by_role(Role::TextInput).type_text("gamma");
        h.run_steps(3);
        h.get_by_label("1 result in 1 file");
        h.get_by_label("gamma alpha").click();
        h.run_steps(3);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(notes.clone()));
        assert_eq!(h.state().current().and_then(|e| e.find.as_ref()).and_then(|f| f.current), Some((17, 22)));
        h.state_mut().show_search = false;

        // Right-click a tab > Move left: the order changes, the open tab stays open.
        let before: Vec<PathBuf> = h.state().tabs.iter().map(|t| t.path.clone()).collect();
        h.get_by_role_and_label(Role::Tab, "notes.md").click_secondary(); // in view: the open tab is scrolled to
        h.run_steps(3);
        h.get_by_label_contains("Move left").click();
        h.run_steps(3);
        let after: Vec<PathBuf> = h.state().tabs.iter().map(|t| t.path.clone()).collect();
        assert_eq!((after[i - 1].clone(), after[i].clone()), (before[i].clone(), before[i - 1].clone()));
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(notes.clone()));

        // An ID in the preview links to its file; one that does not exist stays text.
        let t1 = root.join("mwp/tasks/open/T-0001-a.md");
        assert!(t1.exists());
        fs::write(&notes, "See T-0001 and T-0404.\n").unwrap();
        h.state_mut().reload();
        h.run_steps(3);
        assert!(h.query_by_label("T-0404").is_none());
        h.get_by_label("T-0001").click();
        h.run_steps(3);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(t1.clone()));

        // Ctrl+Tab / Ctrl+Shift+Tab step through the tabs, wrapping; Ctrl+W closes the open one.
        let (n, at) = (h.state().tabs.len(), h.state().active.unwrap());
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::Tab]);
        h.run_steps(3);
        assert_eq!(h.state().active, Some((at + 1) % n));
        h.key_combination_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, &[egui::Key::Tab]);
        h.run_steps(3);
        assert_eq!(h.state().active, Some(at));
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::W]);
        h.run_steps(3);
        assert!(h.state().tabs.len() == n - 1 && !h.state().tabs.iter().any(|t| t.path == t1));

        // Close others: the unsaved tabs among them are asked about once; the open tab stays.
        let n = h.state().tabs.len();
        assert!(n >= 3);
        h.state_mut().tabs[0].text.push_str("unsaved");
        h.state_mut().tabs[1].text.push_str("unsaved");
        h.state_mut().active = Some(n - 1);
        h.run_steps(3);
        let keep = h.state().tabs[n - 1].path.clone();
        h.get_by_role_and_label(Role::Tab, &folder_name(&keep)).click_secondary();
        h.run_steps(3);
        h.get_by_label_contains("Close others").click();
        h.run_steps(30);
        assert!(h.query_by_label_contains("Save the changes to 2 files?").is_some());
        h.get_by_label("Don't save").click();
        h.run_steps(3);
        assert_eq!(h.state().tabs.iter().map(|t| t.path.clone()).collect::<Vec<_>>(), [keep.clone()]);
        assert_eq!(h.state().active, Some(0));

        // Context in a task's header: what an agent loads, missing IDs in red; a file opens on click.
        fs::write(&t1, "---\nkind: task\nid: T-0001\nstatus: open\nsize: S\nmodules: []\nrefs: [T-0404]\n---\n# T-0001 — Add login\n").unwrap();
        h.state_mut().reload();
        let ctx = h.ctx.clone();
        h.state_mut().apply(&ctx, Action::Open(t1.clone()));
        h.run_steps(3);
        h.get_by_label_contains("Context").click();
        h.run_steps(3);
        h.get_by_label_contains("Missing: T-0404 (refs)");
        h.get_by_label_contains("mwp/constitution.md").click();
        h.run_steps(3);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(root.join("mwp/constitution.md")));

        // File > New project creates the project and opens its PROJECT.md.
        h.get_all_by_label("File").next().unwrap().click(); // the menu; the module template has a "File" column
        h.run_steps(3);
        h.get_by_label_contains("New project…").click();
        h.run_steps(3);
        h.run_steps(30); // the modal ignores clicks while it fades in
        let new = base.join("new");
        if let Some(Dialog::New { dir, agents, .. }) = h.state_mut().dialog.as_mut() {
            *dir = new.display().to_string();
            agents.iter_mut().for_each(|a| *a = false); // none checked: Claude
        } else {
            panic!("the New project dialog did not open");
        }
        h.run_steps(3); // the dialog relays out for the new folder before the click
        let before = h.get_by_label("Create").rect();
        h.run_steps(3);
        assert_eq!(h.get_by_label("Create").rect(), before, "the dialog must keep its size");
        h.get_by_label("Create").click();
        h.run_steps(3);
        assert!(h.state().dialog.is_none(), "the dialog did not create the project");
        assert!(new.join("mwp/constitution.md").exists() && new.join("CLAUDE.md").exists());
        assert_eq!(h.state().root, new);
        assert_eq!(h.state().current().map(|e| e.path.clone()), Some(new.join("mwp/PROJECT.md")));
        let _ = fs::remove_dir_all(&base);
    }
}
