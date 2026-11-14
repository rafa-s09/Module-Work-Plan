//! The task board: columns, cards and the task lifecycle steps (start, release, close).

use super::*;

pub const COLUMNS: [(&str, &str); 5] = [("open", "Open"), ("in_progress", "In progress"), ("blocked", "Blocked"), ("review", "Review"), ("done", "Done")];

/// A task lifecycle step asked from the file header or a card; the task by ID.
#[derive(Debug, PartialEq)]
pub enum TaskCmd {
    Start(String),
    Close(String),
    Release(String, &'static str),
}

/// Where a task not done can be moved by hand (`task release`): its locks are freed.
pub const RELEASE: [(&str, &str); 3] = [("open", "Back to open"), ("blocked", "Blocked"), ("review", "Ready for review")];

/// The status changes for a task, as menu items.
pub fn task_status_items(ui: &mut egui::Ui, id: &str, status: &str, cmd: &mut Option<TaskCmd>) {
    for (s, label) in RELEASE.into_iter().filter(|r| r.0 != status) {
        let (icon, color) = status_icon(ui, s);
        let r = ui.button((icon.rich_text().color(color), tr(label)));
        if r.on_hover_text(tr("Frees the task's claim and module locks")).clicked() {
            *cmd = Some(TaskCmd::Release(id.to_string(), s));
            ui.close();
        }
    }
}

/// Start / Status / Close in the header of an open task (laid out right to left).
pub fn task_buttons(ui: &mut egui::Ui, t: &Doc, cmd: &mut Option<TaskCmd>) {
    let (id, status) = (t.id(), t.get("status"));
    if id.is_empty() || status == "done" {
        return;
    }
    if matches!(status, "in_progress" | "review") && ui.button(ic(ICON_DONE_ALL, tr("Close"))).on_hover_text(tr("Set done and move to tasks/done/; ## Closure must be filled")).clicked() {
        *cmd = Some(TaskCmd::Close(id.to_string()));
    }
    ui.menu_button(ic(ICON_SWAP_HORIZ, tr("Status")), |ui| task_status_items(ui, id, status, cmd));
    if status != "in_progress" {
        let start = egui::Button::new(ic(ICON_PLAY_ARROW, tr("Start"))).fill(ui.visuals().selection.bg_fill);
        if ui.add(start).on_hover_text(tr("Claim the task for an agent and lock its modules")).clicked() {
            *cmd = Some(TaskCmd::Start(id.to_string()));
        }
    }
}

/// The same steps in a card's context menu.
pub fn task_menu(ui: &mut egui::Ui, t: &Doc, cmd: &mut Option<TaskCmd>) {
    let (id, status) = (t.id(), t.get("status"));
    if id.is_empty() || status == "done" {
        return;
    }
    if status != "in_progress" && ui.button(ic(ICON_PLAY_ARROW, tr("Start…"))).clicked() {
        *cmd = Some(TaskCmd::Start(id.to_string()));
        ui.close();
    }
    if matches!(status, "in_progress" | "review") && ui.button(ic(ICON_DONE_ALL, tr("Close"))).clicked() {
        *cmd = Some(TaskCmd::Close(id.to_string()));
        ui.close();
    }
    task_status_items(ui, id, status, cmd);
}

/// Accept / Reject in the header of a proposed ADR (laid out right to left). Only a person
/// decides an ADR, so the CLI agents use has no such command.
pub fn adr_buttons(ui: &mut egui::Ui, adr: &Doc, decide: &mut Option<(String, &'static str)>) {
    if adr.get("status") != "proposed" {
        return;
    }
    let accept = egui::Button::new(ic(ICON_CHECK_CIRCLE, tr("Accept"))).fill(ui.visuals().selection.bg_fill);
    if ui.add(accept).on_hover_text(tr("Set status: accepted. The decision is in force from now on")).clicked() {
        *decide = Some((adr.id().to_string(), "accepted"));
    }
    if ui.button(ic(ICON_CANCEL, tr("Reject"))).on_hover_text(tr("Set status: rejected")).clicked() {
        *decide = Some((adr.id().to_string(), "rejected"));
    }
}

/// The Context button of a task's header: what an agent loads for the task, like `mwp context`,
/// with a token estimate and the cited IDs that do not exist. A file opens on click.
pub fn context_button(ui: &mut egui::Ui, p: &Project, t: &Doc, ask: &mut Asks) {
    let c = ops::context(p, t);
    let label = if c.missing.is_empty() { ic(ICON_LIBRARY_BOOKS, tr("Context")) } else { ic(ICON_WARNING, tr("Context")) };
    let r = ui.menu_button(label, |ui| {
        ui.strong(trf("What an agent loads for {id}", &[("id", t.id())]));
        ui.weak(tr("Like mwp context: the constitution, the task, then what its front matter cites."));
        ui.separator();
        egui::Grid::new(("context", t.id())).num_columns(3).spacing([16.0, 4.0]).show(ui, |ui| {
            for (why, path, tokens) in &c.files {
                let (icon, color) = file_icon(ui, p, path);
                if ui.selectable_label(false, (icon.rich_text().color(color), p.rel(path))).on_hover_text(tr("Open file")).clicked() {
                    ask.action = Some(Action::Open(path.clone()));
                    ui.close();
                }
                ui.weak(*why);
                ui.label(RichText::new(tokens.to_string()).monospace());
                ui.end_row();
            }
            ui.strong(tr("Total"));
            ui.weak(tr("tokens (estimate)"));
            ui.strong(RichText::new(c.tokens().to_string()).monospace());
            ui.end_row();
        });
        for m in &c.missing {
            ui.colored_label(ui.visuals().error_fg_color, format!("{} {}", ICON_ERROR.codepoint, trf("Missing: {item}", &[("item", m)])));
        }
    });
    r.response.on_hover_text(tr(if c.missing.is_empty() { "What an agent loads for this task" } else { "The task cites IDs that do not exist" }));
}

/// A task card: ID, size, title, then its modules, agent and work codes. The whole card is clickable.
pub fn card(ui: &mut egui::Ui, t: &Doc) -> egui::Response {
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
    let r = ui.interact(rect, ui.scope_id().with(("card", t.id())), egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if r.hovered() {
        ui.painter().rect_stroke(rect, 6.0, ui.visuals().widgets.hovered.bg_stroke, egui::StrokeKind::Inside);
    }
    r
}

/// The step a card dropped from the `from` column on the `to` column asks for: the same ones its
/// menu offers (Start, Close for a task in progress or in review, a release). None for the rest.
pub fn drop_cmd(id: &str, from: &str, to: &str) -> Option<TaskCmd> {
    match to {
        _ if from == to || from == "done" => None,
        "in_progress" => Some(TaskCmd::Start(id.into())),
        "done" => matches!(from, "in_progress" | "review").then(|| TaskCmd::Close(id.into())),
        _ => RELEASE.iter().find(|r| r.0 == to).map(|r| TaskCmd::Release(id.into(), r.0)),
    }
}

/// The board's filters; an empty field keeps every card.
#[derive(Default)]
pub struct BoardFilter {
    /// Part of the title, the ID or a work code, in any case.
    pub text: String,
    pub module: String,
    pub agent: String,
    pub size: String,
}

impl BoardFilter {
    pub fn keeps(&self, t: &Doc) -> bool {
        let text = self.text.trim().to_lowercase();
        let has = |s: &str| s.to_lowercase().contains(&text);
        (text.is_empty() || has(t.title()) || has(t.id()) || t.list("external").iter().any(|e| has(e)))
            && (self.module.is_empty() || t.list("modules").contains(&self.module))
            && (self.agent.is_empty() || t.get("agent") == self.agent)
            && (self.size.is_empty() || t.get("size") == self.size)
    }

    /// The filter as it is kept between runs (see `layout`): text, module, agent, size.
    pub fn fields(&self) -> [String; 4] {
        [self.text.clone(), self.module.clone(), self.agent.clone(), self.size.clone()]
    }

    /// A kept filter back, without a module, agent or size the project no longer has: those
    /// would hide every card with nothing in the menus to say why.
    pub fn from_fields([text, module, agent, size]: [String; 4], p: &Project) -> BoardFilter {
        let module = if p.by_id("module", &module).is_some() { module } else { String::new() };
        let agent = if p.of_kind("task").any(|t| t.get("agent") == agent) { agent } else { String::new() };
        let size = if SIZES.iter().any(|s| s.0 == size) { size } else { String::new() };
        BoardFilter { text, module, agent, size }
    }

    fn active(&self) -> bool {
        !(self.text.trim().is_empty() && self.module.is_empty() && self.agent.is_empty() && self.size.is_empty())
    }

    /// The filter row: text, then a menu each for module, agent and size, and Clear.
    fn show(&mut self, ui: &mut egui::Ui, p: &Project) {
        ui.horizontal(|ui| {
            ui.label(ICON_FILTER_LIST.rich_text().color(ui.visuals().weak_text_color()));
            ui.add(egui::TextEdit::singleline(&mut self.text).hint_text(tr("Title, ID or work code")).desired_width(220.0));
            let mut modules: Vec<String> = p.of_kind("module").map(|m| m.id().to_string()).collect();
            modules.sort();
            let mut agents: Vec<String> = p.of_kind("task").map(|t| t.get("agent").to_string()).filter(|a| !a.is_empty()).collect();
            agents.sort();
            agents.dedup();
            let sizes: Vec<String> = SIZES.iter().map(|s| s.0.to_string()).collect();
            for (value, all, options) in [(&mut self.module, "All modules", modules), (&mut self.agent, "All agents", agents), (&mut self.size, "All sizes", sizes)] {
                let shown = if value.is_empty() { tr(all).to_string() } else { value.clone() };
                let r = egui::ComboBox::from_id_salt(all).selected_text(&shown).show_ui(ui, |ui| {
                    ui.selectable_value(value, String::new(), tr(all));
                    for o in options {
                        ui.selectable_value(value, o.clone(), o);
                    }
                });
                // Named for screen readers by what it shows ("All modules", or the module).
                r.response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::ComboBox, true, &shown));
            }
            if self.active() && ui.button(ic(ICON_CLOSE, tr("Clear"))).clicked() {
                *self = BoardFilter::default();
            }
        });
    }
}

/// The board: one column per status, a card per task. A card dragged to another column changes
/// the task's status; the column it can go to is outlined while it is dragged.
pub fn board(ui: &mut egui::Ui, v: &View, show_done: &mut bool, filter: &mut BoardFilter, ask: &mut Asks) {
    ui.horizontal(|ui| {
        ui.label(ICON_VIEW_KANBAN.rich_text().size(24.0).color(ui.visuals().hyperlink_color));
        ui.heading(tr("Board"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ask.new_task |= ui.add(egui::Button::new(ic(ICON_ADD_TASK, tr("New task"))).fill(ui.visuals().selection.bg_fill)).clicked();
            ui.checkbox(show_done, tr("Show done"));
        });
    });
    filter.show(ui, &v.project);
    ui.add_space(6.0);
    let columns: Vec<_> = COLUMNS.iter().filter(|c| c.0 != "done" || *show_done).collect();
    let (mut rects, mut dragged, mut dropped) = (Vec::new(), None, None);
    ui.columns(columns.len(), |cols| {
        for (ui, (status, label)) in cols.iter_mut().zip(columns) {
            rects.push((*status, ui.max_rect()));
            let mut tasks: Vec<&Doc> = v.project.of_kind("task").filter(|t| t.get("status") == *status && filter.keeps(t)).collect();
            tasks.sort_by_key(|t| id_num(t.id()));
            ui.horizontal(|ui| {
                let (icon, color) = status_icon(ui, status);
                ui.label(icon.rich_text().color(color));
                ui.strong(tr(label));
                ui.weak(tasks.len().to_string());
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt(*status).show(ui, |ui| {
                for t in tasks {
                    let r = card(ui, t);
                    if r.clicked() {
                        ask.action = Some(Action::Open(t.path.clone()));
                    }
                    if r.dragged() {
                        dragged = Some(t);
                    }
                    if r.drag_stopped() {
                        dropped = Some(t);
                    }
                    r.context_menu(|ui| {
                        if ui.button(ic(ICON_EDIT, tr("Open task"))).clicked() {
                            ask.action = Some(Action::Open(t.path.clone()));
                            ui.close();
                        }
                        task_menu(ui, t, &mut ask.task_cmd);
                        if ops::deletable(&v.project, &t.path) && ui.button(ic(ICON_DELETE, tr("Delete…"))).clicked() {
                            ask.ask_delete = Some(t.path.clone());
                            ui.close();
                        }
                    });
                }
            });
        }
    });
    let Some(pointer) = ui.ctx().pointer_latest_pos() else { return };
    let target = |t: &Doc| rects.iter().find(|r| r.1.contains(pointer)).and_then(|r| drop_cmd(t.id(), t.get("status"), r.0).map(|cmd| (r.1, cmd)));
    if let Some(t) = dragged {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some((rect, _)) = target(t) {
            ui.painter().rect_stroke(rect.expand(2.0), 6.0, egui::Stroke::new(2.0, ui.visuals().selection.stroke.color), egui::StrokeKind::Outside);
        }
        // The card follows the pointer.
        egui::Area::new(egui::Id::unique("dragged-card")).order(egui::Order::Tooltip).interactable(false).fixed_pos(pointer + egui::vec2(12.0, 8.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.strong(t.id());
                ui.label(t.title());
            });
        });
    }
    if let Some(t) = dropped {
        ask.task_cmd = target(t).map(|(_, cmd)| cmd);
    }
}

impl App {
    /// Start asks for the agent; Close and the status changes run at once. A task with unsaved
    /// edits is left alone: the step rewrites (or moves) its file.
    pub fn run_task_cmd(&mut self, cmd: TaskCmd) {
        let (TaskCmd::Start(id) | TaskCmd::Close(id) | TaskCmd::Release(id, _)) = &cmd;
        let Some((path, modules)) = self.view.as_ref().ok().and_then(|v| v.project.by_id("task", id)).map(|t| (t.path.clone(), t.list("modules"))) else { return };
        let notice = |title: String, text: String| Some(Dialog::Notice { title, text });
        if self.tabs.iter().any(|e| e.path == path && e.dirty()) {
            self.dialog = notice(trf("Save {id} first", &[("id", id)]), tr("The task has unsaved changes. Save them before changing its status.").into());
            return;
        }
        let project = Project::open(&self.root);
        match cmd {
            TaskCmd::Start(id) => {
                let agents: Vec<String> = scaffold::installed(&self.root).iter().map(|a| a.id.to_string()).collect();
                let agent = std::env::var("MWP_AGENT").ok().filter(|a| !a.trim().is_empty()).or_else(|| agents.first().cloned()).unwrap_or_else(|| "claude".into());
                self.dialog = Some(Dialog::StartTask { id, agent, agents, modules, error: String::new() });
            }
            TaskCmd::Close(id) => match project.and_then(|p| ops::task_close(&p, &id)) {
                Ok((lines, report)) => {
                    // The task moved to tasks/done/ (`close T-0001 -> mwp/tasks/done/...`): its tab follows it.
                    let dest = lines.iter().find_map(|l| l.split_once(" -> ")).map(|(_, rel)| self.root.join(rel));
                    if let (Some(dest), Some(tab)) = (dest, self.tabs.iter_mut().find(|e| e.path == path)) {
                        if let Ok(ed) = Editor::open(&dest) {
                            *tab = ed;
                        }
                    }
                    if !report.errors.is_empty() {
                        self.message = Some(trf("{id} closed; lint found {n} error(s)", &[("id", &id), ("n", &report.errors.len().to_string())]));
                    }
                    self.reload();
                }
                Err(e) => self.dialog = notice(trf("Cannot close {id}", &[("id", &id)]), e),
            },
            TaskCmd::Release(id, status) => match project.and_then(|p| ops::task_release(&p, &id, status)) {
                Ok(_) => self.reload(),
                Err(e) => self.dialog = notice(trf("Cannot change {id}", &[("id", &id)]), e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_card_asks_what_its_menu_offers() {
        assert_eq!(drop_cmd("T-1", "open", "in_progress"), Some(TaskCmd::Start("T-1".into())));
        assert_eq!(drop_cmd("T-1", "in_progress", "blocked"), Some(TaskCmd::Release("T-1".into(), "blocked")));
        assert_eq!(drop_cmd("T-1", "review", "done"), Some(TaskCmd::Close("T-1".into())));
        assert_eq!(drop_cmd("T-1", "open", "done"), None, "only a task in progress or in review closes");
        assert_eq!(drop_cmd("T-1", "open", "open"), None);
        assert_eq!(drop_cmd("T-1", "done", "open"), None, "a done task never moves");
    }

    #[test]
    fn board_filter_keeps_matching_cards() {
        let f = std::env::temp_dir().join(format!("mwp-filter-{}.md", std::process::id()));
        fs::write(&f, "---\nkind: task\nid: T-0007\nstatus: in_progress\nsize: M\nmodules: [order, cart]\nagent: codex\nexternal: [PROJ-12]\n---\n# T-0007 — Checkout total\n").unwrap();
        let t = Doc::load(&f).unwrap();
        let _ = fs::remove_file(&f);
        let filter = |text: &str, module: &str, agent: &str, size: &str| BoardFilter { text: text.into(), module: module.into(), agent: agent.into(), size: size.into() };
        assert!(filter("", "", "", "").keeps(&t) && !filter("", "", "", "").active());
        for text in ["checkout", "t-0007", "proj-12"] {
            assert!(filter(text, "", "", "").keeps(&t), "{text}");
        }
        assert!(!filter("login", "", "", "").keeps(&t));
        assert!(filter("", "cart", "codex", "M").keeps(&t));
        assert!(!filter("", "billing", "", "").keeps(&t) && !filter("", "", "claude", "").keeps(&t) && !filter("", "", "", "S").keeps(&t));
    }

    #[test]
    fn kept_filter_drops_what_the_project_lost() {
        let root = std::env::temp_dir().join(format!("mwp-kept-filter-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        scaffold::init(&root, Some("Demo"), &[], false).unwrap();
        fs::write(root.join("mwp/modules/order.md"), "---\nkind: module\nid: order\ntype: domain\nsummary: Orders\nupdated: 2026-01-01\n---\n# order\n").unwrap();
        fs::write(root.join("mwp/tasks/open/T-0001-a.md"), "---\nkind: task\nid: T-0001\nstatus: in_progress\nsize: S\nmodules: [order]\nagent: codex\n---\n# T-0001 — A\n").unwrap();
        let p = Project::open(&root).unwrap();
        let f = BoardFilter::from_fields(["a".into(), "order".into(), "codex".into(), "S".into()], &p);
        assert_eq!(f.fields(), ["a", "order", "codex", "S"]);
        let f = BoardFilter::from_fields(["a".into(), "billing".into(), "claude".into(), "XL".into()], &p);
        assert_eq!(f.fields(), ["a", "", "", ""], "a module, agent or size no longer there is dropped");
        let _ = fs::remove_dir_all(&root);
    }
}
