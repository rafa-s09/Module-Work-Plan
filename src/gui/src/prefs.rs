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
/// `lang` puts its own CJK font first, so Chinese text is not drawn with Japanese glyph shapes.
pub fn apply_font(ctx: &egui::Context, file: Option<&Path>, lang: &str) -> Result<(), String> {
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
    let mut fonts: Vec<_> = cjk_fonts().iter().collect();
    fonts.sort_by_key(|f| f.0 != lang);
    for (_, name, data) in fonts {
        defs.font_data.insert(name.clone(), data.clone());
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            defs.families.entry(family).or_default().push(name.clone());
        }
    }
    // The icon font goes into the same definitions: inside a frame, add_font after set_fonts
    // sees the icons still installed, skips them, and the new definitions then drop them.
    let icons = egui_material_icons::font_insert();
    defs.font_data.insert(icons.name.clone(), std::sync::Arc::new(icons.data));
    for f in icons.families {
        let list = defs.families.entry(f.family).or_default();
        match f.priority {
            egui::epaint::text::FontPriority::Highest => list.insert(0, icons.name.clone()),
            egui::epaint::text::FontPriority::Lowest => list.push(icons.name.clone()),
        }
    }
    ctx.set_fonts(defs);
    result
}

/// System fonts with Japanese, Korean and Chinese characters, which egui's fonts lack: the
/// language and the first font found of each group, read once. A system without them shows
/// those characters as boxes.
type CjkFont = (&'static str, String, std::sync::Arc<egui::FontData>);
fn cjk_fonts() -> &'static [CjkFont] {
    static FONTS: std::sync::OnceLock<Vec<CjkFont>> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let win = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let noto = ["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc"];
        let japanese = [format!(r"{win}\Fonts\YuGothR.ttc"), format!(r"{win}\Fonts\msgothic.ttc"), "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc".into()];
        let korean = [format!(r"{win}\Fonts\malgun.ttf"), "/System/Library/Fonts/AppleSDGothicNeo.ttc".into()];
        let simplified = [format!(r"{win}\Fonts\msyh.ttc"), "/System/Library/Fonts/PingFang.ttc".into()];
        let traditional = [format!(r"{win}\Fonts\msjh.ttc"), "/System/Library/Fonts/PingFang.ttc".into()];
        let mut out: Vec<CjkFont> = Vec::new();
        for (lang, group) in [("ja", japanese.as_slice()), ("ko", korean.as_slice()), ("zh-CN", simplified.as_slice()), ("zh-TW", traditional.as_slice())] {
            let found = group.iter().map(String::as_str).chain(noto).find_map(|p| Some(p).zip(fs::read(p).ok().filter(|b| is_font(b))));
            if let Some((path, bytes)) = found {
                if !out.iter().any(|(_, name, _)| name == path) {
                    let mut data = egui::FontData::from_owned(bytes);
                    // Yu Gothic draws its glyphs well above egui's baseline: shift them down to line up.
                    if path.ends_with("YuGothR.ttc") {
                        data.tweak.y_offset_factor = 0.3;
                    }
                    out.push((lang, path.to_string(), std::sync::Arc::new(data)));
                }
            }
        }
        out
    })
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
        assert!(apply_font(&ctx, Some(&good), "en").is_ok());
        let bad = dir.join("page.ttf");
        fs::write(&bad, "<html>").unwrap();
        assert!(apply_font(&ctx, Some(&bad), "en").unwrap_err().contains("page.ttf"));
        assert!(apply_font(&ctx, Some(&dir.join("gone.ttf")), "en").is_err());
        assert!(apply_font(&ctx, None, "zh-CN").is_ok());
        let _ = fs::remove_dir_all(&dir);
        // Windows ships a Japanese, a Korean and both Chinese fonts, so all are found there.
        #[cfg(windows)]
        assert_eq!(cjk_fonts().iter().map(|f| f.0).collect::<Vec<_>>(), ["ja", "ko", "zh-CN", "zh-TW"]);
    }
}
