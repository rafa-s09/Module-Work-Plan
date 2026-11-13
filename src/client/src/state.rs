//! State shared by agents working at the same time: the ID ledger, task claims and module locks.
//! Plain files under `.mwp/`, created atomically; no git, no daemon.

use crate::doc::{next_id, read, Project};
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

/// Removes the ledger lock when the allocation ends, even on error.
struct Guard(PathBuf);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn guard(state: &Path) -> io::Result<Guard> {
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
pub fn allocate(p: &Project, prefix: &str) -> io::Result<u64> {
    let _lock = guard(&p.state)?;
    let n = next_id(p, prefix);
    let mut ledger = read_ledger(&p.state);
    ledger.insert(prefix.to_string(), n);
    let body: Vec<String> = ledger.iter().map(|(k, v)| format!("  \"{k}\": {v}")).collect();
    fs::write(p.state.join("ledger.json"), format!("{{\n{}\n}}\n", body.join(",\n")))?;
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
