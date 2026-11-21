//! State shared by agents working at the same time: the ID ledger, task claims and module locks.
//! Plain files under `.mwp/`, created atomically; no git, no daemon.

use crate::doc::{next_id, read, used_id, Project, PREFIXES};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// `.mwp/` in the project, or `MWP_STATE_DIR` when several checkouts of one project
/// (one per agent) must share IDs and locks.
pub fn dir(root: &Path) -> PathBuf {
    std::env::var_os("MWP_STATE_DIR").filter(|v| !v.is_empty()).map_or_else(|| root.join(".mwp"), PathBuf::from)
}

/// `ledger.json`: last reserved number per prefix, so a deleted file never frees its ID.
pub fn read_ledger(state: &Path) -> BTreeMap<String, u64> {
    let text = read(&state.join("ledger.json")).unwrap_or_default();
    text.trim().trim_start_matches('{').trim_end_matches('}').split(',')
        .filter_map(|kv| kv.split_once(':'))
        .filter_map(|(k, v)| Some((k.trim().trim_matches('"').to_string(), v.trim().parse().ok()?)))
        .collect()
}

/// The ledger read strictly: `{ "T": 12, ... }` with known prefixes and whole numbers.
/// None when the file is missing or does not read (a bad merge, an edit by hand).
pub fn parse_ledger(state: &Path) -> Option<BTreeMap<String, u64>> {
    let text = read(&state.join("ledger.json"))?;
    let body = text.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut out = BTreeMap::new();
    for kv in body.split(',').filter(|_| !body.is_empty()) {
        let (k, v) = kv.split_once(':')?;
        let k = k.trim().strip_prefix('"')?.strip_suffix('"')?;
        if !PREFIXES.contains(&k) || out.insert(k.to_string(), v.trim().parse::<u64>().ok()?).is_some() {
            return None;
        }
    }
    Some(out)
}

fn write_ledger(state: &Path, ledger: &BTreeMap<String, u64>) -> Result<(), String> {
    let body: Vec<String> = ledger.iter().map(|(k, v)| format!("  \"{k}\": {v}")).collect();
    fs::create_dir_all(state).and_then(|_| fs::write(state.join("ledger.json"), format!("{{\n{}\n}}\n", body.join(",\n")))).map_err(|e| format!("ledger.json: {e}"))
}

/// Task, SPEC and ADR IDs held by more than one file, as `T-0005 (a.md, b.md)`.
pub fn duplicate_ids(p: &Project) -> Vec<String> {
    let mut by_id: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for d in p.docs.iter().filter(|d| matches!(d.kind(), "task" | "spec" | "adr") && !d.id().is_empty()) {
        by_id.entry(d.id()).or_default().push(p.rel(&d.path));
    }
    by_id.into_iter().filter(|(_, files)| files.len() > 1).map(|(id, files)| format!("{id} ({})", files.join(", "))).collect()
}

/// Refuses while two files share an ID: a new ID could not tell them apart.
fn no_duplicates(p: &Project) -> Result<(), String> {
    let dups = duplicate_ids(p);
    if dups.is_empty() {
        return Ok(());
    }
    // Above the files and any reservation still in the ledger.
    let mut prefixes: Vec<&str> = dups.iter().filter_map(|d| d.split_once('-').map(|x| x.0)).collect();
    prefixes.dedup();
    let free: Vec<String> = prefixes.iter().map(|x| format!("{x}-{:04}", next_id(p, x))).collect();
    Err(format!("duplicate ids: {}. Give one of each a free id by hand ({}), then run `mwp ledger`", dups.join("; "), free.join(", ")))
}

/// The ledger from the highest ID of each prefix in the project, never below what a ledger that
/// still reads already reserved. Refuses while two files share an ID.
fn rebuild(p: &Project) -> Result<BTreeMap<String, u64>, String> {
    no_duplicates(p)?;
    let old = parse_ledger(&p.state).unwrap_or_default();
    let ledger: BTreeMap<String, u64> = PREFIXES.iter()
        .map(|x| (x.to_string(), used_id(p, x).max(old.get(*x).copied().unwrap_or(0))))
        .filter(|(_, n)| *n > 0)
        .collect();
    write_ledger(&p.state, &ledger)?;
    Ok(ledger)
}

/// Checks the ledger before an ID is handed out: no duplicate IDs, and a ledger that reads.
/// A missing or broken ledger is rebuilt; returns a note when it was.
fn check(p: &Project) -> Result<Option<String>, String> {
    no_duplicates(p)?;
    if parse_ledger(&p.state).is_some() {
        return Ok(None);
    }
    let why = if p.state.join("ledger.json").exists() { "did not read" } else { "was missing" };
    rebuild(p)?;
    Ok(Some(format!("ledger.json {why}: rebuilt from the highest ID of each prefix")))
}

/// `check`, holding the ledger lock.
pub fn check_ledger(p: &Project) -> Result<Option<String>, String> {
    let _lock = guard(&p.state).map_err(|e| e.to_string())?;
    check(p)
}

/// `mwp ledger`: writes the ledger again from the project. Returns it.
pub fn rebuild_ledger(p: &Project) -> Result<BTreeMap<String, u64>, String> {
    let _lock = guard(&p.state).map_err(|e| e.to_string())?;
    rebuild(p)
}

/// Removes the ledger lock when the allocation ends, even on error.
struct Guard(PathBuf);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn guard(state: &Path) -> io::Result<Guard> {
    fs::create_dir_all(state)?;
    let path = state.join("ledger.lock");
    for _ in 0..600 {
        if create(&path, "")? {
            return Ok(Guard(path));
        }
        // A process that died holding the lock: take it over after 10 s.
        let stale = fs::metadata(&path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() >= 10);
        if stale {
            let _ = fs::remove_file(&path);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err(io::Error::other("the ID ledger is locked by another process"))
}

/// Takes the next free ID and records it, one process at a time: two agents never get the same ID.
/// The ledger is checked first (see `check`).
pub fn allocate(p: &Project, prefix: &str) -> Result<u64, String> {
    let _lock = guard(&p.state).map_err(|e| e.to_string())?;
    check(p)?;
    let n = next_id(p, prefix);
    let mut ledger = read_ledger(&p.state);
    ledger.insert(prefix.to_string(), n);
    write_ledger(&p.state, &ledger)?;
    Ok(n)
}

/// Who holds a task or a module.
pub struct Claim {
    pub task: String,
    pub agent: String,
}

/// File name for a task or module id.
fn key(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

/// Creates the file only if it does not exist (atomic); `false` if someone got there first.
fn create(path: &Path, content: &str) -> io::Result<bool> {
    fs::create_dir_all(path.parent().unwrap())?;
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut f) => f.write_all(content.as_bytes()).map(|_| true),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

fn holder(path: &Path) -> Claim {
    let text = read(path).unwrap_or_default();
    let (task, agent) = text.trim().split_once(' ').unwrap_or(("?", "?"));
    Claim { task: task.into(), agent: agent.into() }
}

/// Claims a task and locks every module it writes. All or nothing.
pub fn claim(state: &Path, task: &str, agent: &str, modules: &[String]) -> Result<(), String> {
    let body = format!("{task} {agent}\n");
    let own = state.join("claims").join(key(task));
    if !create(&own, &body).map_err(|e| e.to_string())? {
        return Err(format!(
            "{task} is already in progress (agent: {}). If that agent stopped, run `mwp task release {task}` first",
            holder(&own).agent
        ));
    }
    for m in modules {
        let path = state.join("locks").join(key(m));
        let error = match create(&path, &body) {
            Ok(true) => continue,
            Ok(false) => {
                let h = holder(&path);
                if h.task == task {
                    continue; // the module is listed twice
                }
                format!("module `{m}` is being written by {} (agent: {}); wait for it to close or pick another task", h.task, h.agent)
            }
            Err(e) => e.to_string(),
        };
        release(state, task);
        return Err(error);
    }
    Ok(())
}

/// Frees a task and every module it locked.
pub fn release(state: &Path, task: &str) {
    let _ = fs::remove_file(state.join("claims").join(key(task)));
    for (path, c) in entries(&state.join("locks")) {
        if c.task == task {
            let _ = fs::remove_file(path);
        }
    }
}

fn entries(dir: &Path) -> Vec<(PathBuf, Claim)> {
    let mut out: Vec<_> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| (e.path(), holder(&e.path()))).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Tasks in progress.
pub fn claims(state: &Path) -> Vec<Claim> {
    entries(&state.join("claims")).into_iter().map(|(_, c)| c).collect()
}

/// Locked modules: (module, holder).
pub fn locks(state: &Path) -> Vec<(String, Claim)> {
    entries(&state.join("locks")).into_iter().map(|(p, c)| (p.file_name().unwrap().to_string_lossy().into_owned(), c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ledger_is_checked_before_an_id_is_given() {
        let root = std::env::temp_dir().join(format!("mwp-ledger-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let open_task = root.join("mwp/tasks/open");
        fs::create_dir_all(&open_task).unwrap();
        let task = |id: &str, file: &str| fs::write(open_task.join(file), format!("---
kind: task
id: {id}
status: open
size: S
modules: []
---
# {id}
")).unwrap();
        task("T-0007", "T-0007-a.md");
        let open = || Project::open(&root).unwrap();
        // Missing: rebuilt from the highest ID in the project.
        assert!(check_ledger(&open()).unwrap().unwrap().contains("missing"));
        assert_eq!(parse_ledger(&open().state).unwrap()["T"], 7);
        assert_eq!(allocate(&open(), "T").unwrap(), 8);
        assert_eq!(check_ledger(&open()).unwrap(), None);
        // Broken: rebuilt too; a reservation in a ledger that still reads is never lowered.
        fs::write(open().state.join("ledger.json"), "{ \"T\": 8,, }").unwrap();
        assert!(check_ledger(&open()).unwrap().unwrap().contains("did not read"));
        assert_eq!(parse_ledger(&open().state).unwrap()["T"], 7, "the broken reservation is lost, the files are not");
        fs::write(open().state.join("ledger.json"), "{
  \"T\": 20
}
").unwrap();
        assert_eq!(rebuild_ledger(&open()).unwrap()["T"], 20);
        // Two files with one ID: no new ID, and no rebuild, until one of them is renumbered.
        task("T-0007", "T-0007-b.md");
        let err = allocate(&open(), "T").unwrap_err();
        assert!(err.contains("T-0007 (mwp/tasks/open/T-0007-a.md, mwp/tasks/open/T-0007-b.md)") && err.contains("(T-0021)"), "{err}");
        assert!(rebuild_ledger(&open()).is_err() && check_ledger(&open()).is_err());
        task("T-0021", "T-0007-b.md");
        assert_eq!(allocate(&open(), "T").unwrap(), 22);
        assert!(parse_ledger(&root.join("nowhere")).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn module_locks_are_all_or_nothing() {
        let state = std::env::temp_dir().join(format!("mwp-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state);
        let mods = |m: &[&str]| m.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        claim(&state, "T-0001", "claude", &mods(&["order", "customer"])).unwrap();
        assert!(claim(&state, "T-0001", "codex", &[]).unwrap_err().contains("claude"));
        // T-0002 wants `stock` (free) and `order` (taken): it gets neither.
        assert!(claim(&state, "T-0002", "codex", &mods(&["stock", "order"])).unwrap_err().contains("T-0001"));
        assert_eq!(locks(&state).len(), 2);
        assert_eq!(claims(&state).len(), 1);

        release(&state, "T-0001");
        claim(&state, "T-0002", "codex", &mods(&["stock", "order"])).unwrap();
        assert_eq!(locks(&state).iter().filter(|l| l.1.agent == "codex").count(), 2);
        let _ = fs::remove_dir_all(&state);
    }
}
