//! The `mwp` command line: parses arguments, calls the core, prints the result.

use mwp_core::doc::{id_num, map_texts, next_id, render_map, Doc, Project, PREFIXES};
use mwp_core::scaffold::{parse_agents, AGENTS};
use mwp_core::{lint, ops, scaffold, state};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "mwp — Module Work Plan

Usage: mwp <command> [options]

Project
  init [dir] [--agent a,b] [--name <name>] [--no-skill] [--update-skills]
                                            Create mwp/ and the files of each agent; asks which agents
                                            (claude, codex, cursor, gemini, copilot; default claude). Never overwrites.
                                            --update-skills only rewrites the skills with this version's
  lint [--strict]                           Check the project; --strict also fails on warnings
  map [--write | --check]                   Print the module map; --write updates map.md, --check fails if stale
  remap [--dry-run]                         Recheck modules (paths, dependencies, tests) and update map.md

Tasks
  task new <title> [--size S|M|L] [--modules a,b] [--reads a,b] [--refs T-XXXX] [--external CODE]
                                            Create an open task from the template with the next T- ID
  task start <T-XXXX> [--agent <name>]      Claim the task and lock its modules; fails if another agent holds them
  task close <T-XXXX>                       Set done, move to tasks/done/, free the locks, run lint
  task release <T-XXXX> [--status open|blocked|review]
                                            Give the task back without closing it (default: open)
  task-list [--status <s>] [--module <id>] [--all]
                                            List tasks (open by default; --all includes done)
  locks                                     Tasks in progress, their agents and locked modules
  context <T-XXXX | external code> [--paths | --cat]
                                            Files an agent reads for a task, with token estimate; --cat prints them

Modules and SPECs
  module new <id> --summary <text> [--type domain|ui|api|infra|lib] [--folder a/b]
                                            Create a module from the template (default type domain), in a
                                            subfolder of mwp/modules/ with --folder, and update map.md
  module folder <a/b>                       Create a folder for modules under mwp/modules/
  spec new <title> --owner <module|T-XXXX> [--lifetime durable|transient]
                                            Create an active SPEC with the next SP- ID (default durable)
  delete <id | path> [--yes]                Delete a module, a SPEC, a task not done or a content file (by path);
                                            frees the task's locks and updates map.md. Asks first; --yes skips
                                            the question (needed when not run from a terminal)

Resources (mwp/content/)
  content add <file>... [--folder a/b]      Copy files into mwp/content/ (or a subfolder); never overwrites
  content folder <a/b>                      Create a folder under mwp/content/

IDs and decisions
  next <T|SP|ADR|TEST> [--reserve]          Next free ID; --reserve records it in .mwp/ledger.json
  adr new <title> [--modules a,b] [--supersedes ADR-XXXX]
                                            Create a proposed ADR (and mark the old one superseded)
  adr list [--status <s>]                   List ADRs

Environment
  MWP_AGENT       Default for --agent in `task start`
  MWP_STATE_DIR   Folder for the ledger and locks, when several checkouts of one project share them
";

/// Flags that take a value; every other `--x` is a switch.
const VALUED: [&str; 15] = [
    "--name", "--status", "--module", "--modules", "--supersedes", "--size", "--reads", "--refs", "--external", "--agent",
    "--summary", "--type", "--folder", "--owner", "--lifetime",
];
// `--folder` is shared by `module new` and `content add`.

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
    // `--help` anywhere shows the usage: agents try `mwp task new --help`.
    if raw.is_empty() || raw.iter().any(|a| a == "--help" || a == "-h" || a == "help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let rest = &raw[1..];
    let result = match raw[0].as_str() {
        "init" => Args::parse(rest, &["--name", "--agent", "--no-skill", "--update-skills"]).and_then(|a| init(&a)),
        "lint" => Args::parse(rest, &["--strict"]).and_then(|a| lint_cmd(&a)),
        "map" => Args::parse(rest, &["--write", "--check"]).and_then(|a| map(&a)),
        "remap" => Args::parse(rest, &["--dry-run"]).and_then(|a| remap(&a)),
        "task" => Args::parse(rest, &["--size", "--modules", "--reads", "--refs", "--external", "--status", "--module", "--all", "--agent"]).and_then(|a| task(&a)),
        "task-list" => Args::parse(rest, &["--status", "--module", "--all"]).and_then(|a| task_list(&a)),
        "locks" => locks(),
        "next" => Args::parse(rest, &["--reserve"]).and_then(|a| next(&a)),
        "context" => Args::parse(rest, &["--paths", "--cat"]).and_then(|a| context(&a)),
        "adr" => Args::parse(rest, &["--modules", "--supersedes", "--status"]).and_then(|a| adr(&a)),
        "module" => Args::parse(rest, &["--summary", "--type", "--folder"]).and_then(|a| module(&a)),
        "spec" => Args::parse(rest, &["--owner", "--lifetime"]).and_then(|a| spec(&a)),
        "delete" => Args::parse(rest, &["--yes"]).and_then(|a| delete(&a)),
        "content" => Args::parse(rest, &["--folder"]).and_then(|a| content(&a)),
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

fn show(lines: ops::Lines) -> Res {
    for l in lines? {
        println!("{l}");
    }
    Ok(true)
}

fn ask(question: &str) -> String {
    print!("{question}");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    let _ = io::stdin().read_line(&mut answer);
    answer.trim().to_string()
}

fn init(a: &Args) -> Res {
    let dir = Path::new(a.pos.first().map_or(".", String::as_str));
    if a.has("--update-skills") {
        // Without --agent: the agents already set up here, or the default.
        let agents = match a.get("--agent") {
            None if !scaffold::installed(dir).is_empty() => scaffold::installed(dir),
            v => parse_agents(v.unwrap_or(""))?,
        };
        return show(scaffold::update_skills(dir, &agents));
    }
    // Questions only for a person at a terminal; an agent or a script gets the defaults.
    let interactive = io::stdin().is_terminal();
    let agents = match a.get("--agent") {
        Some(v) => v.to_string(),
        None if interactive => {
            println!("Which agents will work on this project?");
            for (i, ag) in AGENTS.iter().enumerate() {
                println!("  {}. {:<8} {}", i + 1, ag.id, ag.name);
            }
            ask("Numbers or names, separated by commas; `all` for every one [claude]: ")
        }
        None => String::new(),
    };
    let agents = parse_agents(&agents)?;
    let name = match a.get("--name") {
        Some(n) => Some(n.to_string()),
        None if interactive && !dir.join("mwp/PROJECT.md").exists() => {
            let folder = std::path::absolute(dir).ok().and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned())).unwrap_or_default();
            let n = ask(&format!("Project name [{folder}]: "));
            Some(if n.is_empty() { folder } else { n })
        }
        None => None,
    };
    show(scaffold::init(dir, name.as_deref().filter(|n| !n.is_empty()), &agents, !a.has("--no-skill")))
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

fn task(a: &Args) -> Res {
    let sub = a.pos.first().map(String::as_str);
    let id = || a.pos.get(1).map(String::as_str).ok_or(format!("usage: mwp task {} <T-XXXX>", sub.unwrap_or("")));
    match sub {
        Some("list") => task_list(a),
        Some("new") => {
            let fields = ["modules", "reads", "refs", "external"].map(|f| (f, a.get(&format!("--{f}"))));
            show(ops::task_new(&Project::find()?, &a.pos[1..].join(" "), a.get("--size").unwrap_or("M"), &fields))
        }
        Some("start") => {
            let agent = a.get("--agent").map(String::from).or_else(|| std::env::var("MWP_AGENT").ok()).filter(|s| !s.trim().is_empty());
            show(ops::task_start(&Project::find()?, id()?, agent.as_deref().unwrap_or("agent").trim()))
        }
        Some("close") => {
            let (lines, report) = ops::task_close(&Project::find()?, id()?)?;
            show(Ok(lines))?;
            report.print();
            Ok(report.errors.is_empty())
        }
        Some("release") => show(ops::task_release(&Project::find()?, id()?, a.get("--status").unwrap_or("open"))),
        _ => Err("usage: mwp task <new|list|start|close|release> ...".into()),
    }
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
        let agent = if t.get("status") == "in_progress" && !t.get("agent").is_empty() { format!(" @{}", t.get("agent")) } else { String::new() };
        println!("{:<8} {:<12} {:<2} {}{ext}{agent}", t.id(), t.get("status"), t.get("size"), t.title());
    }
    if tasks.is_empty() {
        println!("no tasks");
    }
    Ok(true)
}

fn locks() -> Res {
    let p = Project::find()?;
    let locks = state::locks(&p.state);
    let claims = state::claims(&p.state);
    for c in &claims {
        let modules: Vec<&str> = locks.iter().filter(|l| l.1.task == c.task).map(|l| l.0.as_str()).collect();
        println!("{:<8} {:<12} {}", c.task, c.agent, if modules.is_empty() { "-".into() } else { modules.join(", ") });
    }
    if claims.is_empty() {
        println!("no tasks in progress");
    }
    Ok(true)
}

fn next(a: &Args) -> Res {
    let prefix = a.pos.first().ok_or("usage: mwp next <T|SP|ADR|TEST> [--reserve]")?;
    if !PREFIXES.contains(&prefix.as_str()) {
        return Err(format!("prefix must be one of {}", PREFIXES.join(", ")));
    }
    let p = Project::find()?;
    let n = if a.has("--reserve") { state::allocate(&p, prefix).map_err(|e| e.to_string())? } else { next_id(&p, prefix) };
    println!("{prefix}-{n:04}");
    Ok(true)
}

fn context(a: &Args) -> Res {
    let key = a.pos.first().ok_or("usage: mwp context <T-XXXX | external code> [--paths | --cat]")?;
    let p = Project::find()?;
    let task = ops::find_task(&p, key)?;
    let c = ops::context(&p, task);
    for (label, path, tokens) in &c.files {
        if a.has("--paths") {
            println!("{}", p.rel(path));
        } else if a.has("--cat") {
            // For agents without skills: the whole context in one stream.
            println!("<!-- file: {} -->\n{}", p.rel(path), mwp_core::doc::read(path).unwrap_or_default());
        } else {
            println!("{:<55} {:>7}  {label}", p.rel(path), tokens);
        }
    }
    if !a.has("--paths") && !a.has("--cat") {
        println!("{:<55} {:>7}  tokens (estimate)", "total", c.tokens());
    }
    for m in &c.missing {
        eprintln!("MISSING {m}");
    }
    Ok(c.missing.is_empty())
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
        Some("new") => show(ops::adr_new(&p, &a.pos[1..].join(" "), a.get("--modules"), a.get("--supersedes"))),
        _ => Err("usage: mwp adr <new|list> ...".into()),
    }
}

fn module(a: &Args) -> Res {
    let p = Project::find()?;
    match (a.pos.first().map(String::as_str), a.pos.get(1)) {
        (Some("new"), Some(id)) => {
            let summary = a.get("--summary").ok_or("usage: mwp module new <id> --summary <text> [--type domain|ui|api|infra|lib] [--folder a/b]")?;
            show(ops::module_new(&p, a.get("--folder").unwrap_or(""), id, a.get("--type").unwrap_or("domain"), summary))
        }
        (Some("folder"), Some(path)) => {
            let path = path.trim_matches(['/', '\\']);
            let (parent, name) = path.rsplit_once(['/', '\\']).unwrap_or(("", path));
            show(ops::module_folder_new(&p, parent, name))
        }
        _ => Err("usage: mwp module new <id> --summary <text> [--type domain|ui|api|infra|lib] [--folder a/b]\n       mwp module folder <a/b>".into()),
    }
}

fn spec(a: &Args) -> Res {
    match a.pos.first().map(String::as_str) {
        Some("new") => {
            let owner = a.get("--owner").ok_or("usage: mwp spec new <title> --owner <module|T-XXXX> [--lifetime durable|transient]")?;
            show(ops::spec_new(&Project::find()?, &a.pos[1..].join(" "), owner, a.get("--lifetime").unwrap_or("durable")))
        }
        _ => Err("usage: mwp spec new <title> --owner <module|T-XXXX> [--lifetime durable|transient]".into()),
    }
}

/// Deletes by ID (a module, a SPEC, or a task, also by its external code) or by path (a content file).
fn delete(a: &Args) -> Res {
    let key = a.pos.first().ok_or("usage: mwp delete <module | SP-XXXX | T-XXXX | mwp/content/file> [--yes]")?;
    let p = Project::find()?;
    let path = match p.docs.iter().find(|d| d.id() == key && ["module", "spec", "task"].contains(&d.kind())).or_else(|| ops::find_task(&p, key).ok()) {
        Some(doc) => doc.path.clone(),
        // Not an ID: a path, from the project root or from here.
        None => [p.root.join(key), std::path::absolute(key).unwrap_or_default()].into_iter().find(|f| f.is_file())
            .ok_or(format!("no module, SPEC, task or file `{key}`"))?,
    };
    if !a.has("--yes") {
        // Deleting cannot be undone: a person confirms, an agent or a script must say --yes.
        if !io::stdin().is_terminal() {
            return Err(format!("deleting {} cannot be undone; pass --yes", p.rel(&path)));
        }
        if !matches!(ask(&format!("Delete {}? This cannot be undone [y/N]: ", p.rel(&path))).to_lowercase().as_str(), "y" | "yes") {
            println!("nothing deleted");
            return Ok(true);
        }
    }
    show(ops::delete(&p, &path))
}

fn content(a: &Args) -> Res {
    let p = Project::find()?;
    match (a.pos.first().map(String::as_str), &a.pos[1.min(a.pos.len())..]) {
        (Some("add"), files) if !files.is_empty() => {
            let lines = ops::content_add(&p, a.get("--folder").unwrap_or(""), &files.iter().map(std::path::PathBuf::from).collect::<Vec<_>>())?;
            let skipped = lines.iter().any(|l| l.starts_with("skip"));
            show(Ok(lines))?;
            Ok(!skipped)
        }
        (Some("folder"), [path]) => {
            let path = path.trim_matches(['/', '\\']);
            let (parent, name) = path.rsplit_once(['/', '\\']).unwrap_or(("", path));
            show(ops::content_folder_new(&p, parent, name))
        }
        _ => Err("usage: mwp content add <file>... [--folder a/b]\n       mwp content folder <a/b>".into()),
    }
}
