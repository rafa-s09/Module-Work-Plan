//! The Markdown editor: one open file, its source view (highlighted, with line numbers and a
//! formatting toolbar) and its rendered preview.

use eframe::egui::{self, text::CCursor, text::LayoutJob, text_selection::CCursorRange, Color32, FontId, TextFormat};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_material_icons::{icons::*, MaterialIcon};
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
}

impl Editor {
    pub fn open(path: &Path) -> Result<Editor, String> {
        let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = raw.replace("\r\n", "\n");
        Ok(Editor { path: path.to_path_buf(), saved: text.clone(), text, crlf: raw.contains("\r\n"), cursor: None, cache: CommonMarkCache::default() })
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn name(&self) -> String {
        self.path.file_name().unwrap().to_string_lossy().into_owned()
    }

    pub fn save(&mut self) -> Result<(), String> {
        let text = if self.crlf { self.text.replace('\n', "\r\n") } else { self.text.clone() };
        fs::write(&self.path, text).map_err(|e| format!("cannot save {}: {e}", self.path.display()))?;
        self.saved = self.text.clone();
        Ok(())
    }

    /// Follows the file on disk (an agent may have changed it) unless there are edits to keep.
    pub fn refresh(&mut self) {
        if !self.dirty() {
            if let Ok(raw) = fs::read_to_string(&self.path) {
                let text = raw.replace("\r\n", "\n");
                if text != self.text {
                    (self.text, self.saved, self.crlf) = (text.clone(), text, raw.contains("\r\n"));
                }
            }
        }
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

/// Toolbar, source and preview for the open file.
pub fn show(ui: &mut egui::Ui, ed: &mut Editor, mode: &mut Mode) {
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
                if ui.add(button).on_hover_text(tip).clicked() {
                    change = Some(f(ed));
                }
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (m, icon, label) in [(Mode::Preview, ICON_VISIBILITY, "Preview"), (Mode::Split, ICON_VERTICAL_SPLIT, "Split"), (Mode::Edit, ICON_CODE, "Source")] {
                ui.selectable_value(mode, m, format!("{} {label}", icon.codepoint));
            }
        });
    });
    ui.separator();

    // Status of the source: position, words, line endings.
    let line_info = {
        let (a, _) = sel(ed);
        let before: String = ed.text.chars().take(a).collect();
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        format!("Ln {line}, Col {col}   ·   {} words   ·   {}   ·   Markdown", ed.text.split_whitespace().count(), if ed.crlf { "CRLF" } else { "LF" })
    };
    // The views get the height minus one status row, so the scrolled text never runs under it.
    let id = ed.id();
    let status_h = ui.spacing().interact_size.y;
    let body = egui::vec2(ui.available_width(), (ui.available_height() - status_h - ui.spacing().item_spacing.y).max(0.0));
    ui.allocate_ui(body, |ui| match *mode {
        Mode::Edit => source(ui, ed, id),
        Mode::Preview => preview(ui, ed),
        Mode::Split => {
            ui.columns(2, |cols| {
                source(&mut cols[0], ed, id);
                preview(&mut cols[1], ed);
            });
        }
    });
    // Align::Min: a centred right-to-left layout would take all the height it is offered.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| ui.weak(line_info));

    if let Some(range) = change {
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(ui.ctx(), id);
        ui.ctx().memory_mut(|m| m.request_focus(id));
        ed.cursor = Some(range);
    }
}

fn source(ui: &mut egui::Ui, ed: &mut Editor, id: egui::Id) {
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
    });
}

fn preview(ui: &mut egui::Ui, ed: &mut Editor) {
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
                                ui.label(v);
                                ui.end_row();
                            }
                        }
                    });
                });
                ui.add_space(8.0);
            }
            CommonMarkViewer::new().show(ui, &mut ed.cache, body);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(text: &str, a: usize, b: usize) -> Editor {
        Editor { path: "x.md".into(), text: text.into(), saved: String::new(), crlf: false, cursor: Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))), cache: CommonMarkCache::default() }
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
    fn highlight_keeps_every_character() {
        let text = "---\nkind: task\n---\n# T — Title\n- [ ] item with `code` and <placeholder>\n```\nfn x() {}\n```\n";
        let p = palette(&egui::Visuals::dark());
        assert_eq!(highlight(text, FontId::monospace(12.0), &p).text, text);
    }
}
