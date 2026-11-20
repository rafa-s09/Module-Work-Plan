//! The explorer: the project read from disk, its sections and folders, file icons.

use super::*;

const PROJECT_FILES: [&str; 4] = ["PROJECT.md", "constitution.md", "map.md", "glossary.md"];
/// Explorer order. The two task folders are drawn last, as "Open" and "Done" under one "Tasks" group.
const SECTIONS: [(&str, &str, MaterialIcon); 6] = [
    ("Modules", "modules", ICON_EXTENSION),
    ("Specs", "specs", ICON_ARTICLE),
    ("Decisions", "decisions", ICON_GAVEL),
    // Resources of any type, not MWP docs.
    ("Content", "content", ICON_PERM_MEDIA),
    ("Open tasks", "tasks/open", ICON_TASK),
    ("Done tasks", "tasks/done", ICON_TASK_ALT),
];

/// A file in the explorer: its ID (empty for the project files) and its title or file name.
/// `ok` is false for a doc without front matter.
pub struct Entry {
    pub path: PathBuf,
    pub id: String,
    pub title: String,
    pub ok: bool,
}

/// One read of the project from disk.
pub struct View {
    pub project: Project,
    pub report: Report,
    pub claims: Vec<Claim>,
    pub locks: Vec<(String, Claim)>,
    pub tree: Vec<(&'static str, MaterialIcon, Vec<Entry>)>,
    /// Every folder under mwp/modules/ and mwp/content/, parents before children, so empty ones show too.
    pub dirs: Vec<PathBuf>,
}

pub fn load(root: &Path) -> Result<View, String> {
    let project = Project::open(root)?;
    let mut dirs = Vec::new();
    for area in Area::ALL {
        subdirs(&project.mwp.join(area.dir()), &mut dirs);
    }
    Ok(View { report: lint(&project), claims: state::claims(&project.state), locks: state::locks(&project.state), tree: tree(&project), dirs, project })
}

/// The explorer sections that keep their folders.
#[derive(Clone, Copy, PartialEq)]
pub enum Area {
    Modules,
    Content,
}

impl Area {
    pub const ALL: [Area; 2] = [Area::Modules, Area::Content];
    pub fn dir(self) -> &'static str {
        match self {
            Area::Modules => "modules",
            Area::Content => "content",
        }
    }
}

pub fn subdirs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    let mut dirs: Vec<PathBuf> = read.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for d in dirs {
        out.push(d.clone());
        subdirs(&d, out);
    }
}

/// A folder as `a/b`, relative to its area (mwp/modules/ or mwp/content/).
pub fn rel_folder(area: &Path, dir: &Path) -> String {
    dir.strip_prefix(area).map_or(String::new(), |r| r.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
}

/// What a folder's context menu asked for. Folders are given as `a/b` under their area.
pub enum FolderCmd {
    NewModule(String),
    /// Pick files and copy them into this content folder.
    AddFiles(String),
    NewFolder(Area, String),
    Remove(PathBuf),
    Rename(PathBuf),
    /// Move the file or folder into the folder (dragged onto it).
    Move(PathBuf, PathBuf),
}

/// A folder title as the place a file or folder dragged in the explorer is dropped into.
/// Dropping it where it already is does nothing.
pub fn drop_target(ui: &egui::Ui, header: &egui::Response, dir: &Path, cmd: &mut Option<FolderCmd>) {
    let fits = |from: &PathBuf| from.parent() != Some(dir) && !dir.starts_with(from);
    if header.dnd_hover_payload::<PathBuf>().is_some_and(|f| fits(&f)) {
        ui.painter().rect_stroke(header.rect.expand(2.0), 4.0, egui::Stroke::new(1.5, ui.visuals().selection.stroke.color), egui::StrokeKind::Outside);
    }
    if let Some(from) = header.dnd_release_payload::<PathBuf>().filter(|f| fits(f)) {
        *cmd = Some(FolderCmd::Move((*from).clone(), dir.to_path_buf()));
    }
}

/// Go to file: the files whose ID, title or file name hold `query` (any case), those that start
/// with it first, at most 50. An empty query lists the files in explorer order.
pub fn quick_matches<'a>(tree: &'a [(&'static str, MaterialIcon, Vec<Entry>)], query: &str) -> Vec<&'a Entry> {
    let q = query.trim().to_lowercase();
    let mut found: Vec<(bool, &Entry)> = tree.iter().flat_map(|s| &s.2).filter_map(|e| {
        let name = folder_name(&e.path).to_lowercase();
        let (id, title) = (e.id.to_lowercase(), e.title.to_lowercase());
        let starts = id.starts_with(&q) || name.starts_with(&q);
        (starts || title.contains(&q) || name.contains(&q) || id.contains(&q)).then_some((!starts, e))
    }).collect();
    found.sort_by_key(|f| f.0); // stable: explorer order inside each group
    found.into_iter().take(50).map(|f| f.1).collect()
}

/// Lets a file or folder of Modules or Content be dragged onto a folder.
pub fn drag_source(r: &egui::Response, path: &Path) -> egui::Response {
    let r = r.interact(egui::Sense::drag());
    r.dnd_set_drag_payload(path.to_path_buf());
    r
}

/// The context menu of the Modules and Content sections and of each folder in them.
pub fn folder_menu(ui: &mut egui::Ui, area: Area, rel: &str, dir: Option<&Path>, cmd: &mut Option<FolderCmd>) {
    let (icon, label, add) = match area {
        Area::Modules => (ICON_EXTENSION, tr("New module here…"), FolderCmd::NewModule(rel.to_string())),
        Area::Content => (ICON_UPLOAD_FILE, tr("Add files here…"), FolderCmd::AddFiles(rel.to_string())),
    };
    if ui.button(ic(icon, label)).clicked() {
        *cmd = Some(add);
        ui.close();
    }
    if ui.button(ic(ICON_CREATE_NEW_FOLDER, tr("New folder…"))).clicked() {
        *cmd = Some(FolderCmd::NewFolder(area, rel.to_string()));
        ui.close();
    }
    if let Some(dir) = dir {
        if ui.add(egui::Button::new(ic(ICON_DRIVE_FILE_RENAME_OUTLINE, tr("Rename…")))).clicked() {
            *cmd = Some(FolderCmd::Rename(dir.to_path_buf()));
            ui.close();
        }
        let empty = fs::read_dir(dir).is_ok_and(|mut r| r.next().is_none());
        let remove = ui.add_enabled(empty, egui::Button::new(ic(ICON_DELETE, tr("Delete folder")))).on_disabled_hover_text(tr("Only an empty folder can be deleted"));
        if remove.clicked() {
            *cmd = Some(FolderCmd::Remove(dir.to_path_buf()));
            ui.close();
        }
    }
}

/// A section without folders: its files, or "Empty".
pub fn flat(ui: &mut egui::Ui, entries: &[Entry], files: &mut dyn FnMut(&mut egui::Ui, &[&Entry])) {
    files(ui, &entries.iter().collect::<Vec<_>>());
    if entries.is_empty() {
        ui.weak(tr("Empty"));
    }
}

/// The files of `dir` in an area: its subfolders first, each collapsible with a context menu, then its files.
#[allow(clippy::too_many_arguments)]
pub fn folder_tree(ui: &mut egui::Ui, area: Area, dir: &Path, root: &Path, dirs: &[PathBuf], entries: &[&Entry], files: &mut dyn FnMut(&mut egui::Ui, &[&Entry]), cmd: &mut Option<FolderCmd>) {
    let subs: Vec<&PathBuf> = dirs.iter().filter(|d| d.parent() == Some(dir)).collect();
    for sub in &subs {
        let inside: Vec<&Entry> = entries.iter().filter(|e| e.path.starts_with(sub)).copied().collect();
        let (_, header) = group(ui, ICON_FOLDER, &folder_name(sub), inside.len(), true, None, |ui| folder_tree(ui, area, sub, root, dirs, &inside, files, cmd));
        let header = drag_source(&header, sub);
        drop_target(ui, &header, sub, cmd);
        header.context_menu(|ui| folder_menu(ui, area, &rel_folder(root, sub), Some(sub), cmd));
    }
    let here: Vec<&Entry> = entries.iter().filter(|e| e.path.parent() == Some(dir)).copied().collect();
    files(ui, &here);
    if here.is_empty() && subs.is_empty() {
        ui.weak(tr("Empty"));
    }
}

pub fn entry(p: &Project, path: PathBuf) -> Entry {
    let file = path.file_name().unwrap().to_string_lossy().into_owned();
    let (id, title) = match p.docs.iter().find(|d| d.path == path) {
        Some(d) if !d.id().is_empty() => (d.id().to_string(), if d.title().is_empty() || d.title() == d.id() { String::new() } else { d.title().to_string() }),
        _ => (String::new(), file),
    };
    Entry { ok: !p.invalid.contains(&path), id, title, path }
}

/// The explorer: the project files, then one section per MWP folder. Templates (`_*`) are left out.
pub fn tree(p: &Project) -> Vec<(&'static str, MaterialIcon, Vec<Entry>)> {
    let top = PROJECT_FILES.iter().map(|f| p.mwp.join(f)).filter(|f| f.exists()).map(|f| entry(p, f)).collect();
    let mut out = vec![("Project", ICON_FOLDER, top)];
    for (label, dir, icon) in SECTIONS {
        let mut files = Vec::new();
        walk(&p.mwp.join(dir), &mut files);
        let name = |f: &PathBuf| f.file_name().unwrap().to_string_lossy().into_owned();
        if dir == "content" {
            files.retain(|f| name(f) != ".gitkeep");
        } else {
            files.retain(|f| f.extension().is_some_and(|e| e == "md") && !name(f).starts_with('_'));
        }
        files.sort();
        out.push((label, icon, files.into_iter().map(|f| entry(p, f)).collect()));
    }
    out
}

/// The icon and colour of a task status.
pub fn status_icon(ui: &egui::Ui, status: &str) -> (MaterialIcon, Color32) {
    let v = ui.visuals();
    match status {
        "in_progress" => (ICON_PENDING, v.hyperlink_color),
        "blocked" => (ICON_BLOCK, v.error_fg_color),
        "review" => (ICON_RATE_REVIEW, v.warn_fg_color),
        "done" => (ICON_CHECK_CIRCLE, Color32::from_rgb(90, 170, 100)),
        "replanned" => (ICON_ALT_ROUTE, v.warn_fg_color),
        _ => (ICON_RADIO_BUTTON_UNCHECKED, v.weak_text_color()),
    }
}

/// Content files the editor opens; any other opens in the system's app for it.
pub const TEXT_FILES: [&str; 7] = ["md", "txt", "json", "csv", "yaml", "yml", "toml"];

pub fn extension(path: &Path) -> String {
    path.extension().map_or(String::new(), |e| e.to_string_lossy().to_lowercase())
}

/// Opens a file in the system's app for its type, or shows it in the file manager.
pub fn open_external(path: &Path, reveal: bool) -> Result<(), String> {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("explorer");
        c.arg(if reveal { format!("/select,{}", path.display()) } else { path.display().to_string() });
        c
    } else {
        let mut c = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" });
        match (reveal, cfg!(target_os = "macos")) {
            (true, true) => c.arg("-R").arg(path),
            (true, false) => c.arg(path.parent().unwrap_or(path)),
            (false, _) => c.arg(path),
        };
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| format!("cannot open {}: {e}", path.display()))
}

/// The icon of a file: by kind, by status for a task, by name for the project files, by type for content.
pub fn file_icon(ui: &egui::Ui, p: &Project, path: &Path) -> (MaterialIcon, Color32) {
    if p.invalid.iter().any(|f| f == path) {
        return (ICON_ERROR, ui.visuals().error_fg_color);
    }
    if path.starts_with(p.mwp.join("content")) {
        let teal = Color32::from_rgb(80, 175, 160);
        return match extension(path).as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" => (ICON_IMAGE, teal),
            "pdf" => (ICON_PICTURE_AS_PDF, Color32::from_rgb(220, 90, 80)),
            "fig" | "sketch" | "psd" | "xd" | "ai" => (ICON_PALETTE, Color32::from_rgb(200, 120, 200)),
            "csv" | "xlsx" | "xls" | "json" | "yaml" | "yml" | "toml" => (ICON_TABLE_CHART, teal),
            "mp4" | "mov" | "webm" => (ICON_MOVIE, teal),
            "mp3" | "wav" | "ogg" => (ICON_AUDIO_FILE, teal),
            "md" | "txt" => (ICON_DESCRIPTION, ui.visuals().weak_text_color()),
            _ => (ICON_FILE_PRESENT, ui.visuals().weak_text_color()),
        };
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

/// A collapsible explorer section: icon, title and count, with an optional "+" on the right.
/// Clicking the title toggles it too. Returns whether "+" was clicked, and the title (for a context menu).
pub fn group(ui: &mut egui::Ui, icon: MaterialIcon, title: &str, count: usize, open: bool, add: Option<&str>, body: impl FnOnce(&mut egui::Ui)) -> (bool, egui::Response) {
    let id = ui.make_persistent_id(("section", title));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open);
    let mut plus = false;
    let mut title_r = None;
    let header = ui.horizontal(|ui| {
        state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
        let label = (icon.rich_text().color(ui.visuals().weak_text_color()), RichText::new(tr(title)).strong(), RichText::new(count.to_string()).weak());
        let r = ui.add(egui::Button::new(label).frame(false));
        if r.clicked() {
            state.toggle(ui);
        }
        title_r = Some(r);
        if let Some(tip) = add {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0); // clear of the scroll bar
                plus = ui.add(egui::Button::new(ICON_ADD.rich_text()).frame(false)).on_hover_text(tr(tip)).clicked();
            });
        }
    });
    state.show_body_indented(&header.response, ui, body);
    (plus, title_r.unwrap())
}

impl App {
    /// The left panel: project name, Board, agents at work, then the sections.
    pub fn explorer(&self, ui: &mut egui::Ui, v: &View, ask: &mut Asks) {
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
                if ui.selectable_label(self.active.is_none(), ic(ICON_VIEW_KANBAN, tr("Board"))).clicked() {
                    ask.action = Some(Action::Board);
                }
                ui.add_space(4.0);
                group(ui, ICON_SMART_TOY, "Agents at work", v.claims.len(), true, None, |ui| {
                    for c in &v.claims {
                        let modules: Vec<&str> = v.locks.iter().filter(|l| l.1.task == c.task).map(|l| l.0.as_str()).collect();
                        let (icon, color) = status_icon(ui, "in_progress");
                        if ui.selectable_label(false, (icon.rich_text().color(color), RichText::new(&c.task).strong(), RichText::new(&c.agent))).clicked() {
                            ask.action = v.project.by_id("task", &c.task).map(|t| Action::Open(t.path.clone()));
                        }
                        ui.horizontal(|ui| {
                            ui.label(ICON_LOCK.rich_text().small().weak());
                            dim(ui, if modules.is_empty() { tr("no module locked").into() } else { modules.join(", ") });
                        });
                    }
                    if v.claims.is_empty() {
                        ui.weak(tr("No task in progress."));
                    }
                });
                let mut files = |ui: &mut egui::Ui, entries: &[&Entry]| {
                        for e in entries {
                            let open = self.current().is_some_and(|ed| ed.path == e.path);
                            let (icon, color) = file_icon(ui, &v.project, &e.path);
                            let title = if e.ok { RichText::new(&e.title) } else { RichText::new(&e.title).color(ui.visuals().error_fg_color) };
                            let r = if e.id.is_empty() {
                                ui.selectable_label(open, (icon.rich_text().color(color), title))
                            } else {
                                ui.selectable_label(open, (icon.rich_text().color(color), RichText::new(&e.id).strong(), title))
                            };
                            let rel = v.project.rel(&e.path);
                            let movable = ops::movable(&v.project, &e.path);
                            let in_area = Area::ALL.iter().any(|a| e.path.starts_with(v.project.mwp.join(a.dir())));
                            let r = if movable && in_area { drag_source(&r, &e.path) } else { r };
                            let r = r.on_hover_text(&rel);
                            // Content that is not text opens in the system's app, not in the editor.
                            let external = e.path.starts_with(v.project.mwp.join("content")) && !TEXT_FILES.contains(&extension(&e.path).as_str());
                            if r.clicked() {
                                if external {
                                    ask.open_error = open_external(&e.path, false).err();
                                } else {
                                    ask.action = Some(Action::Open(e.path.clone()));
                                }
                            }
                            r.context_menu(|ui| {
                                let open = if external { ic(ICON_OPEN_IN_NEW, tr("Open with system app")) } else { ic(ICON_EDIT, tr("Open file")) };
                                if ui.button(open).clicked() {
                                    if external {
                                        ask.open_error = open_external(&e.path, false).err();
                                    } else {
                                        ask.action = Some(Action::Open(e.path.clone()));
                                    }
                                    ui.close();
                                }
                                if ui.button(ic(ICON_FOLDER_OPEN, tr("Show in folder"))).clicked() {
                                    ask.open_error = open_external(&e.path, true).err();
                                    ui.close();
                                }
                                // Tasks cite resources by path: copy it to paste into a task.
                                if ui.button(ic(ICON_CONTENT_COPY, tr("Copy path"))).on_hover_text(&rel).clicked() {
                                    ui.ctx().copy_text(rel.clone());
                                    ui.close();
                                }
                                if movable && ui.add(egui::Button::new(ic(ICON_DRIVE_FILE_RENAME_OUTLINE, tr("Rename…"))).shortcut_text("F2")).clicked() {
                                    ask.rename = Some(e.path.clone());
                                    ui.close();
                                }
                                if ops::deletable(&v.project, &e.path) && ui.button(ic(ICON_DELETE, tr("Delete…"))).clicked() {
                                    ask.ask_delete = Some(e.path.clone());
                                    ui.close();
                                }
                            });
                        }
                };
                for (label, icon, entries) in v.tree.iter().filter(|s| !s.0.ends_with("tasks")) {
                    let add = match *label {
                        "Modules" => Some("New module"),
                        "Specs" => Some("New SPEC"),
                        "Decisions" => Some("New ADR"),
                        "Content" => Some("Add files (or drop them on the window)"),
                        _ => None,
                    };
                    // Modules and Content keep their folders; right-click a folder (or the section) to add to it.
                    let area = Area::ALL.into_iter().find(|a| SECTIONS.iter().any(|s| s.0 == *label && s.1 == a.dir()));
                    let (plus, header) = match area {
                        Some(area) => {
                            let root = v.project.mwp.join(area.dir());
                            let refs: Vec<&Entry> = entries.iter().collect();
                            group(ui, *icon, label, entries.len(), true, add, |ui| folder_tree(ui, area, &root, &root, &v.dirs, &refs, &mut files, &mut ask.folder_cmd))
                        }
                        None => group(ui, *icon, label, entries.len(), true, add, |ui| flat(ui, entries, &mut files)),
                    };
                    if let Some(area) = area {
                        drop_target(ui, &header, &v.project.mwp.join(area.dir()), &mut ask.folder_cmd);
                        header.context_menu(|ui| folder_menu(ui, area, "", None, &mut ask.folder_cmd));
                    }
                    if plus {
                        (ask.new_module, ask.new_spec, ask.new_adr) = (*label == "Modules", *label == "Specs", *label == "Decisions");
                        if *label == "Content" {
                            ask.folder_cmd = Some(FolderCmd::AddFiles(String::new()));
                        }
                    }
                }
                // Tasks last, as one group with Open and Done inside.
                let tasks = |name: &str| v.tree.iter().find(|s| s.0 == name).map(|s| (s.1, &s.2[..])).unwrap();
                let ((open_icon, open), (done_icon, done)) = (tasks("Open tasks"), tasks("Done tasks"));
                let mut plus = false;
                group(ui, ICON_ASSIGNMENT, "Tasks", open.len() + done.len(), true, None, |ui| {
                    plus = group(ui, open_icon, "Open", open.len(), true, Some("New task"), |ui| flat(ui, open, &mut files)).0;
                    group(ui, done_icon, "Done", done.len(), false, None, |ui| flat(ui, done, &mut files));
                });
                ask.new_task |= plus;
            });
        });
        // What is being dragged follows the pointer.
        if let (Some(path), Some(pointer)) = (egui::DragAndDrop::payload::<PathBuf>(ui.ctx()), ui.ctx().pointer_latest_pos()) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            egui::Area::new(egui::Id::unique("dragged-file")).order(egui::Order::Tooltip).interactable(false).fixed_pos(pointer + egui::vec2(12.0, 8.0)).show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| ui.label(folder_name(&path)));
            });
        }
    }
}
