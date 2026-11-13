//! `mwp-gui`: an editor for MWP projects. Explorer of the mwp/ files, a text editor, the task
//! board, lint problems and a dialog to create a project. All the MWP logic is in `mwp_core`.

use eframe::egui;
use mwp_core::doc::{id_num, walk, Doc, Project};
use mwp_core::lint::{lint, Report};
use mwp_core::scaffold::{self, AGENTS};
use mwp_core::state::{self, Claim};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// shortcut: the project is reread on a timer, not on file events; add a file watcher
// if rereading every 2 s gets slow on a very large project.
const REFRESH: Duration = Duration::from_secs(2);

const COLUMNS: [(&str, &str); 5] = [("open", "Open"), ("in_progress", "In progress"), ("blocked", "Blocked"), ("review", "Review"), ("done", "Done")];
const PROJECT_FILES: [&str; 4] = ["PROJECT.md", "constitution.md", "map.md", "glossary.md"];
const SECTIONS: [(&str, &str); 5] = [("Modules", "modules"), ("Open tasks", "tasks/open"), ("Done tasks", "tasks/done"), ("Specs", "specs"), ("Decisions", "decisions")];
const SAVE: egui::KeyboardShortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

/// A file in the explorer. `ok` is false for a doc without front matter.
struct Entry {
    path: PathBuf,
    label: String,
    ok: bool,
}

/// One read of the project from disk.
struct View {
    project: Project,
    report: Report,
    claims: Vec<Claim>,
    locks: Vec<(String, Claim)>,
    tree: Vec<(&'static str, Vec<Entry>)>,
}

fn load(root: &Path) -> Result<View, String> {
    let project = Project::open(root)?;
    Ok(View { report: lint(&project), claims: state::claims(&project.state), locks: state::locks(&project.state), tree: tree(&project), project })
}

fn entry(p: &Project, path: PathBuf) -> Entry {
    let label = match p.docs.iter().find(|d| d.path == path) {
        Some(d) if !d.id().is_empty() => match d.title() {
            t if t.is_empty() || t == d.id() => d.id().to_string(),
            t => format!("{}  {t}", d.id()),
        },
        _ => path.file_name().unwrap().to_string_lossy().into_owned(),
    };
    Entry { ok: !p.invalid.contains(&path), label, path }
}

/// The explorer: the project files, then one section per MWP folder. Templates (`_*`) are left out.
fn tree(p: &Project) -> Vec<(&'static str, Vec<Entry>)> {
    let top = PROJECT_FILES.iter().map(|f| p.mwp.join(f)).filter(|f| f.exists()).map(|f| entry(p, f)).collect();
    let mut out = vec![("Project", top)];
    for (label, dir) in SECTIONS {
        let mut files = Vec::new();
        walk(&p.mwp.join(dir), &mut files);
        files.retain(|f| f.extension().is_some_and(|e| e == "md") && !f.file_name().unwrap().to_string_lossy().starts_with('_'));
        files.sort();
        out.push((label, files.into_iter().map(|f| entry(p, f)).collect()));
    }
    out
}

/// The file a lint line is about: lines are `mwp/path.md: message`.
fn problem_path(root: &Path, line: &str) -> Option<PathBuf> {
    line.split_once(": ").map(|(path, _)| root.join(path)).filter(|p| p.is_file())
}

/// The open file. Kept with `\n` endings while editing, written back with the file's own.
struct Editor {
    path: PathBuf,
    text: String,
    saved: String,
    crlf: bool,
}

impl Editor {
    fn open(path: &Path) -> Result<Editor, String> {
        let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = raw.replace("\r\n", "\n");
        Ok(Editor { path: path.to_path_buf(), saved: text.clone(), text, crlf: raw.contains("\r\n") })
    }
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
}

/// Something that would replace the open file; asked first when it has unsaved changes.
enum Action {
    Open(PathBuf),
    Board,
    /// Switch to the project in this folder; `true` also opens its PROJECT.md.
    Project(PathBuf, bool),
    Quit,
}

enum Dialog {
    New { dir: String, name: String, agents: Vec<bool>, skills: bool, error: String },
    Open { dir: String, error: String },
}

struct App {
    root: PathBuf,
    view: Result<View, String>,
    loaded: Instant,
    editor: Option<Editor>,
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
            editor: None,
            pending: None,
            dialog: None,
            message: None,
            show_done: false,
            show_problems: true,
            title: String::new(),
        }
    }

    fn reload(&mut self) {
        self.view = load(&self.root);
        self.loaded = Instant::now();
        // An agent may have changed the open file: follow it unless there are edits to keep.
        if let Some(ed) = self.editor.as_mut().filter(|e| !e.dirty()) {
            if let Ok(fresh) = Editor::open(&ed.path) {
                *ed = fresh;
            }
        }
    }

    fn save(&mut self) -> bool {
        let Some(ed) = self.editor.as_mut() else { return true };
        let text = if ed.crlf { ed.text.replace('\n', "\r\n") } else { ed.text.clone() };
        match fs::write(&ed.path, text) {
            Ok(()) => {
                ed.saved = ed.text.clone();
                self.message = None;
                self.reload();
                true
            }
            Err(e) => {
                self.message = Some(format!("cannot save {}: {e}", ed.path.display()));
                false
            }
        }
    }

    /// Runs the action, or asks first when it would drop unsaved edits.
    fn request(&mut self, ctx: &egui::Context, a: Action) {
        let same = matches!((&a, &self.editor), (Action::Open(p), Some(ed)) if *p == ed.path);
        if self.editor.as_ref().is_some_and(Editor::dirty) && !same {
            self.pending = Some(a);
        } else {
            self.apply(ctx, a);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::Open(path) => match Editor::open(&path) {
                Ok(ed) => self.editor = Some(ed),
                Err(e) => self.message = Some(e),
            },
            Action::Board => self.editor = None,
            Action::Project(dir, open_project) => {
                self.root = dir;
                self.editor = None;
                self.reload();
                if open_project {
                    self.apply(ctx, Action::Open(self.root.join("mwp/PROJECT.md")));
                }
            }
            Action::Quit => {
                self.editor = None; // nothing left to guard
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn new_project_dialog(&self) -> Dialog {
        Dialog::New { dir: self.root.display().to_string(), name: folder_name(&self.root), agents: AGENTS.iter().map(|a| a.id == "claude").collect(), skills: true, error: String::new() }
    }

    /// The New / Open project dialogs; returns the project to switch to.
    fn dialogs(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let mut close = false;
        match self.dialog.as_mut()? {
            Dialog::New { dir, name, agents, skills, error } => {
                egui::Modal::new(egui::Id::unique("new-project")).show(ctx, |ui| {
                    ui.set_width(540.0);
                    ui.heading("New MWP project");
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

                    if !error.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                    }
                    ui.separator();
                    right_row(ui, |ui| {
                        let create = egui::Button::new(egui::RichText::new("Create").strong()).fill(ui.visuals().selection.bg_fill).min_size(egui::vec2(96.0, 0.0));
                        if ui.add_enabled(!dir.trim().is_empty(), create).clicked() {
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
                        close = ui.add(egui::Button::new("Cancel").min_size(egui::vec2(96.0, 0.0))).clicked();
                    });
                });
                close |= ctx.input(|i| i.key_pressed(egui::Key::Escape));
            }
            Dialog::Open { dir, error } => {
                egui::Modal::new(egui::Id::unique("open-project")).show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.heading("Open MWP project");
                    ui.label("The folder that contains mwp/.");
                    ui.add(egui::TextEdit::singleline(dir).desired_width(f32::INFINITY));
                    if !error.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Open").clicked() {
                            let folder = PathBuf::from(dir.trim());
                            match Project::open(&folder) {
                                Ok(_) => action = Some(Action::Project(folder, false)),
                                Err(e) => *error = format!("{e}. Use File > New project to create one."),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                });
            }
        }
        if close || action.is_some() {
            self.dialog = None;
        }
        action
    }
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

/// A task card; returns true when clicked.
fn card(ui: &mut egui::Ui, t: &Doc) -> bool {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let clicked = ui.selectable_label(false, egui::RichText::new(format!("{}  ·  {}", t.id(), t.get("size"))).strong()).clicked();
        ui.label(t.title());
        let modules = t.list("modules");
        if !modules.is_empty() {
            dim(ui, format!("modules: {}", modules.join(", ")));
        }
        if !t.get("agent").is_empty() && t.get("status") == "in_progress" {
            dim(ui, format!("agent: {}", t.get("agent")));
        }
        let external = t.list("external");
        if !external.is_empty() {
            dim(ui, external.join(", "));
        }
        clicked
    })
    .inner
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.loaded.elapsed() >= REFRESH {
            self.reload();
        }
        ctx.request_repaint_after(REFRESH);
        if ctx.input(|i| i.viewport().close_requested()) && self.editor.as_ref().is_some_and(Editor::dirty) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Action::Quit);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE)) {
            self.save();
        }

        let title = match &self.editor {
            Some(ed) => format!("{}{} — MWP", if ed.dirty() { "● " } else { "" }, ed.path.file_name().unwrap().to_string_lossy()),
            None => format!("{} — MWP", self.root.display()),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        let mut action: Option<Action> = None;
        let mut save = false;
        let mut reload = false;
        let (mut new_dialog, mut open_dialog) = (false, false);

        egui::Panel::top("menu").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New project…").clicked() {
                        self.dialog = Some(self.new_project_dialog());
                    }
                    if ui.button("Open project…").clicked() {
                        self.dialog = Some(Dialog::Open { dir: self.root.display().to_string(), error: String::new() });
                    }
                    ui.separator();
                    save = ui.add_enabled(self.editor.is_some(), egui::Button::new("Save").shortcut_text(ctx.format_shortcut(&SAVE))).clicked();
                    reload = ui.button("Reload").clicked();
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        action = Some(Action::Quit);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Board").clicked() {
                        action = Some(Action::Board);
                    }
                    ui.checkbox(&mut self.show_done, "Done tasks on the board");
                    ui.checkbox(&mut self.show_problems, "Problems panel");
                });
            });
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Ok(v) = &self.view {
                    let (e, w) = (v.report.errors.len(), v.report.warnings.len());
                    let text = egui::RichText::new(format!("{e} error(s), {w} warning(s)"));
                    let text = if e > 0 { text.color(ui.visuals().error_fg_color) } else if w > 0 { text.color(ui.visuals().warn_fg_color) } else { text };
                    ui.toggle_value(&mut self.show_problems, text);
                    ui.separator();
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
                        ui.heading("No MWP project open");
                        ui.colored_label(ui.visuals().error_fg_color, e);
                        ui.add_space(12.0);
                        new_dialog = ui.button("New project…").clicked();
                        open_dialog = ui.button("Open project…").clicked();
                    });
                });
            }
            Ok(v) => {
                egui::Panel::left("explorer").resizable(true).default_size(260.0).show(ui, |ui| {
                    let name = v.project.of_kind("project").next().map_or("", |d| d.get("name"));
                    ui.strong(if name.is_empty() { "MWP" } else { name });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("explorer").auto_shrink([false, false]).show(ui, |ui| {
                        if ui.selectable_label(self.editor.is_none(), "Board").clicked() {
                            action = Some(Action::Board);
                        }
                        egui::CollapsingHeader::new(format!("Agents at work ({})", v.claims.len())).default_open(true).show(ui, |ui| {
                            for c in &v.claims {
                                let modules: Vec<&str> = v.locks.iter().filter(|l| l.1.task == c.task).map(|l| l.0.as_str()).collect();
                                if ui.selectable_label(false, format!("{}  ·  {}", c.task, c.agent)).clicked() {
                                    action = v.project.by_id("task", &c.task).map(|t| Action::Open(t.path.clone()));
                                }
                                dim(ui, if modules.is_empty() { "no module locked".into() } else { format!("locks: {}", modules.join(", ")) });
                            }
                            if v.claims.is_empty() {
                                ui.weak("No task in progress.");
                            }
                        });
                        for (label, entries) in &v.tree {
                            egui::CollapsingHeader::new(format!("{label} ({})", entries.len())).default_open(*label != "Done tasks").show(ui, |ui| {
                                for e in entries {
                                    let open = self.editor.as_ref().is_some_and(|ed| ed.path == e.path);
                                    let text = egui::RichText::new(&e.label);
                                    let text = if e.ok { text } else { text.color(ui.visuals().error_fg_color) };
                                    if ui.selectable_label(open, text).on_hover_text(v.project.rel(&e.path)).clicked() {
                                        action = Some(Action::Open(e.path.clone()));
                                    }
                                }
                            });
                        }
                    });
                });

                if self.show_problems {
                    egui::Panel::bottom("problems").resizable(true).default_size(110.0).show(ui, |ui| {
                        ui.strong("Problems");
                        egui::ScrollArea::vertical().id_salt("problems").auto_shrink([false, false]).show(ui, |ui| {
                            let errors = v.report.errors.iter().map(|l| (l, ui.visuals().error_fg_color, "ERROR"));
                            let warnings = v.report.warnings.iter().map(|l| (l, ui.visuals().warn_fg_color, "WARN "));
                            for (line, color, tag) in errors.chain(warnings).collect::<Vec<_>>() {
                                let r = ui.selectable_label(false, egui::RichText::new(format!("{tag} {line}")).color(color).monospace());
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

                egui::CentralPanel::default().show(ui, |ui| match self.editor.as_mut() {
                    Some(ed) => {
                        ui.horizontal(|ui| {
                            ui.strong(v.project.rel(&ed.path));
                            if ed.dirty() {
                                ui.colored_label(ui.visuals().warn_fg_color, "unsaved");
                            }
                            save |= ui.add_enabled(ed.dirty(), egui::Button::new("Save")).clicked();
                            if ui.button("Close").clicked() {
                                action = Some(Action::Board);
                            }
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().id_salt("editor").auto_shrink([false, false]).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut ed.text).code_editor().desired_width(f32::INFINITY).min_size(ui.available_size()));
                        });
                    }
                    None => {
                        let columns: Vec<_> = COLUMNS.iter().filter(|c| c.0 != "done" || self.show_done).collect();
                        ui.columns(columns.len(), |cols| {
                            for (ui, (status, label)) in cols.iter_mut().zip(columns) {
                                let mut tasks: Vec<&Doc> = v.project.of_kind("task").filter(|t| t.get("status") == *status).collect();
                                tasks.sort_by_key(|t| id_num(t.id()));
                                ui.strong(format!("{label} ({})", tasks.len()));
                                ui.separator();
                                egui::ScrollArea::vertical().id_salt(*status).show(ui, |ui| {
                                    for t in tasks {
                                        if card(ui, t) {
                                            action = Some(Action::Open(t.path.clone()));
                                        }
                                    }
                                });
                            }
                        });
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
        if let Some(a) = self.dialogs(&ctx) {
            action = Some(a);
        }

        if let Some(ed) = self.editor.as_ref().filter(|_| self.pending.is_some()) {
            let file = ed.path.file_name().unwrap().to_string_lossy().into_owned();
            let mut choice = None;
            egui::Modal::new(egui::Id::unique("unsaved")).show(&ctx, |ui| {
                ui.heading("Unsaved changes");
                ui.label(format!("Save the changes to {file}?"));
                ui.separator();
                ui.horizontal(|ui| {
                    for (label, c) in [("Save", "save"), ("Don't save", "discard"), ("Cancel", "cancel")] {
                        if ui.button(label).clicked() {
                            choice = Some(c);
                        }
                    }
                });
            });
            match choice {
                Some("save") if self.save() => {
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
            self.save();
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
    eframe::run_native("MWP", options, Box::new(|_cc| Ok(Box::new(App::new(root)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_lists_docs_and_flags_broken_ones() {
        let root = std::env::temp_dir().join(format!("mwp-gui-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        scaffold::init(&root, Some("Demo"), &[&AGENTS[0]], false).unwrap();
        fs::write(root.join("mwp/tasks/open/T-0001-a.md"), "---\nkind: task\nid: T-0001\nstatus: open\nsize: S\nmodules: []\n---\n# T-0001 — Add login\n").unwrap();
        fs::write(root.join("mwp/tasks/open/T-0002-b.md"), "# no front matter\n").unwrap();

        let v = load(&root).unwrap();
        let open = &v.tree.iter().find(|s| s.0 == "Open tasks").unwrap().1;
        assert_eq!(open.iter().map(|e| (e.label.as_str(), e.ok)).collect::<Vec<_>>(), [("T-0001  Add login", true), ("T-0002-b.md", false)]);
        assert!(v.tree[0].1.iter().any(|e| e.label == "constitution.md"));

        let line = v.report.errors.iter().find(|l| l.contains("T-0002")).unwrap();
        assert_eq!(problem_path(&root, line), Some(root.join("mwp/tasks/open/T-0002-b.md")));

        // CRLF files keep their endings after an edit.
        let f = root.join("mwp/glossary.md");
        fs::write(&f, "a\r\nb\r\n").unwrap();
        let mut app = App::new(root.clone());
        app.editor = Some(Editor::open(&f).unwrap());
        app.editor.as_mut().unwrap().text.push_str("c\n");
        assert!(app.save());
        assert_eq!(fs::read_to_string(&f).unwrap(), "a\r\nb\r\nc\r\n");
        let _ = fs::remove_dir_all(&root);
    }

    /// Drives the real UI headless: open, edit, save, the unsaved guard, and New project.
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
        let mut h = Harness::builder().with_size([1280.0, 760.0]).build_eframe(move |_| App::new(start));
        h.run_steps(3);

        // Open from the explorer, type, save with Ctrl+S.
        h.get_by_label("T-0001  Add login").click();
        h.run_steps(3);
        assert_eq!(h.state().editor.as_ref().map(|e| e.path.clone()), Some(task.clone()));
        h.get_by_role(Role::MultilineTextInput).focus();
        h.run_steps(2);
        h.get_by_role(Role::MultilineTextInput).type_text("edited");
        h.run_steps(3);
        assert!(h.state().editor.as_ref().unwrap().dirty());
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::S]);
        h.run_steps(3);
        assert!(fs::read_to_string(&task).unwrap().contains("edited"));
        assert!(!h.state().editor.as_ref().unwrap().dirty());

        // Leaving with unsaved edits asks first; "Don't save" drops them.
        h.get_by_role(Role::MultilineTextInput).focus();
        h.run_steps(2);
        h.get_by_role(Role::MultilineTextInput).type_text("lost");
        h.run_steps(3);
        h.get_by_label("Board").click();
        h.run_steps(3);
        assert!(h.state().pending.is_some() && h.state().editor.is_some());
        h.get_by_label("Don't save").click();
        h.run_steps(3);
        assert!(h.state().editor.is_none());
        assert!(!fs::read_to_string(&task).unwrap().contains("lost"));

        // File > New project creates the project and opens its PROJECT.md.
        h.get_by_label("File").click();
        h.run_steps(3);
        h.get_by_label("New project…").click();
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
        assert_eq!(h.state().editor.as_ref().map(|e| e.path.clone()), Some(new.join("mwp/PROJECT.md")));
        let _ = fs::remove_dir_all(&base);
    }
}
