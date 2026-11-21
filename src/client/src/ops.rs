//! Operations that change the project. They return the lines to show; nothing here prints.

use crate::doc::{map_texts, parse_list, read, set_field, slug, today, Doc, Project};
use crate::lint::{closed, field_filled, has_content, lint, needs_plan, Report, MODULE_TYPES, SIZES};
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

/// Sets a task's `updated:` to today: the date of its last change of status (or plan approval)
/// made through `mwp` or the GUI. An edit by hand does not move it.
fn stamp(text: &str) -> String {
    set_field(text, "updated", &today())
}

/// Creates an open task. `fields` are `(modules | reads | refs | external, "a,b")`.
pub fn task_new(p: &Project, title: &str, size: &str, fields: &[(&str, Option<&str>)]) -> Lines {
    let title = title.trim();
    if title.is_empty() {
        return Err("the task needs a title".into());
    }
    if !SIZES.contains(&size) {
        return Err(format!("--size must be one of {}", SIZES.join(", ")));
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
    let id = format!("T-{:04}", state::allocate(p, "T")?);
    let mut text = template(if matches!(size, "XS" | "S") { "mwp/tasks/open/_template-task-s.md" } else { "mwp/tasks/open/_template-task.md" })?
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
    if closed(t.get("status")) {
        return Err(format!("{id} is {}; rework is a new task with `refs: [{id}]`", t.get("status")));
    }
    let modules = t.list("modules");
    state::claim(&p.state, id, agent, &modules)?;
    let text = read(&t.path).ok_or("cannot read the task")?;
    if let Err(e) = write(&t.path, stamp(&set_field(&set_field(&text, "status", "in_progress"), "agent", agent))) {
        state::release(&p.state, id);
        return Err(e);
    }
    let locked = if modules.is_empty() { "none".to_string() } else { modules.join(", ") };
    Ok(vec![format!("start {id} (agent: {agent}, modules locked: {locked})")])
}

/// Closes the task: `status: done` (or `replanned`), moved to `tasks/done/`, locks freed. Returns
/// the lint of the result. A replanned task gave up its plan: its `**Replanning:**` line says why.
pub fn task_close(p: &Project, key: &str, replanned: bool) -> Result<(Vec<String>, Report), String> {
    let t = find_task(p, key)?;
    let id = t.id();
    if closed(t.get("status")) {
        return Err(format!("{id} is already {}", t.get("status")));
    }
    if !t.section("Closure").is_some_and(has_content) {
        return Err(format!("{id}: fill `## Closure` with the evidence for every criterion before closing"));
    }
    if !replanned && needs_plan(t.get("size")) && t.get("plan_approved").is_empty() {
        return Err(format!("{id}: size {} needs `plan_approved:`; a person approves `## Plan` first", t.get("size")));
    }
    if replanned && !t.section("Closure").is_some_and(|c| field_filled(c, "Replanning")) {
        return Err(format!("{id}: fill `**Replanning:**` in `## Closure` with why the plan failed and the new tasks"));
    }
    let text = read(&t.path).ok_or("cannot read the task")?;
    let dest = p.mwp.join("tasks").join("done").join(t.path.file_name().unwrap());
    write(&dest, set_field(&stamp(&set_field(&text, "status", if replanned { "replanned" } else { "done" })), "closed", &today()))?;
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
    if closed(t.get("status")) {
        return Err(format!("{id} is {}", t.get("status")));
    }
    let text = read(&t.path).ok_or("cannot read the task")?;
    write(&t.path, stamp(&set_field(&text, "status", status)))?;
    state::release(&p.state, id);
    Ok(vec![format!("release {id} (status: {status}, locks freed)")])
}

/// Brings a project written for an older mwp up to this one. Only mechanical changes:
/// `mwp_version` and `created:` in PROJECT.md, the front matter of the constitution, map and
/// glossary, the `_template` files (the same in every package), and the agents' skills. The
/// constitution's text is the project's own: sections of the current rules it lacks are reported,
/// never written. Each file keeps its line endings. `dry_run` writes nothing.
pub fn upgrade(p: &Project, dry_run: bool) -> Lines {
    use crate::scaffold::{installed, project_files, update_skills, DEFAULT, VERSION};
    let raw = |path: &Path| fs::read_to_string(path).ok();
    let plain = |s: &str| s.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    // A file written with CRLF stays CRLF.
    let eol = |old: &str, new: String| if old.contains("\r\n") { new.replace("\r\n", "\n").replace('\n', "\r\n") } else { new };
    let mut changes: Vec<(PathBuf, String)> = Vec::new();
    let mut lines = Vec::new();

    let project = p.mwp.join("PROJECT.md");
    let old = raw(&project).ok_or("mwp/PROJECT.md not found")?;
    let text = plain(&old);
    let mut new = set_field(&text, "mwp_version", VERSION);
    if !crate::doc::split_front_matter(&text).is_some_and(|(fm, _)| fm.lines().any(|l| l.starts_with("created:"))) {
        // The earliest task is the best record of when the project began.
        let created = p.of_kind("task").map(|t| t.get("created").to_string()).filter(|d| crate::lint::is_date(d)).min().unwrap_or_else(today);
        new = set_field(&new, "created", &created);
    }
    if new != text {
        changes.push((project, eol(&old, new)));
    }

    for (file, kind) in [("constitution.md", "constitution"), ("map.md", "map"), ("glossary.md", "glossary")] {
        let path = p.mwp.join(file);
        let Some(old) = raw(&path) else { continue };
        let text = plain(&old);
        let new = match crate::doc::split_front_matter(&text) {
            None if kind == "constitution" => format!("---\nkind: {kind}\nupdated: {}\n---\n\n{text}", today()),
            None => format!("---\nkind: {kind}\n---\n\n{text}"),
            Some((fm, _)) if kind == "constitution" && !fm.lines().any(|l| l.starts_with("updated:")) => set_field(&text, "updated", &today()),
            Some(_) => continue,
        };
        changes.push((path, eol(&old, new)));
    }

    let current = project_files(DEFAULT)?;
    for (rel, content) in current.iter().filter(|(rel, _)| rel.rsplit('/').next().is_some_and(|f| f.starts_with("_template"))) {
        let path = p.root.join(rel);
        match raw(&path) {
            Some(old) if plain(&old) == plain(content) => {}
            Some(old) => changes.push((path, eol(&old, content.clone()))),
            None => changes.push((path, content.clone())),
        }
    }

    let mine = read(&p.mwp.join("constitution.md")).unwrap_or_default();
    // `## 10. Task sizes`, `## Task sizes` and `## Task sizes (rules)` are the same section.
    let heading = |l: &str| l.strip_prefix("## ").map(|h| {
        let h = h.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
        h.split(" (").next().unwrap_or(h).trim().to_lowercase()
    });
    let have: Vec<String> = mine.lines().filter_map(heading).collect();
    let rules = current.get("mwp/constitution.md").map(|c| plain(c)).unwrap_or_default();
    for l in rules.lines() {
        if heading(l).is_some_and(|h| !have.contains(&h)) {
            lines.push(format!("review mwp/constitution.md: the current rules have `{l}`; yours has no such section"));
        }
    }

    let mut done: Vec<String> = Vec::new();
    for (path, text) in &changes {
        done.push(format!("{} {}", if path.exists() { "update" } else { "create" }, p.rel(path)));
        if !dry_run {
            write(path, text.clone())?;
        }
    }
    done.extend(lines);
    let agents = installed(&p.root);
    if dry_run {
        done.push("dry run: nothing written; the skills are checked when it runs".into());
    } else if !agents.is_empty() {
        done.extend(update_skills(&p.root, &agents)?);
    }
    done.push(format!("\nmwp_version: {VERSION}. The constitution is yours: compare it with this version's rules by hand."));
    Ok(done)
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
    let mut text = template("mwp/modules/_template.md")?.replace("# <Module name>", &format!("# {id}"));
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
    let id = format!("SP-{:04}", state::allocate(p, "SP")?);
    let mut text = template("mwp/specs/_template.md")?
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

/// What can be renamed or moved: the files that can be deleted, and the folders under
/// `mwp/modules/` and `mwp/content/`. ADRs and done tasks are records and stay where they are.
pub fn movable(p: &Project, path: &Path) -> bool {
    let in_area = ["modules", "content"].iter().any(|a| path.starts_with(p.mwp.join(a)) && path != p.mwp.join(a));
    deletable(p, path) || (path.is_dir() && in_area)
}

/// Renames a file or folder in place. A doc keeps `.md` (added when left out); a folder is a slug.
/// Renaming a module's file does not change its ID.
pub fn rename(p: &Project, path: &Path, name: &str) -> Lines {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err("the name cannot be empty or hold `/`".into());
    }
    let in_content = path.starts_with(p.mwp.join("content"));
    let name = if path.is_dir() {
        if slug(name) != name {
            return Err("the folder name is lowercase words joined by `-`, like `billing`".into());
        }
        name.to_string()
    } else if in_content {
        name.to_string()
    } else {
        if name.starts_with('_') {
            return Err("names starting with `_` are templates".into());
        }
        if name.ends_with(".md") { name.to_string() } else { format!("{name}.md") }
    };
    relocate(p, path, &path.with_file_name(name))
}

/// Moves a file or folder into `dir`, a folder of the same area (`mwp/modules/` or `mwp/content/`).
pub fn move_into(p: &Project, path: &Path, dir: &Path) -> Lines {
    let area = ["modules", "content"].iter().map(|a| p.mwp.join(a)).find(|a| path.starts_with(a));
    match area {
        Some(a) if dir.starts_with(&a) && dir.is_dir() => {}
        _ => return Err("files move only between folders of Modules, or of Content".into()),
    }
    if dir.starts_with(path) {
        return Err("a folder cannot move into itself".into());
    }
    relocate(p, path, &dir.join(path.file_name().ok_or("nothing to move")?))
}

/// Renames `from` to `to`, never over an existing file.
fn relocate(p: &Project, from: &Path, to: &Path) -> Lines {
    if !movable(p, from) {
        return Err(format!("{} cannot be renamed or moved", p.rel(from)));
    }
    if to == from {
        return Ok(Vec::new());
    }
    // Case-only renames on Windows: the target "exists" because it is the same file.
    if to.exists() && !to.to_string_lossy().eq_ignore_ascii_case(&from.to_string_lossy()) {
        return Err(format!("{} already exists", p.rel(to)));
    }
    fs::rename(from, to).map_err(|e| format!("{}: {e}", p.rel(from)))?;
    Ok(vec![format!("move {} -> {}", p.rel(from), p.rel(to))])
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
    let id = format!("ADR-{:04}", state::allocate(p, "ADR")?);
    let date = today();
    let text = template("mwp/decisions/_template.md")?
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

/// A person approves the `## Plan` of an L, XL or XXL task: sets `plan_approved:` to today.
/// Like deciding an ADR, only a person does it, so the CLI agents use has no such command.
pub fn plan_approve(p: &Project, key: &str) -> Lines {
    let t = find_task(p, key)?;
    let id = t.id();
    if !needs_plan(t.get("size")) {
        return Err(format!("{id} is size {}: only L, XL and XXL need an approved plan", t.get("size")));
    }
    if closed(t.get("status")) {
        return Err(format!("{id} is {}", t.get("status")));
    }
    if !t.section("Plan").is_some_and(has_content) {
        return Err(format!("{id}: `## Plan` is empty; the agent writes it first"));
    }
    let text = read(&t.path).ok_or("cannot read the task")?;
    write(&t.path, stamp(&set_field(&text, "plan_approved", &today())))?;
    Ok(vec![format!("update {} (plan_approved: {})", p.rel(&t.path), today())])
}

/// A person accepts or rejects a proposed ADR: sets `status` and `date`. The body is not touched.
pub fn adr_decide(p: &Project, id: &str, status: &str) -> Lines {
    if !matches!(status, "accepted" | "rejected") {
        return Err("an ADR is accepted or rejected".into());
    }
    let adr = p.by_id("adr", id).ok_or(format!("no adr `{id}`"))?;
    if adr.get("status") != "proposed" {
        return Err(format!("{id} is {}, not proposed", adr.get("status")));
    }
    let t = read(&adr.path).ok_or(format!("cannot read {id}"))?;
    write(&adr.path, set_field(&set_field(&t, "status", status), "date", &today()))?;
    Ok(vec![format!("update {} (status: {status})", p.rel(&adr.path))])
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
        init(&root, Some("Demo"), &parse_agents("claude,codex").unwrap(), true, crate::scaffold::DEFAULT).unwrap();
        for f in ["CLAUDE.md", "AGENTS.md", ".claude/skills/mwp/SKILL.md", ".agents/skills/module/SKILL.md", ".gitignore"] {
            assert!(root.join(f).exists(), "{f}");
        }
        let open = || Project::open(&root).unwrap();
        fs::write(root.join("mwp/modules/order.md"), "---\nkind: module\nid: order\ntype: domain\nsummary: Orders\ncode: [src/order.rs]\ntests: [tests/order]\nupdated: 2026-01-01\n---\n# order\n\n## Tests\n| ID | Rule | Scenario | File |\n|---|---|---|---|\n| TEST-0001 | FR-01 | total | tests/order |\n").unwrap();
        fs::write(root.join("mwp/map.md"), map_texts(&open()).unwrap().1).unwrap();

        task_new(&open(), "First", "S", &[("modules", Some("order"))]).unwrap();
        task_new(&open(), "Second", "M", &[("modules", Some("order")), ("external", Some("PROJ-9"))]).unwrap();
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("size"), "M");
        // Code and tests not written yet are warnings, not errors.
        let r = lint(&open());
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.iter().any(|w| w.contains("src/order.rs")) && r.warnings.iter().any(|w| w.contains("TEST-0001")));
        fs::create_dir_all(root.join("tests/order")).unwrap();
        assert!(lint(&open()).errors.iter().any(|e| e.contains("TEST-0001")), "a test missing from an existing folder is an error");
        fs::write(root.join("tests/order/total.rs"), "// TEST-0001: total
").unwrap();
        // The top files carry their kind; a missing header is a warning, a wrong one an error.
        assert!(!r.warnings.iter().any(|w| w.contains("front matter") || w.contains("created")), "{:?}", r.warnings);
        let map = root.join("mwp/map.md");
        let map_text = read(&map).unwrap();
        fs::write(&map, format!("{map_text}
{}
{}
", crate::doc::MAP_BEGIN, crate::doc::MAP_END)).unwrap();
        assert!(lint(&open()).errors.iter().any(|e| e.contains("more than once")));
        fs::write(&map, &map_text).unwrap();
        let glossary = root.join("mwp/glossary.md");
        let glossary_text = read(&glossary).unwrap();
        fs::write(&glossary, glossary_text.replace("kind: glossary", "kind: map")).unwrap();
        assert!(lint(&open()).errors.iter().any(|e| e.contains("use `kind: glossary`")));
        fs::write(&glossary, glossary_text.split_once("---

").map_or(glossary_text.clone(), |(_, body)| body.to_string())).unwrap();
        assert!(lint(&open()).warnings.iter().any(|w| w.contains("glossary.md: no front matter")));
        fs::write(&glossary, &glossary_text).unwrap();
        // Six sizes; XS uses the short template like S.
        assert!(task_new(&open(), "Huge", "XXXL", &[]).unwrap_err().contains("XXL"));
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

        // A proposed ADR is accepted (or rejected) once; the body stays as it was.
        adr_new(&open(), "Use Postgres", Some("order"), None).unwrap();
        let body = |p: &Project| crate::doc::split_front_matter(&read(&p.by_id("adr", "ADR-0001").unwrap().path).unwrap()).unwrap().1.to_string();
        let before = body(&open());
        assert!(adr_decide(&open(), "ADR-0001", "superseded").is_err());
        adr_decide(&open(), "ADR-0001", "accepted").unwrap();
        assert_eq!(open().by_id("adr", "ADR-0001").unwrap().get("status"), "accepted");
        assert_eq!(body(&open()), before);
        assert!(adr_decide(&open(), "ADR-0001", "rejected").unwrap_err().contains("not proposed"));
        assert!(adr_decide(&open(), "ADR-0404", "accepted").is_err());

        task_start(&open(), "T-0001", "claude").unwrap();
        assert!(task_start(&open(), "PROJ-9", "codex").unwrap_err().contains("T-0001"));
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("status"), "open");

        assert!(task_close(&open(), "T-0001", false).is_err()); // empty closure

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
        let (_, report) = task_close(&open(), "T-0001", false).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(root.join("mwp/tasks/done").join(task.file_name().unwrap()).exists() && !task.exists());

        // The module is free again; a released task frees it too.
        task_start(&open(), "T-0002", "codex").unwrap();
        task_release(&open(), "T-0002", "blocked").unwrap();
        assert!(state::locks(&open().state).is_empty());
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("updated"), today());

        // An L task closes only with its plan approved, and only a written plan can be approved.
        task_new(&open(), "Big one", "L", &[("modules", Some("order"))]).unwrap();
        let big = open().of_kind("task").find(|t| t.title() == "Big one").unwrap().id().to_string();
        task_start(&open(), &big, "claude").unwrap();
        assert!(lint(&open()).warnings.iter().any(|w| w.contains("plan_approved")));
        let big_path = open().by_id("task", &big).unwrap().path.clone();
        fs::write(&big_path, read(&big_path).unwrap().replace("- **Done:**", "- **Done:** all of it")).unwrap();
        assert!(task_close(&open(), &big, false).err().unwrap().contains("plan_approved"));
        assert!(plan_approve(&open(), &big).unwrap_err().contains("Plan"));
        assert!(plan_approve(&open(), "T-0002").unwrap_err().contains("size M"));
        fs::write(&big_path, read(&big_path).unwrap().replace("## Plan
", "## Plan
1. Do it.
")).unwrap();
        plan_approve(&open(), &big).unwrap();
        assert_eq!(open().by_id("task", &big).unwrap().get("plan_approved"), today());
        task_close(&open(), &big, false).unwrap();

        // Upgrade: an old project (no headers, old version) is brought up once; a second run changes nothing.
        let old_project = root.join("mwp/PROJECT.md");
        fs::write(&old_project, read(&old_project).unwrap().replace(&format!("mwp_version: {}", crate::scaffold::VERSION), "mwp_version: 2")).unwrap();
        let glossary = root.join("mwp/glossary.md");
        let body = read(&glossary).unwrap().split_once("---\n\n").unwrap().1.replace('\n', "\r\n");
        fs::write(&glossary, &body).unwrap();
        assert!(lint(&open()).warnings.iter().any(|w| w.contains("run `mwp upgrade`")));
        let dry = upgrade(&open(), true).unwrap();
        assert!(dry.iter().any(|l| l == "update mwp/glossary.md") && fs::read_to_string(&glossary).unwrap() == body, "{dry:?}");
        upgrade(&open(), false).unwrap();
        let after = fs::read_to_string(&glossary).unwrap();
        assert!(after.starts_with("---\r\nkind: glossary\r\n") && after.ends_with(&body), "CRLF is kept: {after:?}");
        assert!(!lint(&open()).warnings.iter().any(|w| w.contains("upgrade") || w.contains("front matter")));
        assert!(!upgrade(&open(), false).unwrap().iter().any(|l| l.starts_with("update mwp/")), "a second run changes nothing");

        // Replanning: closed as replanned only with `**Replanning:**` filled; then it is closed for good.
        let second = open().by_id("task", "T-0002").unwrap().path.clone();
        fs::write(&second, read(&second).unwrap().replace("- **Done:**", "- **Done:** half of it")).unwrap();
        assert!(task_close(&open(), "T-0002", true).err().unwrap().contains("Replanning"));
        fs::write(&second, read(&second).unwrap() + "- **Replanning:** too big; split into T-0004 and T-0005.
").unwrap();
        let (_, report) = task_close(&open(), "T-0002", true).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(report.warnings.iter().any(|w| w.contains("no criterion is marked")), "a replanned task names its failed criteria");
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("status"), "replanned");
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("closed"), today());
        assert!(task_start(&open(), "T-0002", "codex").unwrap_err().contains("replanned"));
        task_new(&open(), "Smaller", "XS", &[("refs", Some("T-0002"))]).unwrap();
        let small = open();
        let small = small.of_kind("task").find(|t| t.title() == "Smaller").unwrap();
        assert!(small.get("size") == "XS" && small.section("Plan").is_none(), "XS uses the short template");
        assert!(lint(&open()).errors.is_empty(), "a replanned task can be cited in refs");

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
        // Rename and move: the ID stays; never over a file; only inside Modules or Content.
        let vat = root.join("mwp/modules/billing/tax/vat.md");
        assert!(rename(&open(), &vat, "_x").is_err() && rename(&open(), &vat, "a/b").is_err());
        rename(&open(), &vat, "value-added").unwrap();
        let renamed = root.join("mwp/modules/billing/tax/value-added.md");
        assert!(renamed.exists() && open().by_id("module", "vat").unwrap().path == renamed);
        assert!(rename(&open(), &root.join("mwp/modules/billing/tax"), "Bad Name").is_err());
        assert!(move_into(&open(), &root.join("mwp/modules/billing"), &root.join("mwp/modules/billing/tax")).is_err(), "not into itself");
        assert!(move_into(&open(), &renamed, &root.join("mwp/specs")).is_err(), "not out of Modules");
        move_into(&open(), &renamed, &root.join("mwp/modules/billing")).unwrap();
        let moved = root.join("mwp/modules/billing/value-added.md");
        fs::write(root.join("mwp/modules/billing/tax/value-added.md"), "x").unwrap();
        assert!(move_into(&open(), &moved, &root.join("mwp/modules/billing/tax")).unwrap_err().contains("already exists"));
        fs::remove_file(root.join("mwp/modules/billing/tax/value-added.md")).unwrap();
        assert!(!movable(&open(), &root.join("mwp/modules")) && !movable(&open(), &root.join("mwp/constitution.md")));
        delete(&open(), &moved).unwrap();
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
        let small = open().of_kind("task").find(|t| t.title() == "Smaller").unwrap().id().to_string();
        task_start(&open(), &small, "codex").unwrap();
        let t2 = open().by_id("task", &small).unwrap().path.clone();
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
