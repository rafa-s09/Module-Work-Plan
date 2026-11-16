//! Languages. A language file is a flat JSON object that maps each English text of the GUI to
//! its translation; keys starting with `_` are notes (`_name` is the language's own name, shown
//! in Settings). A text missing from the file stays in English.
//!
//! The files live in `<config>/mwp-gui/lang/`. The bundled ones (`en`, `pt-BR`, `es`, `ja`, `ko`) are written there
//! the first time, never over the user's copy, and are also read from the binary first: a text
//! added in a new version is translated even when the user's file predates it. Any other
//! `<code>.json` dropped in the folder shows up as a language.

use super::*;
use std::collections::HashMap;
use std::sync::RwLock;

/// The languages that come with the GUI: code and file.
pub const BUNDLED: [(&str, &str); 5] = [
    ("en", include_str!("../lang/en.json")),
    ("pt-BR", include_str!("../lang/pt-BR.json")),
    ("es", include_str!("../lang/es.json")),
    ("ja", include_str!("../lang/ja.json")),
    ("ko", include_str!("../lang/ko.json")),
];

/// The texts of the language in use; `None` is English.
static TEXTS: RwLock<Option<HashMap<&'static str, &'static str>>> = RwLock::new(None);

/// The text in the language in use, or the English text itself when it has no translation.
pub fn tr(s: &str) -> &str {
    match TEXTS.read().ok().as_ref().and_then(|m| m.as_ref()?.get(s).copied()) {
        Some(found) => found,
        None => s,
    }
}

/// A text with `{name}` slots: translated first (the slots stay as they are in the file), then
/// filled in.
pub fn trf(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = tr(template).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// "1 file", "3 files": the number and the word for it, each form translated on its own.
pub fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", tr(if n == 1 { one } else { many }))
}

/// Where the language files are: next to `recent.txt`. None in tests (only the bundled ones).
pub fn folder() -> Option<PathBuf> {
    recent_file().and_then(|f| f.parent().map(|d| d.join("lang")))
}

/// Creates the folder with the bundled files that are not there yet.
pub fn ensure_folder() {
    let Some(dir) = folder() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return; // no folder: the bundled languages still work from the binary
    }
    for (code, text) in BUNDLED {
        let file = dir.join(format!("{code}.json"));
        if !file.exists() {
            let _ = fs::write(file, text);
        }
    }
}

/// A language file as text → translation; an error says what is wrong in it.
pub fn parse(text: &str) -> Result<HashMap<String, String>, String> {
    let all: HashMap<String, serde_json::Value> = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for (k, v) in all {
        match v {
            serde_json::Value::String(s) => {
                out.insert(k, s);
            }
            other => return Err(format!("`{k}`: the translation must be text, not {other}")),
        }
    }
    Ok(out)
}

/// The languages to choose from: code and name, English first. Bundled ones, then the files in
/// the folder; a file that does not read is listed with its code, so the error shows on picking it.
pub fn available() -> Vec<(String, String)> {
    let name = |code: &str, text: &str| parse(text).ok().and_then(|m| m.get("_name").cloned()).unwrap_or_else(|| code.to_string());
    let mut out: Vec<(String, String)> = BUNDLED.iter().map(|(c, text)| (c.to_string(), name(c, text))).collect();
    let files = folder().and_then(|d| fs::read_dir(d).ok()).into_iter().flatten().flatten().map(|e| e.path());
    for path in files.filter(|p| extension(p) == "json") {
        let code = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !code.is_empty() && !out.iter().any(|o| o.0 == code) {
            out.push((code.clone(), name(&code, &fs::read_to_string(&path).unwrap_or_default())));
        }
    }
    out
}

/// The texts of a language: the bundled file (if any) with the user's file over it.
pub fn texts(code: &str) -> Result<HashMap<String, String>, String> {
    let mut out = match BUNDLED.iter().find(|b| b.0 == code) {
        Some((_, text)) => parse(text).map_err(|e| format!("bundled {code}.json: {e}"))?,
        None => HashMap::new(),
    };
    if let Some(file) = folder().map(|d| d.join(format!("{code}.json"))).filter(|f| f.exists()) {
        let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        out.extend(parse(&text).map_err(|e| format!("{}: {e}", file.display()))?);
    }
    if out.is_empty() && code != "en" {
        return Err(format!("no language file for `{code}`"));
    }
    Ok(out)
}

/// Switches the GUI to a language. On an error the language in use stays.
// shortcut: each switch leaks its texts (a few KB) so `tr` can hand out `&'static str`; fine for
// a switch now and then, would add up only if languages were switched in a loop.
pub fn set(code: &str) -> Result<(), String> {
    let texts = if code == "en" { None } else { Some(texts(code)?) };
    let leaked = texts.map(|m| m.into_iter().filter(|(k, _)| !k.starts_with('_')).map(|(k, v)| (&*Box::leak(k.into_boxed_str()), &*Box::leak(v.into_boxed_str()))).collect());
    if let Ok(mut cur) = TEXTS.write() {
        *cur = leaked;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_files_read_and_report_errors() {
        let m = parse("\u{feff}{\"_name\": \"Test\", \"Save\": \"Salvar\"}").unwrap();
        assert_eq!((m["_name"].as_str(), m["Save"].as_str()), ("Test", "Salvar"));
        assert!(parse("{\"Save\": 3}").unwrap_err().contains("`Save`"));
        assert!(parse("{\"Save\": \"x\",}").is_err(), "a trailing comma is an error, not a silent skip");
        // Every bundled file reads and translates every text en has.
        let en = parse(BUNDLED[0].1).unwrap();
        for (code, text) in &BUNDLED[1..] {
            let lang = parse(text).unwrap();
            let missing: Vec<&String> = en.keys().filter(|k| !k.starts_with('_') && !lang.contains_key(*k)).collect();
            assert!(missing.is_empty(), "{code}.json lacks: {missing:?}");
            // A translation keeps the {slots} of its text, or trf leaves them unfilled.
            let slots = |s: &str| {
                let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}')).map(|(n, _)| n.to_string()).collect();
                v.sort();
                v
            };
            let broken: Vec<&String> = lang.iter().filter(|(k, v)| !k.starts_with('_') && slots(k) != slots(v)).map(|(k, _)| k).collect();
            assert!(broken.is_empty(), "{code}.json changes the {{slots}} of: {broken:?}");
        }
        assert!(en.iter().all(|(k, v)| k.starts_with('_') || k == v), "en.json maps each text to itself");
    }

    /// Every text the code passes to `tr`, `trf` or `count` is in en.json, so a new text can
    /// never be left out of the language files. (Texts kept in tables and shown through
    /// `tr(label)` are listed in en.json by hand.)
    #[test]
    fn every_text_in_the_code_is_in_en_json() {
        let en = parse(BUNDLED[0].1).unwrap();
        let (mut missing, mut seen) = (Vec::new(), 0);
        for entry in fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).unwrap().flatten() {
            let src = fs::read_to_string(entry.path()).unwrap();
            let code = src.split("#[cfg(test)]").next().unwrap();
            // The string literals right after `tr(`, `trf(`, and the two after `count(n, `.
            for (i, _) in code.match_indices('(') {
                let before = &code[..i];
                let word = before.rsplit(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
                let lits: usize = match word {
                    "tr" | "trf" => 1,
                    "count" => 2,
                    _ => continue,
                };
                let mut rest = &code[i + 1..];
                if word == "count" {
                    rest = rest.split_once(", ").map_or("", |r| r.1);
                }
                for _ in 0..lits {
                    let Some(body) = rest.strip_prefix('"') else { break };
                    let end = body.find('"').unwrap(); // the texts hold no escaped quotes
                    seen += 1;
                    let text = body[..end].replace("\\n", "\n");
                    if !en.contains_key(&text) {
                        missing.push(text);
                    }
                    rest = body[end + 1..].trim_start_matches(", ");
                }
            }
        }
        assert!(seen > 200, "the scan found only {seen} texts");
        assert!(missing.is_empty(), "add to lang/en.json and lang/pt-BR.json: {missing:?}");
    }
}
