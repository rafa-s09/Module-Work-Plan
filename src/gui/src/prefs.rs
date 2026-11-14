//! Appearance preferences: theme, text size and text font. Kept with the window state
//! (`layout`) and applied when the GUI starts and whenever they change.

use super::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    /// Follow the system's light or dark setting.
    System,
    Dark,
    Light,
}

/// The themes, as stored and as shown.
pub const THEMES: [(Theme, &str, &str); 3] = [(Theme::System, "system", "Follow the system"), (Theme::Dark, "dark", "Dark"), (Theme::Light, "light", "Light")];

impl Theme {
    pub fn code(self) -> &'static str {
        THEMES.iter().find(|t| t.0 == self).map_or("system", |t| t.1)
    }
    pub fn from_code(code: &str) -> Theme {
        THEMES.iter().find(|t| t.1 == code).map_or(Theme::System, |t| t.0)
    }
}

/// The text sizes offered, as a share of egui's own.
pub const SCALE_MIN: f32 = 0.8;
pub const SCALE_MAX: f32 = 1.6;

pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    ctx.set_theme(match theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });
}

/// Every text style at `scale` times its default size, in both themes.
pub fn apply_scale(ctx: &egui::Context, scale: f32) {
    let scale = scale.clamp(SCALE_MIN, SCALE_MAX);
    let base = egui::Style::default().text_styles;
    ctx.all_styles_mut(|s| {
        for (style, font) in s.text_styles.iter_mut() {
            if let Some(b) = base.get(style) {
                font.size = (b.size * scale).round();
            }
        }
    });
}

/// The fonts: egui's, with the chosen file first for the interface and the preview (the source
/// editor keeps its monospace font). egui's own fonts stay behind it, so a character the file
/// lacks still shows, and a file egui cannot read is skipped rather than breaking the text.
pub fn apply_font(ctx: &egui::Context, file: Option<&Path>) -> Result<(), String> {
    let mut defs = egui::FontDefinitions::default();
    let mut result = Ok(());
    if let Some(file) = file {
        match fs::read(file) {
            Ok(bytes) if is_font(&bytes) => {
                defs.font_data.insert("user".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
                defs.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "user".into());
            }
            Ok(_) => result = Err(trf("{name} is not a font file (TrueType or OpenType)", &[("name", &folder_name(file))])),
            Err(e) => result = Err(format!("{}: {e}", file.display())),
        }
    }
    ctx.set_fonts(defs);
    egui_material_icons::initialize(ctx); // set_fonts drops the icon font: add it back
    result
}

/// TrueType, OpenType or a collection of them, by the first four bytes.
pub fn is_font(bytes: &[u8]) -> bool {
    matches!(bytes.get(..4), Some([0, 1, 0, 0] | b"OTTO" | b"true" | b"ttcf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_round_trip_and_fonts_are_recognised() {
        for (t, code, _) in THEMES {
            assert_eq!(Theme::from_code(code), t);
        }
        assert_eq!(Theme::from_code("neon"), Theme::System);
        assert!(is_font(&[0, 1, 0, 0, 9]) && is_font(b"OTTO..") && !is_font(b"<html>") && !is_font(&[]));
    }

    #[test]
    fn a_font_file_loads_and_a_bad_one_is_refused() {
        let ctx = egui::Context::default();
        let dir = std::env::temp_dir().join(format!("mwp-font-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        // One of egui's own fonts, written out as a file, stands in for a font the user picks.
        let defs = egui::FontDefinitions::default();
        let (name, data) = defs.font_data.iter().next().unwrap();
        let good = dir.join(format!("{name}.ttf"));
        fs::write(&good, data.font.as_ref()).unwrap();
        assert!(apply_font(&ctx, Some(&good)).is_ok());
        let bad = dir.join("page.ttf");
        fs::write(&bad, "<html>").unwrap();
        assert!(apply_font(&ctx, Some(&bad)).unwrap_err().contains("page.ttf"));
        assert!(apply_font(&ctx, Some(&dir.join("gone.ttf"))).is_err());
        assert!(apply_font(&ctx, None).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }
}
