//! Operations that change the project. They return the lines to show; nothing here prints.

use crate::doc::{parse_list, read, set_field, slug, today, Doc, Project};
use crate::lint::{has_content, lint, Report};
use crate::scaffold::template;
use crate::state;
use std::fs;
use std::path::Path;

pub type Lines = Result<Vec<String>, String>;

/// `a, b` from the command line as a front matter list: `[a, b]`.
pub fn list_arg(v: Option<&str>) -> String {
    format!("[{}]", v.unwrap_or("").split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "))
}

fn write(path: &Path, text: String) -> Result<(), String> {
    fs::create_dir_all(path.parent().unwrap()).and_then(|_| fs::write(path, text)).map_err(|e| format!("{}: {e}", path.display()))
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

        task_start(&open(), "T-0001", "claude").unwrap();
        assert!(task_start(&open(), "PROJ-9", "codex").unwrap_err().contains("T-0001"));
        assert_eq!(open().by_id("task", "T-0002").unwrap().get("status"), "open");

        assert!(task_close(&open(), "T-0001").is_err()); // empty closure
        let task = open().by_id("task", "T-0001").unwrap().path.clone();
        fs::write(&task, read(&task).unwrap() + "Checked by hand.\n").unwrap();
        let (_, report) = task_close(&open(), "T-0001").unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(root.join("mwp/tasks/done").join(task.file_name().unwrap()).exists() && !task.exists());

        // The module is free again; a released task frees it too.
        task_start(&open(), "T-0002", "codex").unwrap();
        task_release(&open(), "T-0002", "blocked").unwrap();
        assert!(state::locks(&open().state).is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
