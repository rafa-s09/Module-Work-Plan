//! Mechanical checks on an MWP project. Nothing here judges content.

use crate::doc::{contains_id, map_texts, read, test_files, valid_id, Doc, Project};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    fn err(&mut self, p: &Project, d: &Doc, msg: String) {
        self.errors.push(format!("{}: {msg}", p.rel(&d.path)));
    }
    fn warn(&mut self, p: &Project, d: &Doc, msg: String) {
        self.warnings.push(format!("{}: {msg}", p.rel(&d.path)));
    }
    pub fn print(&self) {
        for e in &self.errors {
            println!("ERROR {e}");
        }
        for w in &self.warnings {
            println!("WARN  {w}");
        }
        println!("{} error(s), {} warning(s)", self.errors.len(), self.warnings.len());
    }
}

const KINDS: [(&str, &str, &[&str]); 5] = [
    ("project", "", &["mwp_version", "name"]),
    ("module", "", &["id", "type", "summary", "updated"]),
    ("task", "T", &["id", "status", "size", "modules"]),
    ("spec", "SP", &["id", "lifetime", "status", "owner"]),
    ("adr", "ADR", &["id", "status", "date"]),
];

const ENUMS: [(&str, &str, &[&str]); 6] = [
    ("task", "status", &["open", "in_progress", "blocked", "review", "done"]),
    ("task", "size", &["S", "M", "L"]),
    ("spec", "lifetime", &["durable", "transient"]),
    ("spec", "status", &["active", "incorporated", "archived"]),
    ("adr", "status", &["proposed", "accepted", "rejected", "superseded", "deprecated"]),
    ("module", "type", &["ui", "domain", "infra"]),
];

/// Line limits from the format; over them is a warning, not an error.
const LIMITS: [(&str, usize); 3] = [("project", 60), ("module", 200), ("task", 80)];

pub fn lint(p: &Project) -> Report {
    let mut r = Report::default();
    let mut seen: BTreeMap<&str, &Doc> = BTreeMap::new();

    for d in &p.docs {
        let Some((kind, prefix, required)) = KINDS.iter().find(|k| k.0 == d.kind()) else {
            r.err(p, d, format!("unknown kind `{}`", d.kind()));
            continue;
        };
        for f in *required {
            if !d.fm.contains_key(*f) || (d.get(f).is_empty() && *f != "modules") {
                r.err(p, d, format!("missing field `{f}`"));
            }
        }
        for (_, field, values) in ENUMS.iter().filter(|e| e.0 == *kind) {
            let v = d.get(field);
            if !v.is_empty() && !values.contains(&v) {
                let msg = format!("`{field}: {v}` is not one of {}", values.join(" | "));
                if *field == "type" { r.warn(p, d, msg) } else { r.err(p, d, msg) }
            }
        }
        if !prefix.is_empty() && !valid_id(d.id(), prefix) {
            r.err(p, d, format!("id `{}` must be {prefix}- plus at least 4 digits", d.id()));
        }
        if !d.id().is_empty() {
            if let Some(other) = seen.insert(d.id(), d) {
                r.err(p, d, format!("duplicate id `{}` (also in {})", d.id(), p.rel(&other.path)));
            }
        }
        if let Some((_, max)) = LIMITS.iter().find(|l| l.0 == *kind) {
            if d.lines > *max {
                r.warn(p, d, format!("{} lines, limit is ~{max}", d.lines));
            }
        }
        match *kind {
            "task" => check_task(p, d, &mut r),
            "spec" => {
                let o = d.get("owner");
                if p.by_id("module", o).is_none() && p.by_id("task", o).is_none() {
                    r.err(p, d, format!("owner `{o}` is not a module or task"));
                }
            }
            "adr" => check_adr(p, d, &mut r),
            _ => {}
        }
    }

    if let Some(c) = read(&p.mwp.join("constitution.md")) {
        if c.lines().count() > 150 {
            r.warnings.push(format!("mwp/constitution.md: {} lines, limit is ~150", c.lines().count()));
        }
    } else {
        r.errors.push("mwp/constitution.md: missing".into());
    }
    match map_texts(p) {
        Ok((cur, new)) if cur != new => r.errors.push("mwp/map.md: out of date (run `mwp remap` or `mwp map --write`)".into()),
        Err(e) => r.errors.push(e),
        _ => {}
    }
    check_modules(p, &mut r);
    r
}

fn refs(p: &Project, d: &Doc, r: &mut Report, field: &str, kind: &str) {
    for id in d.list(field) {
        if p.by_id(kind, &id).is_none() {
            r.err(p, d, format!("`{field}` cites `{id}`, no {kind} has that id"));
        }
    }
}

fn check_task(p: &Project, d: &Doc, r: &mut Report) {
    refs(p, d, r, "modules", "module");
    refs(p, d, r, "reads", "module");
    refs(p, d, r, "specs", "spec");
    refs(p, d, r, "decisions", "adr");
    refs(p, d, r, "refs", "task");
    let in_done = d.path.parent().is_some_and(|f| f.ends_with("done"));
    let done = d.get("status") == "done";
    if in_done != done {
        r.err(p, d, format!("`status: {}` but the file is in tasks/{}", d.get("status"), if in_done { "done" } else { "open" }));
    }
    if done && !d.section("Closure").is_some_and(has_content) {
        r.err(p, d, "status is done but `## Closure` is empty".into());
    }
    if !d.path.file_name().unwrap().to_string_lossy().starts_with(d.id()) {
        r.warn(p, d, format!("file name should start with `{}`", d.id()));
    }
    for t in d.list("refs") {
        if p.by_id("task", &t).is_some_and(|x| x.get("status") != "done") {
            r.err(p, d, format!("`refs` cites `{t}`, which is not done"));
        }
    }
}

/// True if a section has text beyond `>` hints and empty `- **Label:**` lines.
fn has_content(s: &str) -> bool {
    s.lines().map(str::trim).filter(|l| !l.starts_with('>')).any(|l| {
        let l = l.trim_start_matches("- ");
        let l = l.strip_prefix("**").and_then(|x| x.split_once(":**")).map_or(l, |(_, rest)| rest);
        !l.trim().is_empty()
    })
}

fn check_adr(p: &Project, d: &Doc, r: &mut Report) {
    refs(p, d, r, "modules", "module");
    refs(p, d, r, "supersedes", "adr");
    let by = d.get("superseded_by");
    if d.get("status") == "superseded" {
        if by.is_empty() {
            r.err(p, d, "status is superseded but `superseded_by` is missing".into());
        } else if p.by_id("adr", by).is_none() {
            r.err(p, d, format!("`superseded_by` cites `{by}`, no adr has that id"));
        }
    }
}

/// Module checks shared by `lint` and `remap`: paths, dependencies and registered tests.
pub fn check_modules(p: &Project, r: &mut Report) {
    for m in p.of_kind("module") {
        refs(p, m, r, "depends_on", "module");
        refs(p, m, r, "specs", "spec");
        for field in ["code", "tests"] {
            for path in m.list(field) {
                if !p.root.join(&path).exists() {
                    r.err(p, m, format!("`{field}` path `{path}` does not exist"));
                }
            }
        }
        let code: Vec<String> = test_files(p, m).iter().filter_map(|f| read(f)).collect();
        for id in registered_tests(m) {
            if !code.iter().any(|t| contains_id(t, &id)) {
                r.err(p, m, format!("{id} is in `## Tests` but not found under `tests` paths"));
            }
        }
    }
}

/// TEST IDs in the first column of the module's `## Tests` table.
pub fn registered_tests(m: &Doc) -> Vec<String> {
    m.section("Tests").unwrap_or("").lines()
        .filter_map(|l| l.trim().strip_prefix('|')?.split('|').next().map(|c| c.trim().to_string()))
        .filter(|c| valid_id(c, "TEST"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_content() {
        assert!(!has_content("> hint\n- **Done:**\n- **SPECs:**\n"));
        assert!(has_content("- **Done:** added discount\n"));
        assert!(has_content("Free text.\n"));
    }
}
