mod doc;
mod lint;

use doc::{id_num, map_texts, next_id, render_map, reserve, set_field, slug, today, Doc, Project, PREFIXES};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

// shortcut: templates are embedded from the repo, so `cargo install` must run from a checkout.
const TEMPLATES: &[(&str, &str)] = &[
    ("AGENTS.md", include_str!("../../../templates/AGENTS.md")),
    ("CLAUDE.md", include_str!("../../../templates/CLAUDE.md")),
    ("mwp/PROJECT.md", include_str!("../../../templates/mwp/PROJECT.md")),
    ("mwp/constitution.md", include_str!("../../../templates/mwp/constitution.md")),
    ("mwp/map.md", include_str!("../../../templates/mwp/map.md")),
    ("mwp/glossary.md", include_str!("../../../templates/mwp/glossary.md")),
    ("mwp/modules/README.md", include_str!("../../../templates/mwp/modules/README.md")),
    ("mwp/modules/_template.md", include_str!("../../../templates/mwp/modules/_template.md")),
    ("mwp/tasks/open/.gitkeep", ""),
    ("mwp/tasks/open/_template-task.md", include_str!("../../../templates/mwp/tasks/open/_template-task.md")),
    ("mwp/tasks/open/_template-task-s.md", include_str!("../../../templates/mwp/tasks/open/_template-task-s.md")),
    ("mwp/tasks/done/.gitkeep", ""),
    ("mwp/specs/_template.md", include_str!("../../../templates/mwp/specs/_template.md")),
    ("mwp/decisions/_template.md", include_str!("../../../templates/mwp/decisions/_template.md")),
    (".claude/skills/mwp/SKILL.md", include_str!("../../skill/SKILL.md")),
];

const USAGE: &str = "mwp — Module Work Plan

Usage: mwp <command> [options]

  init [dir] [--name <name>] [--no-skill]   Create mwp/, AGENTS.md, CLAUDE.md and the skill; never overwrites
  lint [--strict]                           Check the project; --strict also fails on warnings
  map [--write | --check]                   Print the module map; --write updates map.md, --check fails if stale
  remap [--dry-run]                         Recheck modules (paths, dependencies, tests) and update map.md
  task new <title> [--size S|M|L] [--modules a,b] [--reads a,b] [--refs T-XXXX] [--external CODE]
                                            Create an open task from the template with the next T- ID
  task-list [--status <s>] [--module <id>] [--all]
                                            List tasks (open by default; --all includes done)
  next <T|SP|ADR|TEST> [--reserve]          Next free ID; --reserve records it in .mwp/ledger.json
  context <T-XXXX | external code> [--paths]
                                            Files an agent reads for a task, with token estimate
  adr new <title> [--modules a,b] [--supersedes ADR-XXXX]
                                            Create a proposed ADR (and mark the old one superseded)
  adr list [--status <s>]                   List ADRs
";

/// Flags that take a value; every other `--x` is a switch.
const VALUED: [&str; 9] = ["--name", "--status", "--module", "--modules", "--supersedes", "--size", "--reads", "--refs", "--external"];

struct Args {
    pos: Vec<String>,
    flags: BTreeMap<String, String>,
}

impl Args {
    fn parse(raw: &[String], allowed: &[&str]) -> Result<Args, String> {
        let (mut pos, mut flags, mut it) = (Vec::new(), BTreeMap::new(), raw.iter());
        while let Some(a) = it.next() {
            if !a.starts_with("--") {
                pos.push(a.clone());
                continue;
            }
            if !allowed.contains(&a.as_str()) {
                return Err(format!("unknown option `{a}`"));
            }
            let v = if VALUED.contains(&a.as_str()) { it.next().ok_or(format!("`{a}` needs a value"))?.clone() } else { String::new() };
            flags.insert(a.clone(), v);
        }
        Ok(Args { pos, flags })
    }
    fn has(&self, f: &str) -> bool {
        self.flags.contains_key(f)
    }
    fn get(&self, f: &str) -> Option<&str> {
        self.flags.get(f).map(String::as_str)
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = raw.first() else {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    let rest = &raw[1..];
    let result = match cmd.as_str() {
        "init" => Args::parse(rest, &["--name", "--no-skill"]).and_then(|a| init(&a)),
        "lint" => Args::parse(rest, &["--strict"]).and_then(|a| lint_cmd(&a)),
        "map" => Args::parse(rest, &["--write", "--check"]).and_then(|a| map(&a)),
        "remap" => Args::parse(rest, &["--dry-run"]).and_then(|a| remap(&a)),
        "task" => Args::parse(rest, &["--size", "--modules", "--reads", "--refs", "--external", "--status", "--module", "--all"]).and_then(|a| task(&a)),
        "task-list" => Args::parse(rest, &["--status", "--module", "--all"]).and_then(|a| task_list(&a)),
        "next" => Args::parse(rest, &["--reserve"]).and_then(|a| next(&a)),
        "context" => Args::parse(rest, &["--paths"]).and_then(|a| context(&a)),
        "adr" => Args::parse(rest, &["--modules", "--supersedes", "--status"]).and_then(|a| adr(&a)),
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            Ok(true)
        }
        "-V" | "--version" => {
            println!("mwp {}", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

type Res = Result<bool, String>;

fn init(a: &Args) -> Res {
    let dir = Path::new(a.pos.first().map_or(".", String::as_str));
    for (path, content) in TEMPLATES {
        if a.has("--no-skill") && path.starts_with(".claude/") {
            continue;
        }
        let target = dir.join(path);
        if target.exists() {
            println!("skip   {path} (exists)");
            continue;
        }
        let content = match a.get("--name") {
            Some(n) if *path == "mwp/PROJECT.md" => content.replace("<Project name>", n),
            _ => content.to_string(),
        };
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(&target, content).map_err(|e| format!("{path}: {e}"))?;
        println!("create {path}");
    }
    println!("\nNext: fill mwp/PROJECT.md and the <placeholders> in mwp/constitution.md.");
    Ok(true)
}

fn lint_cmd(a: &Args) -> Res {
    let r = lint::lint(&Project::find()?);
    r.print();
    Ok(r.errors.is_empty() && !(a.has("--strict") && !r.warnings.is_empty()))
}

fn map(a: &Args) -> Res {
    let p = Project::find()?;
    if a.has("--write") {
        return write_map(&p);
    }
    if a.has("--check") {
        let (cur, new) = map_texts(&p)?;
        println!("{}", if cur == new { "map.md is up to date" } else { "map.md is out of date (run `mwp map --write`)" });
        return Ok(cur == new);
    }
    print!("{}", render_map(&p));
    Ok(true)
}

fn write_map(p: &Project) -> Res {
    let (cur, new) = map_texts(p)?;
    if cur == new {
        println!("map.md unchanged");
    } else {
        fs::write(p.mwp.join("map.md"), new).map_err(|e| e.to_string())?;
        println!("map.md updated ({} modules)", p.of_kind("module").count());
    }
    Ok(true)
}

fn remap(a: &Args) -> Res {
    let p = Project::find()?;
    let mut r = lint::Report::default();
    lint::check_modules(&p, &mut r);
    r.print();
    if a.has("--dry-run") {
        let (cur, new) = map_texts(&p)?;
        println!("{}", if cur == new { "map.md is up to date" } else { "map.md would change" });
    } else {
        write_map(&p)?;
    }
    Ok(r.errors.is_empty())
}

fn by_num(mut docs: Vec<&Doc>) -> Vec<&Doc> {
    docs.sort_by_key(|d| id_num(d.id()));
    docs
}

fn task_list(a: &Args) -> Res {
    let p = Project::find()?;
    let tasks = by_num(p.of_kind("task")
        .filter(|t| a.has("--all") || a.has("--status") || t.get("status") != "done")
        .filter(|t| a.get("--status").is_none_or(|s| t.get("status") == s))
        .filter(|t| a.get("--module").is_none_or(|m| t.list("modules").iter().any(|x| x == m)))
        .collect());
    for t in &tasks {
        let ext = t.list("external");
        let ext = if ext.is_empty() { String::new() } else { format!(" [{}]", ext.join(", ")) };
        println!("{:<8} {:<12} {:<2} {}{ext}", t.id(), t.get("status"), t.get("size"), t.title());
    }
    if tasks.is_empty() {
        println!("no tasks");
    }
    Ok(true)
}

fn next(a: &Args) -> Res {
    let prefix = a.pos.first().ok_or("usage: mwp next <T|SP|ADR|TEST> [--reserve]")?;
    if !PREFIXES.contains(&prefix.as_str()) {
        return Err(format!("prefix must be one of {}", PREFIXES.join(", ")));
    }
    let p = Project::find()?;
    let n = next_id(&p, prefix);
    if a.has("--reserve") {
        reserve(&p.root, prefix, n).map_err(|e| e.to_string())?;
    }
    println!("{prefix}-{n:04}");
    Ok(true)
}

fn context(a: &Args) -> Res {
    let key = a.pos.first().ok_or("usage: mwp context <T-XXXX | external code> [--paths]")?;
    let p = Project::find()?;
    let task = p.of_kind("task").find(|t| t.id() == key || t.list("external").contains(key))
        .ok_or(format!("no task with id or external code `{key}`"))?;

    let mut files = vec![("always", p.mwp.join("constitution.md")), ("always", task.path.clone())];
    let mut missing = Vec::new();
    for (field, kind) in [("modules", "module"), ("reads", "module"), ("specs", "spec"), ("decisions", "adr"), ("refs", "task")] {
        for id in task.list(field) {
            match p.by_id(kind, &id) {
                Some(d) => files.push((field, d.path.clone())),
                None => missing.push(format!("{id} ({field})")),
            }
        }
    }
    let mut total = 0;
    for (label, path) in &files {
        if a.has("--paths") {
            println!("{}", p.rel(path));
            continue;
        }
        // shortcut: ~4 bytes per token, good enough to spot an oversized task.
        let tokens = fs::metadata(path).map(|m| m.len() / 4).unwrap_or(0);
        total += tokens;
        println!("{:<55} {:>7}  {label}", p.rel(path), tokens);
    }
    if !a.has("--paths") {
        println!("{:<55} {:>7}  tokens (estimate)", "total", total);
    }
    for m in &missing {
        eprintln!("MISSING {m}");
    }
    Ok(missing.is_empty())
}

fn adr(a: &Args) -> Res {
    let p = Project::find()?;
    match a.pos.first().map(String::as_str) {
        Some("list") => {
            for d in by_num(p.of_kind("adr").filter(|d| a.get("--status").is_none_or(|s| d.get("status") == s)).collect()) {
                println!("{:<9} {:<11} {:<10} {}", d.id(), d.get("status"), d.get("date"), d.title());
            }
            Ok(true)
        }
        Some("new") => {
            let title = a.pos[1..].join(" ");
            if title.trim().is_empty() {
                return Err("usage: mwp adr new <title> [--modules a,b] [--supersedes ADR-XXXX]".into());
            }
            let old = match a.get("--supersedes") {
                Some(id) => Some(p.by_id("adr", id).ok_or(format!("no adr `{id}`"))?),
                None => None,
            };
            let n = next_id(&p, "ADR");
            let id = format!("ADR-{n:04}");
            let date = today();
            let template = template("mwp/decisions/_template.md");
            let text = template.replace("id: <ADR-XXXX>", &format!("id: {id}")).replace("# <ADR-XXXX> — <Title>", &format!("# {id} — {}", title.trim()))
                .replace("<YYYY-MM-DD>", &date)
                .replace("modules: []", &format!("modules: {}", list_arg(a.get("--modules"))))
                .replace("supersedes: []", &format!("supersedes: {}", list_arg(a.get("--supersedes"))));
            let path = p.mwp.join("decisions").join(format!("{id}-{}.md", slug(&title)));
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(&path, text).map_err(|e| e.to_string())?;
            reserve(&p.root, "ADR", n).map_err(|e| e.to_string())?;
            println!("create {} (status: proposed)", p.rel(&path));
            if let Some(old) = old {
                let t = doc::read(&old.path).ok_or("cannot read the superseded ADR")?;
                let t = set_field(&set_field(&set_field(&t, "status", "superseded"), "superseded_by", &id), "date", &date);
                fs::write(&old.path, t).map_err(|e| e.to_string())?;
                println!("update {} (status: superseded, superseded_by: {id})", p.rel(&old.path));
            }
            Ok(true)
        }
        _ => Err("usage: mwp adr <new|list> ...".into()),
    }
}

fn template(path: &str) -> &'static str {
    TEMPLATES.iter().find(|t| t.0 == path).unwrap().1
}

/// `a, b` from the command line as a front matter list: `[a, b]`.
fn list_arg(v: Option<&str>) -> String {
    format!("[{}]", v.unwrap_or("").split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "))
}

fn task(a: &Args) -> Res {
    match a.pos.first().map(String::as_str) {
        Some("list") => task_list(a),
        Some("new") => task_new(a),
        _ => Err("usage: mwp task <new|list> ...".into()),
    }
}

fn task_new(a: &Args) -> Res {
    let title = a.pos[1..].join(" ");
    if title.trim().is_empty() {
        return Err("usage: mwp task new <title> [--size S|M|L] [--modules a,b] [--reads a,b] [--refs T-XXXX] [--external CODE]".into());
    }
    let size = a.get("--size").unwrap_or("M");
    if !["S", "M", "L"].contains(&size) {
        return Err("--size must be S, M or L".into());
    }
    let p = Project::find()?;
    for (flag, kind) in [("--modules", "module"), ("--reads", "module"), ("--refs", "task")] {
        for id in doc::parse_list(&list_arg(a.get(flag))) {
            if p.by_id(kind, &id).is_none() {
                eprintln!("warning: {flag} cites `{id}`, no {kind} has that id");
            }
        }
    }
    let n = next_id(&p, "T");
    let id = format!("T-{n:04}");
    let mut text = template(if size == "S" { "mwp/tasks/open/_template-task-s.md" } else { "mwp/tasks/open/_template-task.md" })
        .replace("id: <T-XXXX>", &format!("id: {id}"))
        .replace("# <T-XXXX> — <Title>", &format!("# {id} — {}", title.trim()))
        .replace("created: <YYYY-MM-DD>", &format!("created: {}", today()))
        .replace("size: M ", &format!("size: {size} "));
    if !text.contains("\ncreated:") {
        text = set_field(&text, "created", &today()); // the S template has no `created:`
    }
    for f in ["modules", "reads", "refs", "external"] {
        let v = list_arg(a.get(&format!("--{f}")));
        if text.contains(&format!("{f}: []")) {
            text = text.replace(&format!("{f}: []"), &format!("{f}: {v}"));
        } else if v != "[]" {
            text = set_field(&text, f, &v);
        }
    }
    let path = p.mwp.join("tasks").join("open").join(format!("{id}-{}.md", slug(&title)));
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    reserve(&p.root, "T", n).map_err(|e| e.to_string())?;
    println!("create {} (size {size}, status: open)", p.rel(&path));
    Ok(true)
}
