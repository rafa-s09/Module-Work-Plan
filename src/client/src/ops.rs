//! Operations that change the project. They return the lines to show; nothing here prints.

use crate::doc::{map_texts, parse_list, read, set_field, slug, today, Doc, Project};
use crate::lint::{has_content, lint, Report, MODULE_TYPES};
use crate::scaffold::template;
use crate::state;
use std::fs;
use std::path::{Path, PathBuf};

pub type Lines = Result<Vec<String>, String>;

/// `a, b` from the command line as a front matter list: `[a, b]`.
pub fn list_arg(v: Option<&str>) -> String {
    format!("[{}]", v.unwrap_or("").split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "))
}

fn write(path: &Path, text: String) -> Result<(), String> {
    fs::create_dir_all(path.parent().unwrap()).and_then(|_| fs::write(path, text)).map_err(|e| format!("{}: {e}", path.display()))
}

/// What an agent loads for a task (`mwp context`).
pub struct Context {
    /// Why each file is loaded (`always`, or the front matter field that cites it), the file and
    /// its size in tokens.
    pub files: Vec<(&'static str, PathBuf, u64)>,
    /// IDs the task cites that do not exist, as `ID (field)`.
    pub missing: Vec<String>,
}

impl Context {
    pub fn tokens(&self) -> u64 {
        self.files.iter().map(|f| f.2).sum()
    }
}

/// The constitution, the task, then what its front matter cites.
pub fn context(p: &Project, task: &Doc) -> Context {
    // shortcut: ~4 bytes per token, good enough to spot an oversized task.
    let tokens = |path: &Path| fs::metadata(path).map(|m| m.len() / 4).unwrap_or(0);
    let mut files: Vec<(&'static str, PathBuf, u64)> = [p.mwp.join("constitution.md"), task.path.clone()].into_iter().map(|f| ("always", f.clone(), tokens(&f))).collect();
    let mut missing = Vec::new();
    for (field, kind) in [("modules", "module"), ("reads", "module"), ("specs", "spec"), ("decisions", "adr"), ("refs", "task")] {
        for id in task.list(field) {
            match p.by_id(kind, &id) {
                Some(d) => files.push((field, d.path.clone(), tokens(&d.path))),
                None => missing.push(format!("{id} ({field})")),
            }
        }
    }
    Context { files, missing }
}

/// A task by ID or by company work code (`external:`).
pub fn find_task<'a>(p: &'a Project, key: &str) -> Result<&'a Doc, String> {
    p.of_kind("task").find(|t| t.id() == key || t.list("external").iter().any(|e| e == key))
        .ok_or(format!("no task with id or external code `{key}`"))
}

/// Creates an open task. `fields` are `(modules | reads | refs | external, "a,b")`.
pub fn task_new(p: &Project, title: &str, size: &str, fields: &[(&str, Option<&str>)]) -> Lines {
    let title = title.trim();
    if title.is_empty() {
        return Err("the task needs a title".into());
    }
    if !["S", "M", "L"].contains(&size) {
        return Err("--size must be S, M or L".into());
    }
    let mut lines = Vec::new();
    for (field, value) in fields {
        let kind = match *field {
            "modules" | "reads" => "module",
            "refs" => "task",
            _ => continue,
        };
        for id in parse_list(&list_arg(*value)) {
            if p.by_id(kind, &id).is_none() {
                lines.push(format!("warning: `{field}` cites `{id}`, no {kind} has that id"));
            }
        }
    }
    let id = format!("T-{:04}", state::allocate(p, "T").map_err(|e| e.to_string())?);
    let mut text = template(if size == "S" { "mwp/tasks/open/_template-task-s.md" } else { "mwp/tasks/open/_template-task.md" })
        .replace("id: <T-XXXX>", &format!("id: {id}"))
        .replace("# <T-XXXX> — <Title>", &format!("# {id} — {title}"));
    text = set_field(&set_field(&text, "size", size), "created", &today());
    for (field, value) in fields {
        let v = list_arg(*value);
        if v != "[]" {
            text = set_field(&text, field, &v);
        }
    }
    let path = p.mwp.join("tasks").join("open").join(format!("{id}-{}.md", slug(title)));
    write(&path, text)?;
    lines.push(format!("create {} (size {size}, status: open)", p.rel(&path)));
    Ok(lines)
}

/// Claims the task for one agent and locks the modules it writes.
pub fn task_start(p: &Project, key: &str, agent: &str) -> Lines {
    let t = find_task(p, key)?;
    let id = t.id();
    if t.get("status") == "done" {
        return Err(format!("{id} is done; rework is a new task with `refs: [{id}]`"));
    }
    let modules = t.list("modules");
    state::claim(&p.state, id, agent, &modules)?;
    let text = read(&t.path).ok_or("cannot read the task")?;
    if let Err(e) = write(&t.path, set_field(&set_field(&text, "status", "in_progress"), "agent", agent)) {
        state::release(&p.state, id);
        return Err(e);
    }
    let locked = if modules.is_empty() { "none".to_string() } else { modules.join(", ") };
    Ok(vec![format!("start {id} (agent: {agent}, modules locked: {locked})")])
}

/// Closes the task: `status: done`, moved to `tasks/done/`, locks freed. Returns the lint of the result.
pub fn task_close(p: &Project, key: &str) -> Result<(Vec<String>, Report), String> {
    let t = find_task(p, key)?;
    let id = t.id();
    if t.get("status") == "done" {
        return Err(format!("{id} is already done"));
    }
    if !t.section("Closure").is_some_and(has_content) {
        return Err(format!("{id}: fill `## Closure` with the evidence for every criterion before closing"));
    }
    let text = read(&t.path).ok_or("cannot read the task")?;
    let dest = p.mwp.join("tasks").join("done").join(t.path.file_name().unwrap());
    write(&dest, set_field(&text, "status", "done"))?;
    if dest != t.path {
        fs::remove_file(&t.path).map_err(|e| e.to_string())?;
    }
    state::release(&p.state, id);
    Ok((vec![format!("close {id} -> {}", p.rel(&dest))], lint(&Project::open(&p.root)?)))
}

/// Gives the task back without closing it: frees its locks and sets `open`, `blocked` or `review`.
pub fn task_release(p: &Project, key: &str, status: &str) -> Lines {
    if !["open", "blocked", "review"].contains(&status) {
        return Err("--status must be open, blocked or review".into());
    }
    let t = find_task(p, key)?;
    let id = t.id();
    if t.get("status") == "done" {
        return Err(format!("{id} is done"));
    }
    let text = read(&t.path).ok_or("cannot read the task")?;
    write(&t.path, set_field(&text, "status", status))?;
    state::release(&p.state, id);
    Ok(vec![format!("release {id} (status: {status}, locks freed)")])
}

/// Rewrites the table in `map.md` when the modules changed. Returns what it did (`warning: ...` on failure).
pub fn refresh_map(root: &Path) -> String {
    match Project::open(root).and_then(|p| map_texts(&p).map(|t| (p, t))) {
        Ok((p, (cur, new))) if cur != new => match fs::write(p.mwp.join("map.md"), new) {
            Ok(()) => "update mwp/map.md".into(),
            Err(e) => format!("warning: cannot write mwp/map.md: {e}"),
        },
        Ok(_) => "mwp/map.md unchanged".into(),
        Err(e) => format!("warning: {e}"),
    }
}

/// A folder under `mwp/<area>/` (`modules` or `content`), given as `a/b`; empty is the area itself.
/// Each part is lowercase words joined by `-`, so nothing can point outside the area.
fn area_dir(p: &Project, area: &str, folder: &str) -> Result<PathBuf, String> {
    let mut dir = p.mwp.join(area);
    for part in folder.split(['/', '\\']).filter(|s| !s.is_empty()) {
        if slug(part) != part {
            return Err(format!("folder `{part}`: use lowercase words joined by `-`"));
        }
        dir.push(part);
    }
    Ok(dir)
}

fn modules_dir(p: &Project, folder: &str) -> Result<PathBuf, String> {
    area_dir(p, "modules", folder)
}

fn folder_new(p: &Project, area: &str, parent: &str, name: &str) -> Lines {
    let name = name.trim();
    if name.is_empty() || slug(name) != name {
        return Err("the folder name is lowercase words joined by `-`, like `billing`".into());
    }
    let dir = area_dir(p, area, parent)?.join(name);
    if dir.exists() {
        return Err(format!("{} already exists", p.rel(&dir)));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", p.rel(&dir)))?;
    Ok(vec![format!("create {}/", p.rel(&dir))])
}

/// Creates a folder for modules inside `parent` (a folder under `mwp/modules/`, empty for the top).
pub fn module_folder_new(p: &Project, parent: &str, name: &str) -> Lines {
    folder_new(p, "modules", parent, name)
}

/// Creates a folder for resources inside `parent` (a folder under `mwp/content/`, empty for the top).
pub fn content_folder_new(p: &Project, parent: &str, name: &str) -> Lines {
    folder_new(p, "content", parent, name)
}

/// Copies files into `mwp/content/<folder>`. Never overwrites: a name already there is skipped
/// and reported (`skip ...`), so a resource is never replaced by accident.
pub fn content_add(p: &Project, folder: &str, files: &[PathBuf]) -> Lines {
    let dir = area_dir(p, "content", folder)?;
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", p.rel(&dir)))?;
    let mut lines = Vec::new();
    for f in files {
        let Some(name) = f.file_name().filter(|_| f.is_file()) else {
            lines.push(format!("skip {}: not a file", f.display()));
            continue;
        };
        let dest = dir.join(name);
        if dest.exists() {
            lines.push(format!("skip {}: already exists", p.rel(&dest)));
            continue;
        }
        fs::copy(f, &dest).map_err(|e| format!("{}: {e}", f.display()))?;
        lines.push(format!("add {}", p.rel(&dest)));
    }
    Ok(lines)
}

/// Creates a module from the template in `folder` (under `mwp/modules/`, empty for the top) and adds it
/// to `map.md`. IDs stay unique across folders.
pub fn module_new(p: &Project, folder: &str, id: &str, kind: &str, summary: &str) -> Lines {
    let (id, summary) = (id.trim(), summary.trim());
    if id.is_empty() || slug(id) != id {
        return Err("the module id is lowercase words joined by `-`, like `order` or `user-auth`".into());
    }
    if !MODULE_TYPES.contains(&kind) {
        return Err(format!("the module type must be one of: {}", MODULE_TYPES.join(", ")));
    }
    if summary.is_empty() {
        return Err("the module needs a one-line summary".into());
    }
    let path = modules_dir(p, folder)?.join(format!("{id}.md"));
    if p.by_id("module", id).is_some() || path.exists() {
        return Err(format!("module `{id}` already exists"));
    }
    let mut text = template("mwp/modules/_template.md").replace("# <Module name>", &format!("# {id}"));
    for (key, value) in [("id", id), ("type", kind), ("summary", summary), ("code", "[]"), ("tests", "[]"), ("updated", &today())] {
        text = set_field(&text, key, value);
    }
    write(&path, text)?;
    Ok(vec![format!("create {}", p.rel(&path)), refresh_map(&p.root)])
}

/// Creates an active SPEC. `owner` is the module or task it belongs to; `lifetime` is durable or transient.
pub fn spec_new(p: &Project, title: &str, owner: &str, lifetime: &str) -> Lines {
    let (title, owner) = (title.trim(), owner.trim());
    if title.is_empty() {
        return Err("the SPEC needs a title".into());
    }
    if !["durable", "transient"].contains(&lifetime) {
        return Err("the SPEC lifetime must be durable or transient".into());
    }
    if p.by_id("module", owner).is_none() && p.by_id("task", owner).is_none() {
        return Err(format!("owner `{owner}` is not a module or task"));
    }
    let id = format!("SP-{:04}", state::allocate(p, "SP").map_err(|e| e.to_string())?);
    let mut text = template("mwp/specs/_template.md")
        .replace("id: <SP-XXXX>", &format!("id: {id}"))
        .replace("# <SP-XXXX> — <Title>", &format!("# {id} — {title}"));
    for (key, value) in [("lifetime", lifetime), ("owner", owner), ("created", &today()), ("updated", &today())] {
        text = set_field(&text, key, value);
    }
    let path = p.mwp.join("specs").join(format!("{id}-{}.md", slug(title)));
    write(&path, text)?;
    Ok(vec![format!("create {} (status: active, owner: {owner})", p.rel(&path))])
}

/// What may be deleted: modules, specs, tasks not done yet and content files. Templates (`_*`) stay.
pub fn deletable(p: &Project, path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if path.starts_with(p.mwp.join("content")) {
        return path.is_file() && name != ".gitkeep"; // resources of any type
    }
    name.ends_with(".md") && !name.starts_with('_') && ["modules", "specs", "tasks/open"].iter().any(|d| path.starts_with(p.mwp.join(d)))
}

/// Deletes a module, a spec or an open task. A task's claim and locks are freed; `map.md` follows the modules.
pub fn delete(p: &Project, path: &Path) -> Lines {
    if !deletable(p, path) {
        return Err(format!("{} cannot be deleted: only modules, specs, open tasks and content files can", p.rel(path)));
    }
    let doc = p.docs.iter().find(|d| d.path == path);
    fs::remove_file(path).map_err(|e| format!("{}: {e}", p.rel(path)))?;
    let mut lines = vec![format!("delete {}", p.rel(path))];
    match doc.map(|d| (d.kind(), d.id())) {
        Some(("task", id)) if !id.is_empty() => state::release(&p.state, id),
        Some(("module", _)) => lines.push(refresh_map(&p.root)),
        _ => {}
    }
    Ok(lines)
}

/// Creates a proposed ADR and, with `supersedes`, marks the old one superseded.
pub fn adr_new(p: &Project, title: &str, modules: Option<&str>, supersedes: Option<&str>) -> Lines {
    let title = title.trim();
    if title.is_empty() {
        return Err("the ADR needs a title".into());
    }
    let old = match supersedes {
        Some(id) => Some(p.by_id("adr", id).ok_or(format!("no adr `{id}`"))?),
        None => None,
    };
    let id = format!("ADR-{:04}", state::allocate(p, "ADR").map_err(|e| e.to_string())?);
    let date = today();
    let text = template("mwp/decisions/_template.md")
        .replace("id: <ADR-XXXX>", &format!("id: {id}"))
        .replace("# <ADR-XXXX> — <Title>", &format!("# {id} — {title}"))
        .replace("<YYYY-MM-DD>", &date)
        .replace("modules: []", &format!("modules: {}", list_arg(modules)))
        .replace("supersedes: []", &format!("supersedes: {}", list_arg(supersedes)));
    let path = p.mwp.join("decisions").join(format!("{id}-{}.md", slug(title)));
    write(&path, text)?;
    let mut lines = vec![format!("create {} (status: proposed)", p.rel(&path))];
    if let Some(old) = old {
        let t = read(&old.path).ok_or("cannot read the superseded ADR")?;
        write(&old.path, set_field(&set_field(&set_field(&t, "status", "superseded"), "superseded_by", &id), "date", &date))?;
        lines.push(format!("update {} (status: superseded, superseded_by: {id})", p.rel(&old.path)));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::map_texts;
    use crate::scaffold::{init, parse_agents};

    #[test]
    fn two_agents_one_project() {
        let root = std::env::temp_dir().join(format!("mwp-ops-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        init(&root, Some("Demo"), &parse_agents("claude,codex").unwrap(), true).unwrap();
        for f in ["CLAUDE.md", "AGENTS.md", ".claude/skills/mwp/SKILL.md", ".agents/skills/module/SKILL.md", ".gitignore"] {
            assert!(root.join(f).exists(), "{f}");
        }
        let open = || Project::open(&root).unwrap();
        fs::write(root.join("mwp/modules/order.md"), "---\nkind: module\nid: order\ntype: domain\nsummary: Orders\nupdated: 2026-01-01\n---\n# order\n").unwrap();
        fs::write(root.join("mwp/map.md"), map_texts(&open()).unwrap().1).unwrap();

        task_new(&open(), "First", "S", &[("modules", Some("order"))]).unwrap();
        task_new(&open(), "Second", "M", &[("modules", Some("order")), ("external", Some("PROJ-9"))]).unwrap();
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("size"), "M");
        // Context: constitution, task, cited module; a cited ID that does not exist is missing.
        task_new(&open(), "Third", "S", &[("modules", Some("order")), ("refs", Some("T-0404"))]).unwrap();
        let p = open();
        let c = context(&p, p.by_id("task", "T-0003").unwrap());
        let files: Vec<(&str, String)> = c.files.iter().map(|f| (f.0, p.rel(&f.1))).collect();
        assert_eq!(files[0], ("always", "mwp/constitution.md".into()));
        assert_eq!((files[1].0, files[2].clone()), ("always", ("modules", "mwp/modules/order.md".into())));
        assert_eq!(c.missing, ["T-0404 (refs)"]);
        assert!(c.tokens() > 0 && c.tokens() == c.files.iter().map(|f| f.2).sum::<u64>());
        delete(&p, &p.by_id("task", "T-0003").unwrap().path).unwrap();

        task_start(&open(), "T-0001", "claude").unwrap();
        assert!(task_start(&open(), "PROJ-9", "codex").unwrap_err().contains("T-0001"));
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("status"), "open");

        assert!(task_close(&open(), "T-0001").is_err()); // empty closure

        // A task without front matter is reported, not silently dropped.
        let broken = root.join("mwp/tasks/open/T-0009-broken.md");
        fs::write(&broken, "# T-0009\n").unwrap();
        assert!(lint(&open()).errors.iter().any(|e| e.contains("T-0009-broken.md")));
        fs::remove_file(&broken).unwrap();

        // An edited skill is restored; only installed agents are touched.
        let skill = root.join(".agents/skills/mwp/SKILL.md");
        fs::write(&skill, "old").unwrap();
        let agents = crate::scaffold::installed(&root);
        assert_eq!(agents.iter().map(|a| a.id).collect::<Vec<_>>(), ["claude", "codex"]);
        assert_eq!(crate::scaffold::update_skills(&root, &agents).unwrap().iter().filter(|l| l.starts_with("update")).count(), 1);
        assert!(read(&skill).unwrap().starts_with("---"));
        let task = open().by_id("task", "T-0001").unwrap().path.clone();
        fs::write(&task, read(&task).unwrap() + "Checked by hand.\n").unwrap();
        let (_, report) = task_close(&open(), "T-0001").unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(root.join("mwp/tasks/done").join(task.file_name().unwrap()).exists() && !task.exists());

        // The module is free again; a released task frees it too.
        task_start(&open(), "T-0002", "codex").unwrap();
        task_release(&open(), "T-0002", "blocked").unwrap();
        assert!(state::locks(&open().state).is_empty());

        // A new module lands in map.md; deleting it takes it out again.
        module_new(&open(), "", "user-auth", "domain", "Sign in").unwrap();
        assert!(module_new(&open(), "", "user-auth", "domain", "x").is_err() && module_new(&open(), "", "Bad Id", "domain", "x").is_err());

        // Modules in subfolders: found by the core and in map.md; IDs stay unique across folders.
        module_folder_new(&open(), "", "billing").unwrap();
        module_folder_new(&open(), "billing", "tax").unwrap();
        assert!(module_folder_new(&open(), "", "billing").is_err());
        for bad in ["..", "Bad Name", "a/b"] {
            assert!(module_folder_new(&open(), "", bad).is_err(), "{bad}");
        }
        assert!(module_new(&open(), "../x", "escape", "domain", "x").is_err());
        assert!(module_new(&open(), "billing/tax", "vat", "service", "x").is_err());
        module_new(&open(), "billing/tax", "vat", "api", "VAT rules").unwrap();
        assert!(root.join("mwp/modules/billing/tax/vat.md").exists() && open().by_id("module", "vat").is_some());
        assert!(module_new(&open(), "billing", "vat", "domain", "x").is_err());
        assert!(read(&root.join("mwp/map.md")).unwrap().contains("| vat | api | VAT rules |"));
        assert!(!lint(&open()).warnings.iter().any(|w| w.contains("vat")), "{:?}", lint(&open()).warnings);
        delete(&open(), &root.join("mwp/modules/billing/tax/vat.md")).unwrap();
        fs::remove_dir_all(root.join("mwp/modules/billing")).unwrap();

        // Content: files of any type are copied in, never over an existing one, and can be deleted.
        let src = root.join("logo.png");
        fs::write(&src, [137u8, 80, 78, 71]).unwrap();
        content_folder_new(&open(), "", "icons").unwrap();
        assert!(content_folder_new(&open(), "", "../x").is_err());
        let lines = content_add(&open(), "icons", &[src.clone(), root.join("missing.png")]).unwrap();
        assert_eq!(lines, ["add mwp/content/icons/logo.png", &format!("skip {}: not a file", root.join("missing.png").display())]);
        fs::write(&src, b"new").unwrap();
        assert_eq!(content_add(&open(), "icons", &[src]).unwrap(), ["skip mwp/content/icons/logo.png: already exists"]);
        let logo = root.join("mwp/content/icons/logo.png");
        assert_eq!(fs::read(&logo).unwrap(), [137u8, 80, 78, 71], "never overwritten");
        assert!(!deletable(&open(), &root.join("mwp/content/.gitkeep")));
        delete(&open(), &logo).unwrap();
        assert!(!logo.exists());
        assert!(read(&root.join("mwp/map.md")).unwrap().contains("| user-auth | domain | Sign in |"));
        assert!(!lint(&open()).errors.iter().any(|e| e.contains("map.md") || e.contains("user-auth")), "{:?}", lint(&open()).errors);
        // A SPEC needs an owner that exists; the new one passes lint.
        assert!(spec_new(&open(), "Session rules", "nobody", "durable").is_err());
        spec_new(&open(), "Session rules", "user-auth", "durable").unwrap();
        let sp = open().by_id("spec", "SP-0001").unwrap().path.clone();
        assert!(sp.ends_with("specs/SP-0001-session-rules.md") && open().by_id("spec", "SP-0001").unwrap().get("owner") == "user-auth");
        assert!(!lint(&open()).errors.iter().any(|e| e.contains("SP-0001")), "{:?}", lint(&open()).errors);
        delete(&open(), &sp).unwrap();
        delete(&open(), &root.join("mwp/modules/user-auth.md")).unwrap();
        assert!(!read(&root.join("mwp/map.md")).unwrap().contains("user-auth"));

        // Deleting an open task frees its claim and locks; done tasks and project files are kept.
        task_start(&open(), "T-0002", "codex").unwrap();
        let t2 = open().by_id("task", "T-0002").unwrap().path.clone();
        delete(&open(), &t2).unwrap();
        assert!(!t2.exists() && state::claims(&open().state).is_empty() && state::locks(&open().state).is_empty());
        for kept in ["mwp/constitution.md", "mwp/modules/_template.md"] {
            assert!(delete(&open(), &root.join(kept)).is_err(), "{kept}");
        }
        let done = root.join("mwp/tasks/done").join(task.file_name().unwrap());
        assert!(delete(&open(), &done).is_err() && done.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
