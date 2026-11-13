//! The editor tab strip.

use super::*;

pub const TAB_HEIGHT: f32 = 34.0;

/// One editor tab, drawn like VS Code: flat, the open one in the editor colour with an accent
/// on top, the close button inside (shown on hover or when open, a dot while unsaved).
/// Returns the tab's response (clicked, dragged, its menu) and whether its close button was clicked.
/// The id comes from `path`, not the position, so a drag goes on while the tab moves.
pub fn tab(ui: &mut egui::Ui, active: bool, (icon, tint): (MaterialIcon, Color32), name: &str, dirty: bool, path: &str) -> (egui::Response, bool) {
    let v = ui.visuals().clone();
    let (pad, gap, close_w) = (12.0, 6.0, 20.0);
    let text = if active { v.strong_text_color() } else { v.weak_text_color() };
    let icon_g = ui.painter().layout_no_wrap(icon.codepoint.to_string(), egui::FontId::new(16.0, icon.font_family()), tint);
    let name_g = ui.painter().layout_no_wrap(name.to_string(), egui::TextStyle::Body.resolve(ui.style()), text);
    let width = pad + icon_g.size().x + gap + name_g.size().x + gap + close_w + pad / 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, TAB_HEIGHT), egui::Sense::hover());
    let r = ui.interact(rect, ui.make_persistent_id(("tab", path)), egui::Sense::click_and_drag());
    r.widget_info(|| egui::WidgetInfo::selected(egui::accesskit::Role::Tab, true, active, name));
    let hovered = ui.rect_contains_pointer(rect);
    let close_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - pad / 2.0 - close_w / 2.0, rect.center().y), egui::vec2(close_w, close_w));
    {
        let p = ui.painter();
        let fill = if active { v.panel_fill } else if hovered { v.widgets.hovered.weak_bg_fill } else { Color32::TRANSPARENT };
        p.rect_filled(rect, 0.0, fill);
        if active {
            p.hline(rect.x_range(), rect.top() + 1.0, egui::Stroke::new(2.0, v.selection.stroke.color));
        }
        p.vline(rect.right() - 0.5, rect.y_range(), v.widgets.noninteractive.bg_stroke);
        let x = rect.left() + pad;
        p.galley(egui::pos2(x, rect.center().y - icon_g.size().y / 2.0), icon_g.clone(), tint);
        p.galley(egui::pos2(x + icon_g.size().x + gap, rect.center().y - name_g.size().y / 2.0), name_g, text);
        if dirty && !hovered {
            p.circle_filled(close_rect.center(), 4.0, text);
        }
    }
    let shown = (active || hovered) && !(dirty && !hovered);
    let close = egui::Button::new(ICON_CLOSE.rich_text().size(16.0).color(if shown { text } else { Color32::TRANSPARENT })).frame(false);
    let close = ui.put(close_rect, close).on_hover_text("Close").clicked();
    if r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    (r.on_hover_text(path), close)
}

/// The open files as tabs: click to show, drag or right-click to move, close.
pub fn tab_strip(ui: &mut egui::Ui, v: &View, tabs: &[Editor], active: Option<usize>, shown: Option<&PathBuf>, ask: &mut Asks) {
    if !tabs.is_empty() {
        let strip = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), TAB_HEIGHT));
        let vis = ui.visuals();
        ui.painter().rect_filled(strip, 0.0, vis.panel_fill.lerp_to_gamma(vis.text_color(), 0.05));
        ui.painter().hline(strip.x_range(), strip.bottom() - 0.5, vis.widgets.noninteractive.bg_stroke);
        egui::ScrollArea::horizontal().id_salt("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (mut rects, mut dragging) = (Vec::new(), None);
                let last = tabs.len() - 1;
                for (i, t) in tabs.iter().enumerate() {
                    let icon = file_icon(ui, &v.project, &t.path);
                    let (r, close) = tab(ui, active == Some(i), icon, &t.name(), t.dirty(), &v.project.rel(&t.path));
                    if r.clicked() {
                        ask.action = Some(Action::Open(t.path.clone()));
                    }
                    if close {
                        ask.action = Some(Action::CloseTab(i));
                    }
                    if r.dragged() {
                        dragging = ui.ctx().pointer_interact_pos().map(|p| (i, p.x));
                    }
                    // A tab that just became the open one is scrolled into view.
                    if active == Some(i) && shown != Some(&t.path) {
                        r.scroll_to_me(Some(egui::Align::Center));
                        ask.shown_tab = Some(t.path.clone());
                    }
                    r.context_menu(|ui| {
                        if ui.add_enabled(i > 0, egui::Button::new(ic(ICON_ARROW_BACK, "Move left"))).clicked() {
                            ask.action = Some(Action::MoveTab(i, i - 1));
                            ui.close();
                        }
                        if ui.add_enabled(i < last, egui::Button::new(ic(ICON_ARROW_FORWARD, "Move right"))).clicked() {
                            ask.action = Some(Action::MoveTab(i, i + 1));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(ic(ICON_CLOSE, "Close")).clicked() {
                            ask.action = Some(Action::CloseTab(i));
                            ui.close();
                        }
                    });
                    rects.push(r.rect);
                }
                // Dragging a tab over another one puts it there, while the drag goes on.
                if let Some((i, x)) = dragging {
                    let to = rects.iter().position(|r| r.x_range().contains(x)).unwrap_or(if x < rects[0].left() { 0 } else { last });
                    if to != i {
                        ask.action = Some(Action::MoveTab(i, to));
                    }
                }
            });
        });
        ui.add_space(6.0);
    }
}
