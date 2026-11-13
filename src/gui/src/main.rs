//! `mwp-gui`: an editor for MWP projects. Explorer of the mwp/ files, a Markdown editor with
//! tabs and preview, the task board, lint problems and a dialog to create a project.
//! All the MWP logic is in `mwp_core`.

mod editor;

use editor::{Editor, Mode};
use eframe::egui::{self, Color32, RichText};
use egui_material_icons::{icons::*, MaterialIcon};
use mwp_core::doc::{id_num, slug, walk, Doc, Project};
use mwp_core::lint::{lint, Report};
use mwp_core::ops;
use mwp_core::scaffold::{self, AGENTS};
use mwp_core::state::{self, Claim};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// shortcut: the project is reread on a timer, not on file events; add a file watcher
// if rereading every 2 s gets slow on a very large project.
const REFRESH: Duration = Duration::from_secs(2);

const COLUMNS: [(&str, &str); 5] = [("open", "Open"), ("in_progress", "In progress"), ("blocked", "Blocked"), ("review", "Review"), ("done", "Done")];
const PROJECT_FILES: [&str; 4] = ["PROJECT.md", "constitution.md", "map.md", "glossary.md"];
const SECTIONS: [(&str, &str, MaterialIcon); 5] = [
    ("Modules", "modules", ICON_EXTENSION),
    ("Open tasks", "tasks/open", ICON_TASK),
    ("Done tasks", "tasks/done", ICON_TASK_ALT),
    ("Specs", "specs", ICON_ARTICLE),
    ("Decisions", "decisions", ICON_GAVEL),
];
const MODULE_TYPES: [(&str, &str); 3] = [("domain", "Business rules"), ("ui", "Screens and components"), ("infra", "Storage, network, tooling")];
const SIZES: [(&str, &str); 3] = [("S", "Small: a few lines, one module"), ("M", "Medium: a feature in one or two modules"), ("L", "Large: several modules, plan first")];
const fn key(modifiers: egui::Modifiers, key: egui::Key) -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(modifiers, key)
}
const CMD: egui::Modifiers = egui::Modifiers::COMMAND;
const SAVE: egui::KeyboardShortcut = key(CMD, egui::Key::S);
const SAVE_ALL: egui::KeyboardShortcut = key(CMD.plus(egui::Modifiers::SHIFT), egui::Key::S);

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

/// A file in the explorer: its ID (empty for the project files) and its title or file name.
/// `ok` is false for a doc without front matter.
struct Entry {
    path: PathBuf,
    id: String,
    title: String,
    ok: bool,
}

/// One read of the project from disk.
struct View {
    project: Project,
    report: Report,
    claims: Vec<Claim>,
    locks: Vec<(String, Claim)>,
    tree: Vec<(&'static str, MaterialIcon, Vec<Entry>)>,
}

fn load(root: &Path) -> Result<View, String> {
    let project = Project::open(root)?;
    Ok(View { report: lint(&project), claims: state::claims(&project.state), locks: state::locks(&project.state), tree: tree(&project), project })
}

fn entry(p: &Project, path: PathBuf) -> Entry {
    let file = path.file_name().unwrap().to_string_lossy().into_owned();
    let (id, title) = match p.docs.iter().find(|d| d.path == path) {
        Some(d) if !d.id().is_empty() => (d.id().to_string(), if d.title().is_empty() || d.title() == d.id() { String::new() } else { d.title().to_string() }),
        _ => (String::new(), file),
    };
    Entry { ok: !p.invalid.contains(&path), id, title, path }
}

/// The explorer: the project files, then one section per MWP folder. Templates (`_*`) are left out.
fn tree(p: &Project) -> Vec<(&'static str, MaterialIcon, Vec<Entry>)> {
    let top = PROJECT_FILES.iter().map(|f| p.mwp.join(f)).filter(|f| f.exists()).map(|f| entry(p, f)).collect();
    let mut out = vec![("Project", ICON_FOLDER, top)];
    for (label, dir, icon) in SECTIONS {
        let mut files = Vec::new();
        walk(&p.mwp.join(dir), &mut files);
        files.retain(|f| f.extension().is_some_and(|e| e == "md") && !f.file_name().unwrap().to_string_lossy().starts_with('_'));
        files.sort();
        out.push((label, icon, files.into_iter().map(|f| entry(p, f)).collect()));
    }
    out
}

/// The icon and colour of a task status.
fn status_icon(ui: &egui::Ui, status: &str) -> (MaterialIcon, Color32) {
    let v = ui.visuals();
    match status {
        "in_progress" => (ICON_PENDING, v.hyperlink_color),
        "blocked" => (ICON_BLOCK, v.error_fg_color),
        "review" => (ICON_RATE_REVIEW, v.warn_fg_color),
        "done" => (ICON_CHECK_CIRCLE, Color32::from_rgb(90, 170, 100)),
        _ => (ICON_RADIO_BUTTON_UNCHECKED, v.weak_text_color()),
    }
}

/// The icon of a file: by kind, by status for a task, by name for the project files.
fn file_icon(ui: &egui::Ui, p: &Project, path: &Path) -> (MaterialIcon, Color32) {
    if p.invalid.iter().any(|f| f == path) {
        return (ICON_ERROR, ui.visuals().error_fg_color);
    }
    let doc = p.docs.iter().find(|d| d.path == path);
    match doc.map(|d| (d.kind(), d)) {
        Some(("task", d)) => status_icon(ui, d.get("status")),
        Some(("module", _)) => (ICON_EXTENSION, Color32::from_rgb(100, 150, 230)),
        Some(("spec", _)) => (ICON_ARTICLE, Color32::from_rgb(170, 130, 220)),
        Some(("adr", _)) => (ICON_GAVEL, Color32::from_rgb(205, 150, 80)),
        _ => {
            let icon = match path.file_name().and_then(|f| f.to_str()) {
                Some("constitution.md") => ICON_POLICY,
                Some("map.md") => ICON_MAP,
                Some("glossary.md") => ICON_MENU_BOOK,
                _ => ICON_DESCRIPTION,
            };
            (icon, ui.visuals().weak_text_color())
        }
    }
}

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
    /// Switch to the project in this folder; `true` also opens its PROJECT.md.
    Project(PathBuf, bool),
    Quit,
}

enum Dialog {
    New { dir: String, name: String, agents: Vec<bool>, skills: bool, error: String },
    Open { dir: String, error: String },
    NewTask { title: String, size: &'static str, modules: Vec<(String, bool)>, error: String },
    NewModule { id: String, kind: &'static str, summary: String, error: String },
    /// `label` names the file; `note` warns about an agent working on it.
    Delete { path: PathBuf, label: String, note: String, error: String },
}

struct App {
    root: PathBuf,
    view: Result<View, String>,
    loaded: Instant,
    tabs: Vec<Editor>,
    /// The tab shown; `None` shows the board.
    active: Option<usize>,
    mode: Mode,
    /// An Edit menu command and how many frames until it runs (focus first, then the event).
    edit_cmd: Option<(EditCmd, u8)>,
    pending: Option<Action>,
    dialog: Option<Dialog>,
    message: Option<String>,
    show_done: bool,
    show_problems: bool,
    title: String,
}

impl App {
    fn new(root: PathBuf) -> App {
        App {
            view: load(&root),
            root,
            loaded: Instant::now(),
            tabs: Vec::new(),
            active: None,
            mode: Mode::Split,
            edit_cmd: None,
            pending: None,
            dialog: None,
            message: None,
            show_done: false,
            show_problems: true,
            title: String::new(),
        }
    }

    /// The app with its icon font and a little more room than egui's default spacing.
    fn start(ctx: &egui::Context, root: PathBuf) -> App {
        egui_material_icons::initialize(ctx);
        ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 5.0);
            s.spacing.button_padding = egui::vec2(8.0, 3.0);
        });
        App::new(root)
    }

    fn reload(&mut self) {
        self.view = load(&self.root);
        self.loaded = Instant::now();
        self.tabs.iter_mut().for_each(Editor::refresh);
    }

    fn current(&self) -> Option<&Editor> {
        self.active.and_then(|i| self.tabs.get(i))
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
            Action::Project(..) | Action::Quit => self.tabs.iter().any(Editor::dirty),
            Action::Open(_) | Action::Board => false,
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
            Action::CloseTab(i) if i < self.tabs.len() => {
                self.tabs.remove(i);
                self.active = match self.active {
                    Some(a) if a > i => Some(a - 1),
                    Some(a) if a == i => (!self.tabs.is_empty()).then(|| i.min(self.tabs.len() - 1)),
                    other => other,
                };
            }
            Action::CloseTab(_) => {}
            Action::Project(dir, open_project) => {
                self.root = dir;
                (self.tabs, self.active) = (Vec::new(), None);
                self.reload();
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

    fn new_project_dialog(&self) -> Dialog {
        Dialog::New { dir: self.root.display().to_string(), name: folder_name(&self.root), agents: AGENTS.iter().map(|a| a.id == "claude").collect(), skills: true, error: String::new() }
    }

    /// New task, with the module open in the editor already checked.
    fn new_task_dialog(&self) -> Option<Dialog> {
        let v = self.view.as_ref().ok()?;
        let open = self.current().map(|e| e.path.as_path());
        let mut modules: Vec<(String, bool)> = v.project.of_kind("module").map(|m| (m.id().to_string(), open == Some(m.path.as_path()))).collect();
        modules.sort();
        Some(Dialog::NewTask { title: String::new(), size: "M", modules, error: String::new() })
    }

    /// Asks before deleting; says when an agent works on the task or open tasks use the module.
    fn delete_dialog(&self, path: PathBuf) -> Option<Dialog> {
        let v = self.view.as_ref().ok()?;
        let doc = v.project.docs.iter().find(|d| d.path == path);
        let label = match doc {
            Some(d) if !d.id().is_empty() => format!("{}  {}", d.id(), d.title()).trim_end().to_string(),
            _ => v.project.rel(&path),
        };
        let note = match doc.map(|d| (d.kind(), d.id())) {
            Some(("task", id)) => v.claims.iter().find(|c| c.task == id).map(|c| format!("{} is working on it: its claim and module locks are freed.", c.agent)).unwrap_or_default(),
            Some(("module", id)) => {
                let users: Vec<&str> = v.project.of_kind("task").filter(|t| t.get("status") != "done" && t.list("modules").iter().any(|m| m == id)).map(Doc::id).collect();
                if users.is_empty() { String::new() } else { format!("Tasks not done still list it: {}.", users.join(", ")) }
            }
            _ => String::new(),
        };
        Some(Dialog::Delete { path, label, note, error: String::new() })
    }

    /// The dialogs; returns what to open or switch to.
    fn dialogs(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let mut close = false;
        let mut deleted: Option<PathBuf> = None;
        let root = self.root.clone();
        match self.dialog.as_mut()? {
            Dialog::NewTask { title, size, modules, error } => {
                egui::Modal::new(egui::Id::unique("new-task")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    dialog_title(ui, ICON_ADD_TASK, "New task");
                    ui.weak("Created in mwp/tasks/open/ with the next free ID, from the task template.");
                    ui.add_space(12.0);
                    section(ui, "Task", |ui| {
                        ui.label("Title");
                        ui.add(egui::TextEdit::singleline(title).hint_text("What changes, in a few words").desired_width(f32::INFINITY));
                        ui.add_space(6.0);
                        ui.label("Size");
                        ui.horizontal(|ui| {
                            for (s, _) in SIZES {
                                ui.selectable_value(size, s, format!("  {s}  "));
                            }
                        });
                        ui.weak(SIZES.iter().find(|s| s.0 == *size).map_or("", |s| s.1));
                    });
                    section(ui, "Modules it may write", |ui| {
                        if modules.is_empty() {
                            ui.weak("No module yet: create one first with New module.");
                        }
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for (id, on) in modules.iter_mut() {
                                ui.checkbox(on, id.as_str());
                            }
                        });
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!title.trim().is_empty(), primary(ui, "Create")).clicked() {
                            let chosen: Vec<&str> = modules.iter().filter(|m| m.1).map(|m| m.0.as_str()).collect();
                            match Project::open(&root).and_then(|p| ops::task_new(&p, title, size, &[("modules", Some(&chosen.join(",")))])) {
                                // The first line is `create mwp/tasks/open/T-0007-x.md (size M, status: open)`.
                                Ok(lines) => action = lines.iter().find_map(|l| l.strip_prefix("create ")?.split(" (").next()).map(|rel| Action::Open(root.join(rel))),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::NewModule { id, kind, summary, error } => {
                egui::Modal::new(egui::Id::unique("new-module")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    dialog_title(ui, ICON_EXTENSION, "New module");
                    ui.weak("Created in mwp/modules/ from the module template and added to map.md.");
                    ui.add_space(12.0);
                    let file = slug(id);
                    section(ui, "Module", |ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(id).hint_text("order, user-auth").desired_width(f32::INFINITY));
                        ui.weak(if file.is_empty() { "Lowercase words joined by -.".to_string() } else { format!("ID {file}  ·  mwp/modules/{file}.md") });
                        ui.add_space(6.0);
                        ui.label("Type");
                        ui.horizontal(|ui| {
                            for (t, _) in MODULE_TYPES {
                                ui.selectable_value(kind, t, format!("  {t}  "));
                            }
                        });
                        ui.weak(MODULE_TYPES.iter().find(|t| t.0 == *kind).map_or("", |t| t.1));
                        ui.add_space(6.0);
                        ui.label("Summary");
                        ui.add(egui::TextEdit::singleline(summary).hint_text("Its responsibility, in one line").desired_width(f32::INFINITY));
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!file.is_empty() && !summary.trim().is_empty(), primary(ui, "Create")).clicked() {
                            match Project::open(&root).and_then(|p| ops::module_new(&p, &file, kind, summary).map(|_| p.mwp.join("modules").join(format!("{file}.md")))) {
                                Ok(path) => action = Some(Action::Open(path)),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::Delete { path, label, note, error } => {
                egui::Modal::new(egui::Id::unique("delete")).show(ctx, |ui| {
                    ui.set_width(420.0);
                    dialog_title(ui, ICON_DELETE, "Delete file");
                    ui.add_space(6.0);
                    ui.label(format!("Delete {label}?"));
                    if !note.is_empty() {
                        ui.colored_label(ui.visuals().warn_fg_color, note.as_str());
                    }
                    ui.weak("The file is removed from disk; the editor cannot bring it back.");
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        let danger = egui::Button::new(RichText::new("Delete").strong().color(Color32::WHITE)).fill(ui.visuals().error_fg_color).min_size(egui::vec2(96.0, 0.0));
                        if ui.add(danger).clicked() {
                            match Project::open(&root).and_then(|p| ops::delete(&p, path)) {
                                Ok(_) => deleted = Some(path.clone()),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::New { dir, name, agents, skills, error } => {
                egui::Modal::new(egui::Id::unique("new-project")).show(ctx, |ui| {
                    ui.set_width(540.0);
                    dialog_title(ui, ICON_CREATE_NEW_FOLDER, "New MWP project");
                    ui.weak("Creates mwp/ and the files of each agent you choose. Existing files are never overwritten.");
                    ui.add_space(12.0);

                    section(ui, "Location", |ui| {
                        ui.label("Folder");
                        right_row(ui, |ui| {
                            if ui.button("Browse…").clicked() {
                                let start = Path::new(dir.trim());
                                let dialog = rfd::FileDialog::new().set_title("Project folder");
                                let dialog = if start.is_dir() { dialog.set_directory(start) } else { dialog };
                                if let Some(picked) = dialog.pick_folder() {
                                    // Follow the folder name unless the user typed a name of their own.
                                    if name.trim().is_empty() || *name == folder_name(Path::new(dir.trim())) {
                                        *name = folder_name(&picked);
                                    }
                                    *dir = picked.display().to_string();
                                }
                            }
                            ui.add(egui::TextEdit::singleline(dir).hint_text("Folder where mwp/ will be created").desired_width(f32::INFINITY));
                        });
                        let folder = Path::new(dir.trim());
                        match () {
                            _ if dir.trim().is_empty() => ui.weak("Choose where the project lives."),
                            _ if folder.join("mwp").is_dir() => ui.colored_label(ui.visuals().warn_fg_color, "This folder already has an MWP project: only missing files will be added."),
                            _ if folder.is_dir() => ui.weak("mwp/ will be created in this folder."),
                            _ => ui.weak("The folder will be created."),
                        };
                        ui.add_space(6.0);
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).hint_text("Project name").desired_width(f32::INFINITY));
                        ui.weak("Written to mwp/PROJECT.md.");
                    });

                    section(ui, "Agents", |ui| {
                        ui.weak("Each agent gets its instruction file and its skills folder.");
                        ui.add_space(4.0);
                        egui::Grid::new("agents-grid").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                            for (on, a) in agents.iter_mut().zip(AGENTS.iter()) {
                                ui.checkbox(on, a.name);
                                ui.weak(format!("{}  ·  {}", a.pointer, a.skills));
                                ui.end_row();
                            }
                        });
                        if !agents.iter().any(|on| *on) {
                            ui.weak("None checked: Claude Code is used.");
                        }
                    });

                    section(ui, "Options", |ui| {
                        ui.checkbox(skills, "Install the agent skills (mwp, module)");
                    });

                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!dir.trim().is_empty(), primary(ui, "Create")).clicked() {
                            let folder = PathBuf::from(dir.trim());
                            let mut chosen: Vec<_> = AGENTS.iter().zip(agents.iter()).filter(|(_, on)| **on).map(|(a, _)| a).collect();
                            if chosen.is_empty() {
                                chosen.push(&AGENTS[0]);
                            }
                            match scaffold::init(&folder, Some(name.trim()).filter(|n| !n.is_empty()), &chosen, *skills) {
                                Ok(_) => action = Some(Action::Project(folder, true)),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::Open { dir, error } => {
                egui::Modal::new(egui::Id::unique("open-project")).show(ctx, |ui| {
                    ui.set_width(460.0);
                    dialog_title(ui, ICON_FOLDER_OPEN, "Open MWP project");
                    ui.label("The folder that contains mwp/.");
                    ui.add(egui::TextEdit::singleline(dir).desired_width(f32::INFINITY));
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add(primary(ui, "Open")).clicked() {
                            let folder = PathBuf::from(dir.trim());
                            match Project::open(&folder) {
                                Ok(_) => action = Some(Action::Project(folder, false)),
                                Err(e) => *error = format!("{e}. Use File > New project to create one."),
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
        }
        close |= ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if close || action.is_some() || deleted.is_some() {
            self.dialog = None;
        }
        if let Some(path) = deleted {
            if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
                self.apply(ctx, Action::CloseTab(i)); // the file is gone; its edits go with it
            }
            self.reload();
        }
        if matches!(action, Some(Action::Open(_))) {
            self.reload();
        }
        action
    }
}

fn dialog_title(ui: &mut egui::Ui, icon: MaterialIcon, title: &str) {
    ui.horizontal(|ui| {
        ui.label(icon.rich_text().size(24.0).color(ui.visuals().hyperlink_color));
        ui.heading(title);
    });
}

fn error_line(ui: &mut egui::Ui, error: &str) {
    if !error.is_empty() {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

/// The main button of a dialog.
fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).strong()).fill(ui.visuals().selection.bg_fill).min_size(egui::vec2(96.0, 0.0))
}

fn secondary(text: &str) -> egui::Button<'static> {
    egui::Button::new(text.to_string()).min_size(egui::vec2(96.0, 0.0))
}

fn folder_name(path: &Path) -> String {
    path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/// One row laid out from the right. Its height is fixed: a right-to-left layout would
/// otherwise take all the height it is offered, and the modal would grow every frame.
fn right_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::right_to_left(egui::Align::Center), add);
}

/// A titled, framed group of fields in a dialog.
fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(title).strong());
    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
    ui.add_space(10.0);
}

/// Secondary text on one line, cut with an ellipsis when the column is narrow.
fn dim(ui: &mut egui::Ui, text: String) {
    ui.add(egui::Label::new(egui::RichText::new(text).weak()).truncate());
}

/// A task card: ID, size, title, then its modules, agent and work codes. The whole card is clickable.
fn card(ui: &mut egui::Ui, t: &Doc) -> egui::Response {
    let rect = egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon, color) = status_icon(ui, t.get("status"));
                ui.label(icon.rich_text().color(color));
                ui.strong(t.id());
                if !t.get("size").is_empty() {
                    ui.label(RichText::new(format!(" {} ", t.get("size"))).small().background_color(ui.visuals().widgets.inactive.bg_fill));
                }
            });
            ui.label(t.title());
            let meta = |ui: &mut egui::Ui, icon: MaterialIcon, text: String| {
                ui.horizontal(|ui| {
                    ui.label(icon.rich_text().small().weak());
                    dim(ui, text);
                });
            };
            let modules = t.list("modules");
            if !modules.is_empty() {
                meta(ui, ICON_EXTENSION, modules.join(", "));
            }
            if !t.get("agent").is_empty() && t.get("status") == "in_progress" {
                meta(ui, ICON_SMART_TOY, t.get("agent").to_string());
            }
            let external = t.list("external");
            if !external.is_empty() {
                meta(ui, ICON_LINK, external.join(", "));
            }
        })
        .response
        .rect;
    ui.add_space(4.0);
    let r = ui.interact(rect, ui.scope_id().with(("card", t.id())), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if r.hovered() {
        ui.painter().rect_stroke(rect, 6.0, ui.visuals().widgets.hovered.bg_stroke, egui::StrokeKind::Inside);
    }
    r
}

/// A collapsible explorer section: icon, title and count, with an optional "+" on the right.
/// Clicking the title toggles it too. Returns true when "+" is clicked.
fn group(ui: &mut egui::Ui, icon: MaterialIcon, title: &str, count: usize, open: bool, add: Option<&str>, body: impl FnOnce(&mut egui::Ui)) -> bool {
    let id = ui.make_persistent_id(("section", title));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open);
    let mut plus = false;
    let header = ui.horizontal(|ui| {
        state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
        let label = (icon.rich_text().color(ui.visuals().weak_text_color()), RichText::new(title).strong(), RichText::new(count.to_string()).weak());
        if ui.add(egui::Button::new(label).frame(false)).clicked() {
            state.toggle(ui);
        }
        if let Some(tip) = add {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0); // clear of the scroll bar
                plus = ui.add(egui::Button::new(ICON_ADD.rich_text()).frame(false)).on_hover_text(tip).clicked();
            });
        }
    });
    state.show_body_indented(&header.response, ui, body);
    plus
}

impl eframe::App for App {
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
        self.run_edit_cmd(&ctx);

        let title = match self.current() {
            Some(ed) => format!("{}{} — MWP", if ed.dirty() { "● " } else { "" }, ed.name()),
            None => format!("{} — MWP", self.root.display()),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        let mut action: Option<Action> = None;
        let (mut save, mut save_all) = (false, false);
        let mut reload = false;
        let (mut new_dialog, mut open_dialog) = (false, false);
        let (mut new_task, mut new_module) = (false, false);
        let mut ask_delete: Option<PathBuf> = None;
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
                    new_task = ui.add_enabled(project, egui::Button::new(ic(ICON_ADD_TASK, "New task…"))).clicked();
                    new_module = ui.add_enabled(project, egui::Button::new(ic(ICON_EXTENSION, "New module…"))).clicked();
                    ui.separator();
                    let dirty = self.current().is_some_and(Editor::dirty);
                    save = ui.add_enabled(dirty, egui::Button::new(ic(ICON_SAVE, "Save")).shortcut_text(ctx.format_shortcut(&SAVE))).clicked();
                    let any = self.tabs.iter().any(Editor::dirty);
                    save_all = ui.add_enabled(any, egui::Button::new(ic(ICON_SAVE_AS, "Save all")).shortcut_text(ctx.format_shortcut(&SAVE_ALL))).clicked();
                    if ui.add_enabled(self.active.is_some(), egui::Button::new(ic(ICON_CLOSE, "Close file"))).clicked() {
                        action = self.active.map(Action::CloseTab);
                    }
                    if ui.add_enabled(deletable.is_some(), egui::Button::new(ic(ICON_DELETE, "Delete file…"))).on_disabled_hover_text("Only modules, specs and open tasks can be deleted").clicked() {
                        ask_delete = deletable.clone();
                    }
                    ui.separator();
                    reload = ui.button(ic(ICON_REFRESH, "Reload")).clicked();
                    ui.separator();
                    if ui.button(ic(ICON_LOGOUT, "Quit")).clicked() {
                        action = Some(Action::Quit);
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
                });
                ui.menu_button("View", |ui| {
                    if ui.button(ic(ICON_VIEW_KANBAN, "Board")).clicked() {
                        action = Some(Action::Board);
                    }
                    ui.separator();
                    for (m, label) in [(Mode::Edit, "Source only"), (Mode::Split, "Source and preview"), (Mode::Preview, "Preview only")] {
                        ui.radio_value(&mut self.mode, m, label);
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_done, "Done tasks on the board");
                    ui.checkbox(&mut self.show_problems, "Problems panel");
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
                    ui.toggle_value(&mut self.show_problems, problems).on_hover_text("Errors and warnings from mwp lint");
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
                    });
                });
            }
            Ok(v) => {
                egui::Panel::left("explorer").resizable(true).default_size(280.0).show(ui, |ui| {
                    let name = v.project.of_kind("project").next().map_or("", |d| d.get("name"));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(ICON_HUB.rich_text().size(20.0).color(ui.visuals().hyperlink_color));
                        ui.heading(if name.is_empty() { "MWP" } else { name });
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("explorer").auto_shrink([false, false]).show(ui, |ui| {
                        if ui.selectable_label(self.active.is_none(), ic(ICON_VIEW_KANBAN, "Board")).clicked() {
                            action = Some(Action::Board);
                        }
                        ui.add_space(4.0);
                        group(ui, ICON_SMART_TOY, "Agents at work", v.claims.len(), true, None, |ui| {
                            for c in &v.claims {
                                let modules: Vec<&str> = v.locks.iter().filter(|l| l.1.task == c.task).map(|l| l.0.as_str()).collect();
                                let (icon, color) = status_icon(ui, "in_progress");
                                if ui.selectable_label(false, (icon.rich_text().color(color), RichText::new(&c.task).strong(), RichText::new(&c.agent))).clicked() {
                                    action = v.project.by_id("task", &c.task).map(|t| Action::Open(t.path.clone()));
                                }
                                ui.horizontal(|ui| {
                                    ui.label(ICON_LOCK.rich_text().small().weak());
                                    dim(ui, if modules.is_empty() { "no module locked".into() } else { modules.join(", ") });
                                });
                            }
                            if v.claims.is_empty() {
                                ui.weak("No task in progress.");
                            }
                        });
                        for (label, icon, entries) in &v.tree {
                            let add = match *label {
                                "Modules" => Some("New module"),
                                "Open tasks" => Some("New task"),
                                _ => None,
                            };
                            let plus = group(ui, *icon, label, entries.len(), *label != "Done tasks", add, |ui| {
                                for e in entries {
                                    let open = self.current().is_some_and(|ed| ed.path == e.path);
                                    let (icon, color) = file_icon(ui, &v.project, &e.path);
                                    let title = if e.ok { RichText::new(&e.title) } else { RichText::new(&e.title).color(ui.visuals().error_fg_color) };
                                    let r = if e.id.is_empty() {
                                        ui.selectable_label(open, (icon.rich_text().color(color), title))
                                    } else {
                                        ui.selectable_label(open, (icon.rich_text().color(color), RichText::new(&e.id).strong(), title))
                                    };
                                    let r = r.on_hover_text(v.project.rel(&e.path));
                                    if r.clicked() {
                                        action = Some(Action::Open(e.path.clone()));
                                    }
                                    r.context_menu(|ui| {
                                        if ui.button(ic(ICON_EDIT, "Open")).clicked() {
                                            action = Some(Action::Open(e.path.clone()));
                                            ui.close();
                                        }
                                        if ops::deletable(&v.project, &e.path) && ui.button(ic(ICON_DELETE, "Delete…")).clicked() {
                                            ask_delete = Some(e.path.clone());
                                            ui.close();
                                        }
                                    });
                                }
                                if entries.is_empty() {
                                    ui.weak("Empty");
                                }
                            });
                            if plus {
                                (new_module, new_task) = (*label == "Modules", *label == "Open tasks");
                            }
                        }
                    });
                });

                if self.show_problems {
                    egui::Panel::bottom("problems").resizable(true).default_size(110.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.strong("Problems");
                            ui.weak(format!("{} errors, {} warnings", v.report.errors.len(), v.report.warnings.len()));
                        });
                        egui::ScrollArea::vertical().id_salt("problems").auto_shrink([false, false]).show(ui, |ui| {
                            let errors = v.report.errors.iter().map(|l| (l, ui.visuals().error_fg_color, ICON_ERROR));
                            let warnings = v.report.warnings.iter().map(|l| (l, ui.visuals().warn_fg_color, ICON_WARNING));
                            for (line, color, icon) in errors.chain(warnings).collect::<Vec<_>>() {
                                let r = ui.selectable_label(false, (icon.rich_text().color(color), RichText::new(line).monospace()));
                                if r.clicked() {
                                    action = problem_path(&self.root, line).map(Action::Open);
                                }
                            }
                            if v.report.errors.is_empty() && v.report.warnings.is_empty() {
                                ui.weak("No errors or warnings.");
                            }
                        });
                    });
                }

                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::horizontal().id_salt("tabs").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(self.active.is_none(), ic(ICON_VIEW_KANBAN, "Board")).clicked() {
                                action = Some(Action::Board);
                            }
                            for (i, t) in self.tabs.iter().enumerate() {
                                ui.separator();
                                let (icon, color) = file_icon(ui, &v.project, &t.path);
                                let name = RichText::new(if t.dirty() { format!("{} •", t.name()) } else { t.name() });
                                if ui.selectable_label(self.active == Some(i), (icon.rich_text().color(color), name)).on_hover_text(v.project.rel(&t.path)).clicked() {
                                    action = Some(Action::Open(t.path.clone()));
                                }
                                if ui.add(egui::Button::new(ICON_CLOSE.rich_text()).frame(false)).on_hover_text("Close").clicked() {
                                    action = Some(Action::CloseTab(i));
                                }
                            }
                        });
                    });
                    ui.separator();
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
                            if deletable.is_some() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(ic(ICON_DELETE, "Delete")).on_hover_text("Delete this file").clicked() {
                                        ask_delete = deletable.clone();
                                    }
                                });
                            }
                        });
                        ui.separator();
                        editor::show(ui, ed, &mut self.mode);
                    }
                    None => {
                        ui.horizontal(|ui| {
                            ui.label(ICON_VIEW_KANBAN.rich_text().size(24.0).color(ui.visuals().hyperlink_color));
                            ui.heading("Board");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                new_task |= ui.add(egui::Button::new(ic(ICON_ADD_TASK, "New task")).fill(ui.visuals().selection.bg_fill)).clicked();
                                ui.checkbox(&mut self.show_done, "Show done");
                            });
                        });
                        ui.add_space(6.0);
                        let columns: Vec<_> = COLUMNS.iter().filter(|c| c.0 != "done" || self.show_done).collect();
                        ui.columns(columns.len(), |cols| {
                            for (ui, (status, label)) in cols.iter_mut().zip(columns) {
                                let mut tasks: Vec<&Doc> = v.project.of_kind("task").filter(|t| t.get("status") == *status).collect();
                                tasks.sort_by_key(|t| id_num(t.id()));
                                ui.horizontal(|ui| {
                                    let (icon, color) = status_icon(ui, status);
                                    ui.label(icon.rich_text().color(color));
                                    ui.strong(*label);
                                    ui.weak(tasks.len().to_string());
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().id_salt(*status).show(ui, |ui| {
                                    for t in tasks {
                                        let r = card(ui, t);
                                        if r.clicked() {
                                            action = Some(Action::Open(t.path.clone()));
                                        }
                                        r.context_menu(|ui| {
                                            if ui.button(ic(ICON_EDIT, "Open")).clicked() {
                                                action = Some(Action::Open(t.path.clone()));
                                                ui.close();
                                            }
                                            if ops::deletable(&v.project, &t.path) && ui.button(ic(ICON_DELETE, "Delete…")).clicked() {
                                                ask_delete = Some(t.path.clone());
                                                ui.close();
                                            }
                                        });
                                    }
                                });
                            }
                        });
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
        if new_task {
            self.dialog = self.new_task_dialog();
        }
        if new_module && self.view.is_ok() {
            self.dialog = Some(Dialog::NewModule { id: String::new(), kind: "domain", summary: String::new(), error: String::new() });
        }
        if let Some(path) = ask_delete {
            self.dialog = self.delete_dialog(path);
        }
        if let Some(a) = self.dialogs(&ctx) {
            action = Some(a);
        }

        if let Some(pending) = &self.pending {
            let targets: Vec<usize> = match pending {
                Action::CloseTab(i) => vec![*i],
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
        if let Some(a) = action {
            self.request(&ctx, a);
        }
    }
}

fn main() -> eframe::Result {
    // The project is the folder given, or the nearest one with mwp/ from here upward.
    let root = match std::env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => Project::find().map(|p| p.root).unwrap_or_else(|_| std::env::current_dir().unwrap_or_default()),
    };
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 760.0]), ..Default::default() };
    eframe::run_native("MWP", options, Box::new(|cc| Ok(Box::new(App::start(&cc.egui_ctx, root)))))
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

    /// Drives the real UI headless: tabs, edit, save, save all, Edit menu, the unsaved guard, New project.
    #[test]
    fn edit_save_guard_and_new_project() {
        use egui::accesskit::Role;
        use egui_kittest::{kittest::Queryable, Harness};

        let base = std::env::temp_dir().join(format!("mwp-gui-ui-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("app");
        scaffold::init(&root, Some("Demo"), &[&AGENTS[0]], false).unwrap();
        let task = root.join("mwp/tasks/open/T-0001-a.md");
        fs::write(&task, "---\nkind: task\nid: T-0001\nstatus: open\nsize: S\nmodules: []\n---\n# T-0001 — Add login\n").unwrap();

        let start = root.clone();
        let mut h = Harness::builder().with_size([1280.0, 760.0]).build_eframe(move |cc| App::start(&cc.egui_ctx, start));
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
