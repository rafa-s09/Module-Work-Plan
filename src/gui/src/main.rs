//! `mwp-gui`: a read-only board for an MWP project. Shows the tasks by status, which agent
//! holds each one, the locked modules and the lint result. All the logic is in `mwp_core`.

use eframe::egui;
use mwp_core::doc::{id_num, Doc, Project};
use mwp_core::lint::{lint, Report};
use mwp_core::state::{self, Claim};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// shortcut: the project is reread on a timer, not on file events; add a file watcher
// if rereading every 2 s gets slow on a very large project.
const REFRESH: Duration = Duration::from_secs(2);

const COLUMNS: [(&str, &str); 5] = [("open", "Open"), ("in_progress", "In progress"), ("blocked", "Blocked"), ("review", "Review"), ("done", "Done")];

/// One read of the project from disk.
struct View {
    project: Project,
    report: Report,
    claims: Vec<Claim>,
    locks: Vec<(String, Claim)>,
}

fn load(root: &PathBuf) -> Result<View, String> {
    let project = Project::open(root)?;
    Ok(View { report: lint(&project), claims: state::claims(&project.state), locks: state::locks(&project.state), project })
}

struct Board {
    root: PathBuf,
    view: Result<View, String>,
    loaded: Instant,
    selected: Option<String>,
    show_done: bool,
    show_lint: bool,
}

impl Board {
    fn new(root: PathBuf) -> Board {
        Board { view: load(&root), root, loaded: Instant::now(), selected: None, show_done: false, show_lint: false }
    }

    fn reload(&mut self) {
        self.view = load(&self.root);
        self.loaded = Instant::now();
    }
}

/// Secondary text on one line, cut with an ellipsis when the column is narrow.
fn dim(ui: &mut egui::Ui, text: String) {
    ui.add(egui::Label::new(egui::RichText::new(text).weak()).truncate());
}

/// A task card; returns true when clicked.
fn card(ui: &mut egui::Ui, t: &Doc, selected: bool) -> bool {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let clicked = ui.selectable_label(selected, egui::RichText::new(format!("{}  ·  {}", t.id(), t.get("size"))).strong()).clicked();
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

impl eframe::App for Board {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.loaded.elapsed() >= REFRESH {
            self.reload();
        }
        ui.ctx().request_repaint_after(REFRESH);

        let mut reload = false;
        let mut clicked: Option<String> = None;

        egui::Panel::top("bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                match &self.view {
                    Ok(v) => {
                        let name = v.project.of_kind("project").next().map_or("", |d| d.get("name"));
                        ui.heading(if name.is_empty() { "MWP" } else { name });
                        let (e, w) = (v.report.errors.len(), v.report.warnings.len());
                        let text = egui::RichText::new(format!("lint: {e} error(s), {w} warning(s)"));
                        let text = if e > 0 { text.color(ui.visuals().error_fg_color) } else if w > 0 { text.color(ui.visuals().warn_fg_color) } else { text };
                        ui.toggle_value(&mut self.show_lint, text);
                    }
                    Err(_) => {
                        ui.heading("MWP");
                    }
                }
                ui.checkbox(&mut self.show_done, "Show done");
                reload = ui.button("Reload").clicked();
                // Last, so a long path is cut instead of pushing the controls out of the window.
                dim(ui, self.root.display().to_string());
            });
        });

        let Ok(v) = &self.view else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, self.view.as_ref().err().map_or("", String::as_str));
                ui.label("Start it inside a project (a folder with mwp/), or pass the folder: mwp-gui <project-dir>");
            });
            if reload {
                self.reload();
            }
            return;
        };

        if self.show_lint {
            egui::Panel::bottom("lint").resizable(true).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("lint").show(ui, |ui| {
                    for e in &v.report.errors {
                        ui.colored_label(ui.visuals().error_fg_color, format!("ERROR {e}"));
                    }
                    for w in &v.report.warnings {
                        ui.colored_label(ui.visuals().warn_fg_color, format!("WARN  {w}"));
                    }
                    if v.report.errors.is_empty() && v.report.warnings.is_empty() {
                        ui.label("No errors or warnings.");
                    }
                });
            });
        }

        egui::Panel::left("agents").resizable(true).default_size(190.0).show(ui, |ui| {
            ui.strong("Agents at work");
            ui.separator();
            for c in &v.claims {
                let modules: Vec<&str> = v.locks.iter().filter(|l| l.1.task == c.task).map(|l| l.0.as_str()).collect();
                if ui.selectable_label(self.selected.as_deref() == Some(&c.task), format!("{}  ·  {}", c.task, c.agent)).clicked() {
                    clicked = Some(c.task.clone());
                }
                dim(ui, if modules.is_empty() { "no module locked".to_string() } else { format!("locks: {}", modules.join(", ")) });
                ui.add_space(6.0);
            }
            if v.claims.is_empty() {
                ui.weak("No task in progress.");
            }
        });

        let selected = self.selected.as_ref().and_then(|id| v.project.by_id("task", id));
        if let Some(t) = selected {
            let mut close = false;
            egui::Panel::right("task").resizable(true).default_size(420.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(v.project.rel(&t.path));
                    close = ui.button("Close").clicked();
                });
                ui.separator();
                egui::ScrollArea::vertical().id_salt("task").show(ui, |ui| {
                    ui.label(&t.body);
                });
            });
            if close {
                clicked = Some(String::new());
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let columns: Vec<_> = COLUMNS.iter().filter(|c| c.0 != "done" || self.show_done).collect();
            ui.columns(columns.len(), |cols| {
                for (ui, (status, label)) in cols.iter_mut().zip(columns) {
                    let mut tasks: Vec<&Doc> = v.project.of_kind("task").filter(|t| t.get("status") == *status).collect();
                    tasks.sort_by_key(|t| id_num(t.id()));
                    ui.strong(format!("{label} ({})", tasks.len()));
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt(*status).show(ui, |ui| {
                        for t in tasks {
                            if card(ui, t, self.selected.as_deref() == Some(t.id())) {
                                clicked = Some(t.id().to_string());
                            }
                        }
                    });
                }
            });
        });

        if let Some(id) = clicked {
            // Clicking the selected card again, or Close, hides the task panel.
            self.selected = if id.is_empty() || self.selected.as_deref() == Some(&id) { None } else { Some(id) };
        }
        if reload {
            self.reload();
        }
    }
}

fn main() -> eframe::Result {
    // The project is the folder given, or the nearest one with mwp/ from here upward.
    let root = match std::env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => Project::find().map(|p| p.root).unwrap_or_else(|_| std::env::current_dir().unwrap_or_default()),
    };
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 720.0]), ..Default::default() };
    eframe::run_native("MWP", options, Box::new(|_cc| Ok(Box::new(Board::new(root)))))
}
