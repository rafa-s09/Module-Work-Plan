//! The dialogs (new project, task, module, SPEC, ADR, folder, start task, delete) and the recent projects.

use super::*;

pub const LIFETIMES: [(&str, &str); 2] = [("durable", "Lasting reference that stays with its owner"), ("transient", "Working document, incorporated or archived when done")];
pub const MODULE_TYPES: [(&str, &str); 5] = [
    ("domain", "Business rules and states"),
    ("ui", "Screens and components"),
    ("api", "Entry points: HTTP routes, CLI commands, webhooks, jobs"),
    ("infra", "Storage, external services, deploy, tooling"),
    ("lib", "Code shared by several modules"),
];
pub const SIZES: [(&str, &str); 3] = [("S", "Small: a few lines, one module"), ("M", "Medium: a feature in one or two modules"), ("L", "Large: several modules, plan first")];

pub enum Dialog {
    New { dir: String, name: String, agents: Vec<bool>, skills: bool, error: String },
    Open { dir: String, error: String },
    NewTask { title: String, size: &'static str, modules: Vec<(String, bool)>, error: String },
    /// `folder` is where it goes under mwp/modules/, as `a/b`; empty for the top.
    NewModule { folder: String, id: String, kind: &'static str, summary: String, error: String },
    /// A folder in Modules or Content; `parent` is `a/b` under that area.
    NewFolder { area: Area, parent: String, name: String, error: String },
    /// `adrs`: the decisions in force (ID, title) it may replace; `supersedes` is empty for none.
    NewAdr { title: String, modules: Vec<(String, bool)>, adrs: Vec<(String, String)>, supersedes: String, error: String },
    /// `agents`: the agents set up in the project, offered as choices; `modules`: what it locks.
    StartTask { id: String, agent: String, agents: Vec<String>, modules: Vec<String>, error: String },
    /// File > Project agents: `agents` checked, by `AGENTS` order; `installed` ones stay checked.
    /// `update` rewrites the installed agents' skills with this version.
    Agents { agents: Vec<bool>, installed: Vec<bool>, update: bool, error: String },
    /// A message with an OK button, for an action that could not be done.
    Notice { title: String, text: String },
    /// `owners`: the modules, then the tasks not done.
    NewSpec { title: String, owner: String, owners: Vec<String>, lifetime: &'static str, error: String },
    /// `label` names the file; `note` warns about an agent working on it.
    Delete { path: PathBuf, label: String, note: String, error: String },
}

impl App {
    pub fn new_project_dialog(&self) -> Dialog {
        Dialog::New { dir: self.root.display().to_string(), name: folder_name(&self.root), agents: AGENTS.iter().map(|a| a.id == "claude").collect(), skills: true, error: String::new() }
    }

    /// Project agents, with the agents whose skills are set up here checked.
    pub fn agents_dialog(&self) -> Option<Dialog> {
        self.view.as_ref().ok()?;
        let here = scaffold::installed(&self.root);
        let installed: Vec<bool> = AGENTS.iter().map(|a| here.iter().any(|h| h.id == a.id)).collect();
        Some(Dialog::Agents { agents: installed.clone(), update: installed.contains(&true), installed, error: String::new() })
    }

    /// New task, with the module open in the editor already checked.
    pub fn new_task_dialog(&self) -> Option<Dialog> {
        let v = self.view.as_ref().ok()?;
        let open = self.current().map(|e| e.path.as_path());
        let mut modules: Vec<(String, bool)> = v.project.of_kind("module").map(|m| (m.id().to_string(), open == Some(m.path.as_path()))).collect();
        modules.sort();
        Some(Dialog::NewTask { title: String::new(), size: "M", modules, error: String::new() })
    }

    /// New ADR, with the module open in the editor already checked.
    pub fn new_adr_dialog(&self) -> Option<Dialog> {
        let Some(Dialog::NewTask { modules, .. }) = self.new_task_dialog() else { return None };
        let v = self.view.as_ref().ok()?;
        let mut adrs: Vec<&Doc> = v.project.of_kind("adr").filter(|d| matches!(d.get("status"), "accepted" | "proposed")).collect();
        adrs.sort_by_key(|d| id_num(d.id()));
        let adrs = adrs.iter().map(|d| (d.id().to_string(), d.title().to_string())).collect();
        Some(Dialog::NewAdr { title: String::new(), modules, adrs, supersedes: String::new(), error: String::new() })
    }

    /// New SPEC, owned by the module or task open in the editor when there is one.
    pub fn new_spec_dialog(&self) -> Option<Dialog> {
        let v = self.view.as_ref().ok()?;
        let mut modules: Vec<&Doc> = v.project.of_kind("module").collect();
        modules.sort_by(|a, b| a.id().cmp(b.id()));
        let mut tasks: Vec<&Doc> = v.project.of_kind("task").filter(|t| t.get("status") != "done").collect();
        tasks.sort_by_key(|t| id_num(t.id()));
        let all: Vec<&Doc> = modules.into_iter().chain(tasks).collect();
        let open = self.current().and_then(|e| all.iter().find(|d| d.path == e.path));
        let owner = open.or(all.first()).map_or(String::new(), |d| d.id().to_string());
        Some(Dialog::NewSpec { title: String::new(), owner, owners: all.iter().map(|d| d.id().to_string()).collect(), lifetime: "durable", error: String::new() })
    }

    /// Asks before deleting; says when an agent works on the task or open tasks use the module.
    pub fn delete_dialog(&self, path: PathBuf) -> Option<Dialog> {
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
    pub fn dialogs(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let mut close = false;
        let mut deleted: Option<PathBuf> = None;
        let mut changed = false; // the dialog changed the project without opening a file
        let mut next: Option<Dialog> = None; // shown in place of this one
        let root = self.root.clone();
        let recent = self.recent.clone();
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
                                Ok(lines) => action = created(&root, &lines),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::NewModule { folder, id, kind, summary, error } => {
                egui::Modal::new(egui::Id::unique("new-module")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    dialog_title(ui, ICON_EXTENSION, "New module");
                    let dir = if folder.is_empty() { "mwp/modules/".to_string() } else { format!("mwp/modules/{folder}/") };
                    ui.weak(format!("Created in {dir} from the module template and added to map.md."));
                    ui.add_space(12.0);
                    let file = slug(id);
                    section(ui, "Module", |ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(id).hint_text("order, user-auth").desired_width(f32::INFINITY));
                        ui.weak(if file.is_empty() { "Lowercase words joined by -.".to_string() } else { format!("ID {file}  ·  {dir}{file}.md") });
                        ui.add_space(6.0);
                        ui.label("Type");
                        ui.horizontal_wrapped(|ui| {
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
                            match Project::open(&root).and_then(|p| ops::module_new(&p, folder, &file, kind, summary)) {
                                Ok(lines) => action = created(&root, &lines),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::NewFolder { area, parent, name, error } => {
                let area = *area;
                egui::Modal::new(egui::Id::unique("new-folder")).show(ctx, |ui| {
                    ui.set_width(420.0);
                    dialog_title(ui, ICON_CREATE_NEW_FOLDER, "New folder");
                    let base = area.dir();
                    let dir = if parent.is_empty() { format!("mwp/{base}/") } else { format!("mwp/{base}/{parent}/") };
                    ui.weak(match area {
                        Area::Modules => format!("A folder for modules, inside {dir}"),
                        Area::Content => format!("A folder for resources, inside {dir}"),
                    });
                    ui.add_space(12.0);
                    let file = slug(name);
                    section(ui, "Folder", |ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).hint_text("billing").desired_width(f32::INFINITY));
                        ui.weak(if file.is_empty() { "Lowercase words joined by -.".to_string() } else { format!("{dir}{file}/") });
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!file.is_empty(), primary(ui, "Create")).clicked() {
                            let made = Project::open(&root).and_then(|p| match area {
                                Area::Modules => ops::module_folder_new(&p, parent, &file),
                                Area::Content => ops::content_folder_new(&p, parent, &file),
                            });
                            match made {
                                Ok(_) => (changed, close) = (true, true),
                                Err(e) => *error = e,
                            }
                        }
                        close |= ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::NewSpec { title, owner, owners, lifetime, error } => {
                egui::Modal::new(egui::Id::unique("new-spec")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    dialog_title(ui, ICON_ARTICLE, "New SPEC");
                    ui.weak("Created in mwp/specs/ with the next free ID, from the SPEC template.");
                    ui.add_space(12.0);
                    section(ui, "SPEC", |ui| {
                        ui.label("Title");
                        ui.add(egui::TextEdit::singleline(title).hint_text("What it specifies").desired_width(f32::INFINITY));
                        ui.add_space(6.0);
                        ui.label("Lifetime");
                        ui.horizontal(|ui| {
                            for (l, _) in LIFETIMES {
                                ui.selectable_value(lifetime, l, format!("  {l}  "));
                            }
                        });
                        ui.weak(LIFETIMES.iter().find(|l| l.0 == *lifetime).map_or("", |l| l.1));
                    });
                    section(ui, "Owner", |ui| {
                        if owners.is_empty() {
                            ui.weak("No module or task yet: create one first.");
                        } else {
                            ui.weak("The module or task this SPEC belongs to.");
                            egui::ComboBox::from_id_salt("spec-owner").selected_text(owner.as_str()).width(ui.available_width()).show_ui(ui, |ui| {
                                for o in owners.iter() {
                                    ui.selectable_value(owner, o.clone(), o.as_str());
                                }
                            });
                        }
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!title.trim().is_empty() && !owner.is_empty(), primary(ui, "Create")).clicked() {
                            match Project::open(&root).and_then(|p| ops::spec_new(&p, title, owner, lifetime)) {
                                Ok(lines) => action = created(&root, &lines),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::NewAdr { title, modules, adrs, supersedes, error } => {
                egui::Modal::new(egui::Id::unique("new-adr")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    dialog_title(ui, ICON_GAVEL, "New ADR");
                    ui.weak("A decision record, created as proposed in mwp/decisions/ with the next free ID.");
                    ui.add_space(12.0);
                    section(ui, "Decision", |ui| {
                        ui.label("Title");
                        ui.add(egui::TextEdit::singleline(title).hint_text("What was decided, in a few words").desired_width(f32::INFINITY));
                        ui.add_space(6.0);
                        ui.label("Replaces");
                        let none = "None: a new decision".to_string();
                        let shown = adrs.iter().find(|a| a.0 == *supersedes).map_or(none.clone(), |a| format!("{}  {}", a.0, a.1));
                        egui::ComboBox::from_id_salt("adr-supersedes").selected_text(shown).width(ui.available_width()).show_ui(ui, |ui| {
                            ui.selectable_value(supersedes, String::new(), none);
                            for (id, t) in adrs.iter() {
                                ui.selectable_value(supersedes, id.clone(), format!("{id}  {t}"));
                            }
                        });
                        if !supersedes.is_empty() {
                            ui.weak(format!("{supersedes} will be marked superseded."));
                        }
                    });
                    section(ui, "Modules it governs", |ui| {
                        if modules.is_empty() {
                            ui.weak("No module yet.");
                        }
                        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
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
                            let old = Some(supersedes.as_str()).filter(|s| !s.is_empty());
                            match Project::open(&root).and_then(|p| ops::adr_new(&p, title, Some(&chosen.join(",")), old)) {
                                Ok(lines) => action = created(&root, &lines),
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::StartTask { id, agent, agents, modules, error } => {
                egui::Modal::new(egui::Id::unique("start-task")).show(ctx, |ui| {
                    ui.set_width(440.0);
                    dialog_title(ui, ICON_PLAY_ARROW, &format!("Start {id}"));
                    ui.weak(if modules.is_empty() { "Claims the task for an agent. It locks no module.".to_string() } else { format!("Claims the task for an agent and locks: {}.", modules.join(", ")) });
                    ui.add_space(12.0);
                    section(ui, "Agent", |ui| {
                        ui.add(egui::TextEdit::singleline(agent).hint_text("Who works on it").desired_width(f32::INFINITY));
                        if !agents.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                for a in agents.iter() {
                                    ui.selectable_value(agent, a.clone(), format!(" {a} "));
                                }
                            });
                        }
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!agent.trim().is_empty(), primary(ui, "Start")).clicked() {
                            match Project::open(&root).and_then(|p| ops::task_start(&p, id, agent.trim())) {
                                Ok(_) => (changed, close) = (true, true),
                                Err(e) => *error = e,
                            }
                        }
                        close |= ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::Agents { agents, installed, update, error } => {
                egui::Modal::new(egui::Id::unique("agents")).show(ctx, |ui| {
                    ui.set_width(540.0);
                    dialog_title(ui, ICON_SMART_TOY, "Project agents");
                    ui.weak("Adds the instruction file and the skills of each agent you check, like mwp init --agent. Existing files are never overwritten.");
                    ui.add_space(12.0);
                    section(ui, "Agents", |ui| {
                        egui::Grid::new("project-agents").num_columns(3).spacing([24.0, 6.0]).show(ui, |ui| {
                            for ((on, here), a) in agents.iter_mut().zip(installed.iter()).zip(AGENTS.iter()) {
                                ui.add_enabled(!here, egui::Checkbox::new(on, a.name));
                                ui.weak(format!("{}  ·  {}", a.pointer, a.skills));
                                ui.weak(if *here { "set up" } else { "" });
                                ui.end_row();
                            }
                        });
                    });
                    let any_here = installed.contains(&true);
                    section(ui, "Skills", |ui| {
                        ui.add_enabled(any_here, egui::Checkbox::new(update, "Update the skills of the agents already set up to this version"));
                        ui.weak("Like mwp init --update-skills: rewrites only their mwp and module skills.");
                    });
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        let new: Vec<&scaffold::Agent> = AGENTS.iter().zip(agents.iter().zip(installed.iter())).filter(|(_, (on, here))| **on && !**here).map(|(a, _)| a).collect();
                        let here: Vec<&scaffold::Agent> = AGENTS.iter().zip(installed.iter()).filter(|(_, h)| **h).map(|(a, _)| a).collect();
                        let update = *update && any_here;
                        if ui.add_enabled(!new.is_empty() || update, primary(ui, "Apply")).clicked() {
                            let mut lines = Vec::new();
                            let done = (if new.is_empty() { Ok(Vec::new()) } else { scaffold::init(&root, None, &new, true) })
                                .and_then(|l| {
                                    lines.extend(l);
                                    if update { scaffold::update_skills(&root, &here) } else { Ok(Vec::new()) }
                                });
                            match done {
                                Ok(l) => {
                                    lines.extend(l);
                                    let changes: Vec<&str> = lines.iter().filter(|l| l.starts_with("create") || l.starts_with("update")).map(String::as_str).collect();
                                    let text = if changes.is_empty() { "Nothing to change: the files were already there and the skills up to date.".to_string() } else { changes.join("\n") };
                                    next = Some(Dialog::Notice { title: "Project agents".into(), text });
                                    changed = true;
                                }
                                Err(e) => *error = e,
                            }
                        }
                        close = ui.add(secondary("Cancel")).clicked();
                    });
                });
            }
            Dialog::Notice { title, text } => {
                egui::Modal::new(egui::Id::unique("notice")).show(ctx, |ui| {
                    ui.set_width(420.0);
                    dialog_title(ui, ICON_INFO, title);
                    ui.add_space(6.0);
                    ui.label(text.as_str());
                    ui.separator();
                    right_row(ui, |ui| close = ui.add(primary(ui, "OK")).clicked());
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
                                if let Some(picked) = pick_folder(dir) {
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
                    ui.set_width(540.0);
                    dialog_title(ui, ICON_FOLDER_OPEN, "Open MWP project");
                    ui.weak("The folder that contains mwp/.");
                    ui.add_space(12.0);
                    let mut open = |folder: PathBuf, error: &mut String| match Project::open(&folder) {
                        Ok(_) => action = Some(Action::Project(folder, false)),
                        Err(e) => *error = format!("{e}. Use File > New project to create one."),
                    };
                    let mut chosen: Option<PathBuf> = None;
                    section(ui, "Folder", |ui| {
                        right_row(ui, |ui| {
                            if ui.button("Browse…").clicked() {
                                // A picked folder opens at once; a wrong one shows why below.
                                chosen = pick_folder(dir);
                                if let Some(f) = &chosen {
                                    *dir = f.display().to_string();
                                }
                            }
                            ui.add(egui::TextEdit::singleline(dir).hint_text("Folder that contains mwp/").desired_width(f32::INFINITY));
                        });
                        let folder = Path::new(dir.trim());
                        match () {
                            _ if dir.trim().is_empty() => ui.weak("Type a folder or browse for it."),
                            _ if folder.join("mwp").is_dir() => ui.weak("MWP project found."),
                            _ => ui.colored_label(ui.visuals().warn_fg_color, "No mwp/ in this folder."),
                        };
                    });
                    if !recent.is_empty() {
                        section(ui, "Recent", |ui| {
                            if let Some(f) = recent_list(ui, &recent) {
                                chosen = Some(f);
                            }
                        });
                    }
                    if let Some(f) = chosen {
                        open(f, error);
                    }
                    error_line(ui, error);
                    ui.separator();
                    right_row(ui, |ui| {
                        if ui.add_enabled(!dir.trim().is_empty(), primary(ui, "Open")).clicked() {
                            open(PathBuf::from(dir.trim()), error);
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
        if next.is_some() {
            self.dialog = next;
        }
        if let Some(path) = deleted {
            if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
                self.apply(ctx, Action::CloseTab(i)); // the file is gone; its edits go with it
            }
            self.reload();
        }
        if changed || matches!(action, Some(Action::Open(_))) {
            self.reload();
        }
        action
    }
}

/// Opens the file an `ops::*_new` reported; its first line is `create mwp/... (details)`.
pub fn created(root: &Path, lines: &[String]) -> Option<Action> {
    lines.iter().find_map(|l| l.strip_prefix("create ")?.split(" (").next()).map(|rel| Action::Open(root.join(rel)))
}

pub fn dialog_title(ui: &mut egui::Ui, icon: MaterialIcon, title: &str) {
    ui.horizontal(|ui| {
        ui.label(icon.rich_text().size(24.0).color(ui.visuals().hyperlink_color));
        ui.heading(title);
    });
}

pub fn error_line(ui: &mut egui::Ui, error: &str) {
    if !error.is_empty() {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

/// The main button of a dialog.
pub fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).strong()).fill(ui.visuals().selection.bg_fill).min_size(egui::vec2(96.0, 0.0))
}

pub fn secondary(text: &str) -> egui::Button<'static> {
    egui::Button::new(text.to_string()).min_size(egui::vec2(96.0, 0.0))
}

pub const RECENT_MAX: usize = 5;

/// Where the recent projects are kept: `<config>/mwp-gui/recent.txt`, one folder per line.
/// Not `<config>/mwp/`: that would make the config folder itself look like an MWP project.
pub fn recent_file() -> Option<PathBuf> {
    if cfg!(test) {
        return None; // tests never touch the user's list
    }
    let config = std::env::var_os("APPDATA").or_else(|| std::env::var_os("XDG_CONFIG_HOME")).map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(config.join("mwp-gui").join("recent.txt"))
}

/// The list with `dir` first, without duplicates or folders that lost their mwp/, at most five.
pub fn remember(list: &[PathBuf], dir: &Path) -> Vec<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut out = vec![dir.clone()];
    out.extend(list.iter().filter(|p| **p != dir && p.join("mwp").is_dir()).cloned());
    out.truncate(RECENT_MAX);
    out
}

/// The recent projects as rows (name, then folder); returns the one clicked.
pub fn recent_list(ui: &mut egui::Ui, recent: &[PathBuf]) -> Option<PathBuf> {
    let mut clicked = None;
    for p in recent {
        let path = p.display().to_string();
        ui.horizontal(|ui| {
            let name = (ICON_FOLDER.rich_text().color(ui.visuals().hyperlink_color), RichText::new(folder_name(p)).strong());
            let a = ui.add(egui::Button::new(name).frame(false)).on_hover_text(&path).clicked();
            // Truncated: a long path would otherwise widen the dialog.
            let b = ui.add(egui::Label::new(RichText::new(&path).weak()).truncate().sense(egui::Sense::click())).on_hover_text(&path).clicked();
            if a || b {
                clicked = Some(p.clone());
            }
        });
    }
    clicked
}

/// The native folder picker, starting at `start` when it is a folder.
pub fn pick_folder(start: &str) -> Option<PathBuf> {
    let start = Path::new(start.trim());
    let dialog = rfd::FileDialog::new().set_title("Project folder");
    let dialog = if start.is_dir() { dialog.set_directory(start) } else { dialog };
    dialog.pick_folder()
}

pub fn folder_name(path: &Path) -> String {
    path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/// One row laid out from the right. Its height is fixed: a right-to-left layout would
/// otherwise take all the height it is offered, and the modal would grow every frame.
pub fn right_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::right_to_left(egui::Align::Center), add);
}

/// A titled, framed group of fields in a dialog.
pub fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(title).strong());
    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
    ui.add_space(10.0);
}

/// Secondary text on one line, cut with an ellipsis when the column is narrow.
pub fn dim(ui: &mut egui::Ui, text: String) {
    ui.add(egui::Label::new(egui::RichText::new(text).weak()).truncate());
}
