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

const KINDS: [(&str, &str, &[&str]); 8] = [
    ("project", "", &["mwp_version", "name"]),
    ("constitution", "", &["updated"]),
    ("map", "", &[]),
    ("glossary", "", &[]),
    ("module", "", &["id", "type", "summary", "updated"]),
    ("task", "T", &["id", "status", "size", "modules"]),
    ("spec", "SP", &["id", "lifetime", "status", "owner"]),
    ("adr", "ADR", &["id", "status", "date"]),
];

const ENUMS: [(&str, &str, &[&str]); 6] = [
    ("task", "status", &["open", "in_progress", "blocked", "review", "done", "replanned"]),
    ("task", "size", &SIZES),
    ("spec", "lifetime", &["durable", "transient"]),
    ("spec", "status", &["active", "incorporated", "archived"]),
    ("adr", "status", &["proposed", "accepted", "rejected", "superseded", "deprecated"]),
    ("module", "type", &MODULE_TYPES),
];

/// Task sizes, smallest first. XS and S use the short template; L and up need an approved plan.
pub const SIZES: [&str; 6] = ["XS", "S", "M", "L", "XL", "XXL"];

/// Sizes whose `## Plan` a person approves (`plan_approved:`) before the task is implemented.
pub fn needs_plan(size: &str) -> bool {
    matches!(size, "L" | "XL" | "XXL")
}

/// A closed task lives in `tasks/done/`: finished (`done`) or given up for new tasks (`replanned`).
pub fn closed(status: &str) -> bool {
    matches!(status, "done" | "replanned")
}

/// True if the `- **<label>:**` line of a section has text after the label.
pub fn field_filled(section: &str, label: &str) -> bool {
    let head = format!("**{label}:**");
    section.lines().filter_map(|l| l.trim().trim_start_matches("- ").strip_prefix(head.as_str())).any(|rest| !rest.trim().is_empty())
}

/// Module types: a label shown in `map.md`; nothing else depends on it.
pub const MODULE_TYPES: [&str; 5] = ["domain", "ui", "api", "infra", "lib"];

/// Line limits, sized for large projects. Over them is always a warning, never an error.
pub const LIMITS: [(&str, usize); 3] = [("project", 150), ("module", 500), ("task", 200)];
pub const CONSTITUTION_LIMIT: usize = 300;

pub fn lint(p: &Project) -> Report {
    let mut r = Report::default();
    let mut seen: BTreeMap<&str, &Doc> = BTreeMap::new();

    // Without this, a broken task or module would vanish from every command.
    for path in &p.invalid {
        r.errors.push(format!("{}: no front matter; the file must start with `---` and close it with `---`", p.rel(path)));
    }

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
        if c.lines().count() > CONSTITUTION_LIMIT {
            r.warnings.push(format!("mwp/constitution.md: {} lines, limit is ~{CONSTITUTION_LIMIT}", c.lines().count()));
        }
    } else {
        r.errors.push("mwp/constitution.md: missing".into());
    }
    project_docs(p, &mut r);
    if p.state.join("ledger.json").exists() && crate::state::parse_ledger(&p.state).is_none() {
        r.warnings.push(".mwp/ledger.json: does not read; `mwp next` rebuilds it, or run `mwp ledger`".into());
    }
    match map_texts(p) {
        Ok((cur, new)) if cur != new => r.errors.push("mwp/map.md: out of date (run `mwp remap` or `mwp map --write`)".into()),
        Err(e) => r.errors.push(e),
        _ => {}
    }
    check_modules(p, &mut r);
    r
}

/// The files at the top of `mwp/`: each one's `kind:` matches its name, and the dates read as dates.
/// A file without front matter predates it: a warning, so older projects keep passing.
fn project_docs(p: &Project, r: &mut Report) {
    for (file, kind) in [("PROJECT.md", "project"), ("constitution.md", "constitution"), ("map.md", "map"), ("glossary.md", "glossary")] {
        let path = p.mwp.join(file);
        match p.docs.iter().find(|d| d.path == path) {
            Some(d) if d.kind() != kind => r.err(p, d, format!("`kind: {}` but this file is the {kind}; use `kind: {kind}`", d.kind())),
            Some(d) => {
                for field in ["created", "updated"].into_iter().filter(|f| d.fm.contains_key(*f)) {
                    if !is_date(d.get(field)) {
                        r.err(p, d, format!("`{field}: {}` is not a date (YYYY-MM-DD)", d.get(field)));
                    }
                }
                if kind == "project" && !d.fm.contains_key("created") {
                    r.warn(p, d, "no `created:` date".into());
                }
                if kind == "project" && !d.get("mwp_version").is_empty() {
                    let v = d.get("mwp_version");
                    match (version(v), version(crate::scaffold::VERSION)) {
                        (Some(theirs), Some(ours)) if theirs > ours => r.err(p, d, format!("written for mwp {v}, newer than this mwp {}: update mwp", crate::scaffold::VERSION)),
                        (Some(theirs), Some(ours)) if theirs == ours => {}
                        _ => r.warn(p, d, format!("written for mwp {v}; this is mwp {}: run `mwp upgrade`", crate::scaffold::VERSION)),
                    }
                }
            }
            None if path.exists() => r.warnings.push(format!("mwp/{file}: no front matter; start it with `---`, `kind: {kind}`, `---`")),
            None => {}
        }
    }
    for d in p.docs.iter().filter(|d| matches!(d.kind(), "constitution" | "map" | "glossary")) {
        if d.path.parent() != Some(p.mwp.as_path()) {
            r.err(p, d, format!("`kind: {}` belongs only to mwp/{}.md", d.kind(), d.kind()));
        }
    }
}

/// `0.3.1` as numbers, to compare; None for anything else (older projects say `2`).
pub fn version(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.trim().split('.').map(|n| n.parse::<u64>().ok());
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// `YYYY-MM-DD`, by shape.
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
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
    let status = d.get("status");
    if in_done != closed(status) {
        r.err(p, d, format!("`status: {status}` but the file is in tasks/{}", if in_done { "done" } else { "open" }));
    }
    if closed(status) && !d.section("Closure").is_some_and(has_content) {
        r.err(p, d, format!("status is {status} but `## Closure` is empty"));
    }
    // Done tasks are history (some predate the field): only open work is checked.
    if needs_plan(d.get("size")) && d.get("plan_approved").is_empty() {
        let msg = format!("size {} needs `plan_approved:`, set when a person approves `## Plan`", d.get("size"));
        match status {
            "review" => r.err(p, d, msg),
            "in_progress" => r.warn(p, d, format!("{msg}; write the plan and stop for approval")),
            _ => {}
        }
    }
    // `- [-]` marks a criterion that failed: what a replanned task has, and a done one must not.
    let failed = d.section("Acceptance criteria").unwrap_or("").lines().any(|l| l.trim_start().starts_with("- [-]"));
    match status {
        "replanned" if !failed => r.warn(p, d, "status is replanned but no criterion is marked `- [-]` (failed)".into()),
        "done" if failed => r.warn(p, d, "status is done but a criterion is marked `- [-]` (failed): replan it, or check it with evidence".into()),
        _ => {}
    }
    if status == "replanned" && !d.section("Closure").is_some_and(|c| field_filled(c, "Replanning")) {
        r.err(p, d, "status is replanned but `**Replanning:**` in `## Closure` is empty".into());
    }
    if !d.path.file_name().unwrap().to_string_lossy().starts_with(d.id()) {
        r.warn(p, d, format!("file name should start with `{}`", d.id()));
    }
    for t in d.list("refs") {
        if p.by_id("task", &t).is_some_and(|x| !closed(x.get("status"))) {
            r.err(p, d, format!("`refs` cites `{t}`, which is not done or replanned"));
        }
    }
}

/// True if a section has text beyond `>` hints and empty `- **Label:**` lines.
pub fn has_content(s: &str) -> bool {
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
        // A path may name code or tests a task has yet to write: a warning, not an error.
        for field in ["code", "tests"] {
            for path in m.list(field) {
                if !p.root.join(&path).exists() {
                    r.warn(p, m, format!("`{field}` path `{path}` does not exist yet"));
                }
            }
        }
        let tests_exist = m.list("tests").iter().any(|t| p.root.join(t).exists());
        let code: Vec<String> = test_files(p, m).iter().filter_map(|f| read(f)).collect();
        for id in registered_tests(m) {
            if !code.iter().any(|t| contains_id(t, &id)) {
                let msg = format!("{id} is in `## Tests` but not found under `tests` paths");
                if tests_exist { r.err(p, m, msg) } else { r.warn(p, m, format!("{msg} (none exists yet)")) }
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
        assert!(is_date("2026-11-20") && !is_date("<YYYY-MM-DD>") && !is_date("2026-1-20"));
        assert!(version("0.3.1") == Some((0, 3, 1)) && version("0.10.0") > version("0.9.9"));
        assert!(version("2").is_none() && version("1.2.3.4").is_none() && version("<mwp version>").is_none());
    }
}
