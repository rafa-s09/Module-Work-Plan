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
        let r = ui.button((icon.rich_text().color(color), label));
        if r.on_hover_text("Frees the task's claim and module locks").clicked() {
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
    if matches!(status, "in_progress" | "review") && ui.button(ic(ICON_DONE_ALL, "Close")).on_hover_text("Set done and move to tasks/done/; ## Closure must be filled").clicked() {
        *cmd = Some(TaskCmd::Close(id.to_string()));
    }
    ui.menu_button(ic(ICON_SWAP_HORIZ, "Status"), |ui| task_status_items(ui, id, status, cmd));
    if status != "in_progress" {
        let start = egui::Button::new(ic(ICON_PLAY_ARROW, "Start")).fill(ui.visuals().selection.bg_fill);
        if ui.add(start).on_hover_text("Claim the task for an agent and lock its modules").clicked() {
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
    if status != "in_progress" && ui.button(ic(ICON_PLAY_ARROW, "Start…")).clicked() {
        *cmd = Some(TaskCmd::Start(id.to_string()));
        ui.close();
    }
    if matches!(status, "in_progress" | "review") && ui.button(ic(ICON_DONE_ALL, "Close")).clicked() {
        *cmd = Some(TaskCmd::Close(id.to_string()));
        ui.close();
    }
    task_status_items(ui, id, status, cmd);
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

/// The board: one column per status, a card per task. A card dragged to another column changes
/// the task's status; the column it can go to is outlined while it is dragged.
pub fn board(ui: &mut egui::Ui, v: &View, show_done: &mut bool, ask: &mut Asks) {
    ui.horizontal(|ui| {
        ui.label(ICON_VIEW_KANBAN.rich_text().size(24.0).color(ui.visuals().hyperlink_color));
        ui.heading("Board");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ask.new_task |= ui.add(egui::Button::new(ic(ICON_ADD_TASK, "New task")).fill(ui.visuals().selection.bg_fill)).clicked();
            ui.checkbox(show_done, "Show done");
        });
    });
    ui.add_space(6.0);
    let columns: Vec<_> = COLUMNS.iter().filter(|c| c.0 != "done" || *show_done).collect();
    let (mut rects, mut dragged, mut dropped) = (Vec::new(), None, None);
    ui.columns(columns.len(), |cols| {
        for (ui, (status, label)) in cols.iter_mut().zip(columns) {
            rects.push((*status, ui.max_rect()));
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
                        ask.action = Some(Action::Open(t.path.clone()));
                    }
                    if r.dragged() {
                        dragged = Some(t);
                    }
                    if r.drag_stopped() {
                        dropped = Some(t);
                    }
                    r.context_menu(|ui| {
                        if ui.button(ic(ICON_EDIT, "Open")).clicked() {
                            ask.action = Some(Action::Open(t.path.clone()));
                            ui.close();
                        }
                        task_menu(ui, t, &mut ask.task_cmd);
                        if ops::deletable(&v.project, &t.path) && ui.button(ic(ICON_DELETE, "Delete…")).clicked() {
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
            self.dialog = notice(format!("Save {id} first"), "The task has unsaved changes. Save them before changing its status.".into());
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
                        self.message = Some(format!("{id} closed; lint found {} error(s)", report.errors.len()));
                    }
                    self.reload();
                }
                Err(e) => self.dialog = notice(format!("Cannot close {id}"), e),
            },
            TaskCmd::Release(id, status) => match project.and_then(|p| ops::task_release(&p, &id, status)) {
                Ok(_) => self.reload(),
                Err(e) => self.dialog = notice(format!("Cannot change {id}"), e),
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
}
