//! MWP files: front matter, project scan, IDs, ledger and the module map.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const PREFIXES: [&str; 4] = ["T", "SP", "ADR", "TEST"];
pub const MAP_BEGIN: &str = "<!-- mwp:map:begin -->";
pub const MAP_END: &str = "<!-- mwp:map:end -->";
/// Folders never scanned for IDs.
const SKIP_DIRS: [&str; 6] = [".git", "target", "node_modules", ".worktrees", ".mwp", ".venv"];

pub struct Doc {
    pub path: PathBuf,
    pub fm: BTreeMap<String, String>,
    pub body: String,
    pub lines: usize,
}

impl Doc {
    pub fn load(path: &Path) -> Option<Doc> {
        let text = read(path)?;
        let (fm, body) = split_front_matter(&text)?;
        Some(Doc { path: path.to_path_buf(), fm: parse_front_matter(fm), body: body.to_string(), lines: text.lines().count() })
    }
    pub fn get(&self, key: &str) -> &str {
        self.fm.get(key).map(String::as_str).unwrap_or("")
    }
    pub fn list(&self, key: &str) -> Vec<String> {
        parse_list(self.get(key))
    }
    pub fn kind(&self) -> &str {
        self.get("kind")
    }
    pub fn id(&self) -> &str {
        self.get("id")
    }
    /// First `# ` heading without the leading `<id> — `.
    pub fn title(&self) -> &str {
        let t = self.body.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or("").trim();
        t.strip_prefix(self.id()).map_or(t, |r| r.trim_start_matches([' ', '—', '-']))
    }
    /// Text under `## <heading>` up to the next `## `.
    pub fn section(&self, heading: &str) -> Option<&str> {
        let start = self.body.find(&format!("## {heading}\n"))? + heading.len() + 4;
        let rest = &self.body[start..];
        Some(&rest[..rest.find("\n## ").unwrap_or(rest.len())])
    }
}

/// Reads a text file with `\n` line endings; `None` if missing or not UTF-8.
pub fn read(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.replace("\r\n", "\n"))
}

pub fn split_front_matter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let after = &rest[end + 4..];
    Some((&rest[..end], after.strip_prefix('\n').unwrap_or(after)))
}

/// Flat `key: value` YAML, the only shape MWP uses. Comments after ` #` are dropped.
pub fn parse_front_matter(fm: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in fm.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let v = v.find(" #").map_or(v, |i| &v[..i]);
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

pub fn parse_list(v: &str) -> Vec<String> {
    let v = v.trim();
    let inner = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')).unwrap_or(v);
    inner.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Sets `key: value` inside the front matter, replacing the key or its commented placeholder.
pub fn set_field(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let close = lines.iter().skip(1).position(|l| l == "---").map(|i| i + 1).unwrap_or(lines.len());
    let new = format!("{key}: {value}");
    let found = (1..close).find(|&i| lines[i].starts_with(&format!("{key}:")))
        .or_else(|| (1..close).find(|&i| lines[i].starts_with(&format!("# {key}:"))));
    match found {
        Some(i) => lines[i] = new,
        None => lines.insert(close, new),
    }
    lines.join("\n") + "\n"
}

pub fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()) {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

pub struct Project {
    pub root: PathBuf,
    pub mwp: PathBuf,
    pub docs: Vec<Doc>,
}

impl Project {
    /// Finds the nearest folder with `mwp/`, from the current directory upward.
    pub fn find() -> Result<Project, String> {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let root = cwd.ancestors().find(|d| d.join("mwp").is_dir())
            .ok_or("no mwp/ folder here or in a parent folder (run `mwp init`)")?
            .to_path_buf();
        let mwp = root.join("mwp");
        let mut files = Vec::new();
        walk(&mwp, &mut files);
        files.sort();
        // content/ holds the project's resources, not MWP docs; its .md files may carry unrelated front matter.
        let content = mwp.join("content");
        files.retain(|p| !p.starts_with(&content));
        let docs = files.iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .filter(|p| !p.file_name().unwrap().to_string_lossy().starts_with('_'))
            .filter_map(|p| Doc::load(p))
            .collect();
        Ok(Project { root, mwp, docs })
    }
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Doc> + 'a {
        self.docs.iter().filter(move |d| d.kind() == kind)
    }
    pub fn by_id(&self, kind: &str, id: &str) -> Option<&Doc> {
        self.docs.iter().find(|d| d.kind() == kind && d.id() == id)
    }
    pub fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root).unwrap_or(p).to_string_lossy().replace('\\', "/")
    }
}

pub fn valid_id(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).and_then(|r| r.strip_prefix('-'))
        .is_some_and(|n| n.len() >= 4 && n.bytes().all(|b| b.is_ascii_digit()))
}

pub fn id_num(id: &str) -> u64 {
    id.rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(0)
}

/// Numbers of every `PREFIX-<digits>` in `text`, matched on a word boundary (`TEST-1` is not `T-1`).
fn ids_in<'a>(text: &'a str, prefix: &'a str) -> impl Iterator<Item = u64> + 'a {
    let pat = format!("{prefix}-");
    let bytes = text.as_bytes();
    text.match_indices(&pat).map(|(i, _)| i).collect::<Vec<_>>().into_iter().filter_map(move |i| {
        if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
            return None;
        }
        let digits: String = text[i + pat.len()..].chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

pub fn contains_id(text: &str, id: &str) -> bool {
    let (prefix, _) = id.rsplit_once('-').unwrap_or((id, ""));
    ids_in(text, prefix).any(|n| n == id_num(id))
}

/// Highest number used in MWP docs, module test folders or the ledger, plus one.
/// Constitution, skill and templates are not scanned: their example IDs are not real.
pub fn next_id(p: &Project, prefix: &str) -> u64 {
    let mut files: Vec<PathBuf> = p.docs.iter().map(|d| d.path.clone()).collect();
    for m in p.of_kind("module") {
        files.extend(test_files(p, m));
    }
    let used = files.iter().filter_map(|f| read(f)).flat_map(|t| ids_in(&t, prefix).collect::<Vec<_>>()).max().unwrap_or(0);
    used.max(read_ledger(&p.root).get(prefix).copied().unwrap_or(0)) + 1
}

/// Every file under a module's `tests:` paths.
pub fn test_files(p: &Project, m: &Doc) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for t in m.list("tests") {
        let path = p.root.join(t);
        if path.is_file() { out.push(path) } else { walk(&path, &mut out) }
    }
    out
}

fn ledger_path(root: &Path) -> PathBuf {
    root.join(".mwp").join("ledger.json")
}

/// `.mwp/ledger.json`: last reserved number per prefix, so a deleted file never frees its ID.
pub fn read_ledger(root: &Path) -> BTreeMap<String, u64> {
    let text = read(&ledger_path(root)).unwrap_or_default();
    text.trim().trim_start_matches('{').trim_end_matches('}').split(',')
        .filter_map(|kv| kv.split_once(':'))
        .filter_map(|(k, v)| Some((k.trim().trim_matches('"').to_string(), v.trim().parse().ok()?)))
        .collect()
}

pub fn reserve(root: &Path, prefix: &str, n: u64) -> std::io::Result<()> {
    let mut ledger = read_ledger(root);
    let e = ledger.entry(prefix.to_string()).or_insert(0);
    *e = (*e).max(n);
    let body: Vec<String> = ledger.iter().map(|(k, v)| format!("  \"{k}\": {v}")).collect();
    fs::create_dir_all(root.join(".mwp"))?;
    fs::write(ledger_path(root), format!("{{\n{}\n}}\n", body.join(",\n")))
}

pub fn render_map(p: &Project) -> String {
    let mut mods: Vec<&Doc> = p.of_kind("module").collect();
    mods.sort_by(|a, b| a.id().cmp(b.id()));
    let mut out = String::from("| ID | Type | Summary | Code | Tests | Depends on |\n|---|---|---|---|---|---|\n");
    for m in mods {
        let cell = |k: &str| m.list(k).join(", ");
        out += &format!("| {} | {} | {} | {} | {} | {} |\n", m.id(), m.get("type"), m.get("summary").replace('|', "\\|"), cell("code"), cell("tests"), cell("depends_on"));
    }
    out
}

/// Current `map.md` text and the text with a fresh table between the markers.
pub fn map_texts(p: &Project) -> Result<(String, String), String> {
    let path = p.mwp.join("map.md");
    let cur = read(&path).ok_or("mwp/map.md not found")?;
    let (b, e) = match (cur.find(MAP_BEGIN), cur.find(MAP_END)) {
        (Some(b), Some(e)) if b < e => (b, e),
        _ => return Err(format!("mwp/map.md: markers {MAP_BEGIN} / {MAP_END} not found")),
    };
    let new = format!("{}{MAP_BEGIN}\n{}{}", &cur[..b], render_map(p), &cur[e..]);
    Ok((cur, new))
}

/// Local date as `YYYY-MM-DD`, from the OS so the user's time zone is respected.
pub fn today() -> String {
    let (y, m, d) = local_date();
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(windows)]
fn local_date() -> (i32, u32, u32) {
    #[repr(C)]
    struct SystemTime([u16; 8]); // year, month, day of week, day, hour, minute, second, ms
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(t: *mut SystemTime);
    }
    let mut t = SystemTime([0; 8]);
    unsafe { GetLocalTime(&mut t) };
    (t.0[0].into(), t.0[1].into(), t.0[3].into())
}

#[cfg(unix)]
fn local_date() -> (i32, u32, u32) {
    extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(t: *const i64, tm: *mut i32) -> *mut i32;
    }
    // struct tm starts with 9 ints (sec, min, hour, mday, mon, year, ...); 16 is room for the rest.
    // shortcut: assumes 64-bit time_t, true on every 64-bit Unix target.
    let mut tm = [0i32; 16];
    unsafe { localtime_r(&time(std::ptr::null_mut()), tm.as_mut_ptr()) };
    (tm[5] + 1900, tm[4] as u32 + 1, tm[3] as u32)
}

pub fn slug(title: &str) -> String {
    let s: String = title.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter() {
        let text = "---\nkind: task\nid: T-0043   # comment\nmodules: [order, customer]\n# superseded_by: x\n---\n# T-0043 — Title\n";
        let (fm, body) = split_front_matter(text).unwrap();
        let fm = parse_front_matter(fm);
        assert_eq!(fm["id"], "T-0043");
        assert_eq!(parse_list(&fm["modules"]), ["order", "customer"]);
        assert!(!fm.contains_key("# superseded_by"));
        assert_eq!(body, "# T-0043 — Title\n");
        assert!(parse_list("[]").is_empty());
    }

    #[test]
    fn ids_respect_word_boundary() {
        let text = "TEST-0007 T-0003 NFR-09 FR-02 xT-0099";
        assert_eq!(ids_in(text, "T").max(), Some(3));
        assert_eq!(ids_in(text, "FR").max(), Some(2));
        assert!(contains_id(text, "TEST-0007"));
        assert!(!contains_id(text, "T-0007"));
        assert!(valid_id("T-0043", "T") && valid_id("T-12345", "T"));
        assert!(!valid_id("T-043", "T") && !valid_id("TEST-0001", "T"));
    }

    #[test]
    fn set_field_replaces_or_inserts() {
        let text = "---\nstatus: accepted  # c\n# superseded_by: <ADR-XXXX>\n---\nbody\n";
        let t = set_field(&set_field(text, "status", "superseded"), "superseded_by", "ADR-0007");
        assert_eq!(t, "---\nstatus: superseded\nsuperseded_by: ADR-0007\n---\nbody\n");
        assert_eq!(set_field("---\na: 1\n---\n", "b", "2"), "---\na: 1\nb: 2\n---\n");
    }

    #[test]
    fn dates_and_slugs() {
        let (y, m, d) = local_date();
        assert!(y >= 2026 && (1..=12).contains(&m) && (1..=31).contains(&d));
        assert_eq!(today().len(), 10);
        assert_eq!(slug("Usar centavos, não decimal!"), "usar-centavos-não-decimal");
    }
}
