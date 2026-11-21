//! `mwp init`: the files a new project gets, and where each agent looks for them.
//! The templates come from `templates.tar.zst` next to the binary (packed from `templates/` by
//! build.rs); the skills are embedded in the binary.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// The templates archive, installed next to `mwp` and `mwp-gui`.
pub const ARCHIVE: &str = "templates.tar.zst";

/// The pointer to the constitution; the same text goes into each agent's instruction file.
const POINTER: &str = "AGENTS.md";

/// Every file in the archive, by its path inside it (`mwp/PROJECT.md`), read once.
fn templates() -> Result<&'static BTreeMap<String, String>, String> {
    static FILES: OnceLock<Result<BTreeMap<String, String>, String>> = OnceLock::new();
    FILES.get_or_init(|| read_archive(&find_archive()?)).as_ref().map_err(Clone::clone)
}

/// Next to the binary or one folder up (tests run from `target/<profile>/deps/`).
fn find_archive() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.ancestors().skip(1).take(2).map(|d| d.join(ARCHIVE)).find(|p| p.is_file())
        .ok_or_else(|| format!("{ARCHIVE} not found next to {}: reinstall mwp", exe.display()))
}

pub fn read_archive(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let file = fs::File::open(path).map_err(|e| err(&e))?;
    let zst = ruzstd::decoding::StreamingDecoder::new(file).map_err(|e| err(&e))?;
    let mut files = BTreeMap::new();
    for entry in tar::Archive::new(zst).entries().map_err(|e| err(&e))? {
        let mut entry = entry.map_err(|e| err(&e))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path().map_err(|e| err(&e))?.to_string_lossy().replace('\\', "/");
        let mut text = String::new();
        std::io::Read::read_to_string(&mut entry, &mut text).map_err(|e| err(&format!("{name}: {e}")))?;
        files.insert(name.trim_start_matches("./").to_string(), text);
    }
    Ok(files)
}

const SKILLS: [(&str, &str); 2] = [
    ("mwp/SKILL.md", include_str!("../../skill/mwp/SKILL.md")),
    ("module/SKILL.md", include_str!("../../skill/module/SKILL.md")),
];

pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    /// Instruction file the agent reads at startup.
    pub pointer: &'static str,
    /// Folder where the agent discovers skills.
    pub skills: &'static str,
}

/// Paths from each agent's documentation (checked 2026-11). The first one is the default.
pub const AGENTS: [Agent; 5] = [
    Agent { id: "claude", name: "Claude Code", pointer: "CLAUDE.md", skills: ".claude/skills" },
    Agent { id: "codex", name: "OpenAI Codex", pointer: "AGENTS.md", skills: ".agents/skills" },
    Agent { id: "cursor", name: "Cursor", pointer: "AGENTS.md", skills: ".cursor/skills" },
    Agent { id: "gemini", name: "Gemini CLI", pointer: "GEMINI.md", skills: ".gemini/skills" },
    Agent { id: "copilot", name: "GitHub Copilot", pointer: ".github/copilot-instructions.md", skills: ".github/skills" },
];

/// Agents from `claude, codex`, `1 3` or `all`. Nothing chosen means the default, Claude.
pub fn parse_agents(raw: &str) -> Result<Vec<&'static Agent>, String> {
    let mut out: Vec<&'static Agent> = Vec::new();
    for token in raw.split([',', ' ', ';']).map(str::trim).filter(|t| !t.is_empty()) {
        let found: Vec<&Agent> = match token.to_lowercase().as_str() {
            "all" => AGENTS.iter().collect(),
            t => AGENTS.iter().enumerate().filter(|(i, a)| a.id == t || (i + 1).to_string() == t).map(|(_, a)| a).collect(),
        };
        if found.is_empty() {
            let ids: Vec<&str> = AGENTS.iter().map(|a| a.id).collect();
            return Err(format!("unknown agent `{token}`; choose from {}, or `all`", ids.join(", ")));
        }
        for a in found {
            if !out.iter().any(|o| o.id == a.id) {
                out.push(a);
            }
        }
    }
    if out.is_empty() {
        out.push(&AGENTS[0]);
    }
    Ok(out)
}

/// Agents already set up in `dir`: the ones whose skills folder has the `mwp` skill.
pub fn installed(dir: &Path) -> Vec<&'static Agent> {
    AGENTS.iter().filter(|a| dir.join(a.skills).join(SKILLS[0].0).exists()).collect()
}

/// Rewrites the skills of `agents` with the version in this binary. Touches nothing else.
pub fn update_skills(dir: &Path, agents: &[&Agent]) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for a in agents {
        for (name, content) in SKILLS {
            let path = format!("{}/{name}", a.skills);
            let target = dir.join(&path);
            if fs::read_to_string(&target).is_ok_and(|cur| cur.replace("\r\n", "\n") == content.replace("\r\n", "\n")) {
                lines.push(format!("same   {path}"));
                continue;
            }
            fs::create_dir_all(target.parent().unwrap()).and_then(|_| fs::write(&target, content)).map_err(|e| format!("{path}: {e}"))?;
            lines.push(format!("update {path}"));
        }
    }
    Ok(lines)
}

pub fn template(path: &str) -> Result<&'static str, String> {
    templates()?.get(path).map(String::as_str).ok_or_else(|| format!("{ARCHIVE} has no {path}: reinstall mwp"))
}

/// Creates the MWP structure in `dir` for the chosen agents. Never overwrites a file.
pub fn init(dir: &Path, name: Option<&str>, agents: &[&Agent], skills: bool) -> Result<Vec<String>, String> {
    let mut files: Vec<(String, &str)> = templates()?.iter().map(|(p, c)| (p.clone(), c.as_str())).collect();
    let pointer = template(POINTER)?;
    for a in agents {
        files.push((a.pointer.to_string(), pointer));
        if skills {
            files.extend(SKILLS.iter().map(|(p, c)| (format!("{}/{p}", a.skills), *c)));
        }
    }
    let mut lines = Vec::new();
    let mut seen = Vec::new();
    for (path, content) in &files {
        if seen.contains(&path) {
            continue; // several agents share AGENTS.md
        }
        seen.push(path);
        let target = dir.join(path);
        if target.exists() {
            lines.push(format!("skip   {path} (exists)"));
            continue;
        }
        let mut content = match name {
            Some(n) if path == "mwp/PROJECT.md" => content.replace("<Project name>", n),
            _ => content.to_string(),
        };
        if matches!(path.as_str(), "mwp/PROJECT.md" | "mwp/constitution.md") {
            content = content.replace("<YYYY-MM-DD>", &crate::doc::today());
        }
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| format!("{path}: {e}"))?;
        fs::write(&target, content).map_err(|e| format!("{path}: {e}"))?;
        lines.push(format!("create {path}"));
    }

    // `.mwp/` holds local agent state (claims, locks); only the ID ledger is shared.
    let ignore = dir.join(".gitignore");
    let cur = fs::read_to_string(&ignore).unwrap_or_default();
    if !cur.contains(".mwp/") {
        let sep = if cur.is_empty() || cur.ends_with('\n') { "" } else { "\n" };
        fs::write(&ignore, format!("{cur}{sep}# MWP: local agent state; the ID ledger is shared\n.mwp/*\n!.mwp/ledger.json\n"))
            .map_err(|e| format!(".gitignore: {e}"))?;
        lines.push("update .gitignore (.mwp/ is local, except the ledger)".into());
    }

    let ids: Vec<&str> = agents.iter().map(|a| a.id).collect();
    lines.push(format!("\nAgents: {}. Next: fill mwp/PROJECT.md and the <placeholders> in mwp/constitution.md.", ids.join(", ")));
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_default_to_claude() {
        let ids = |raw: &str| parse_agents(raw).unwrap().iter().map(|a| a.id).collect::<Vec<_>>();
        assert_eq!(ids(""), ["claude"]);
        assert_eq!(ids("  "), ["claude"]);
        assert_eq!(ids("2, Gemini 2"), ["codex", "gemini"]);
        assert_eq!(ids("all").len(), AGENTS.len());
        assert!(parse_agents("claude, gpt").is_err());
    }

    #[test]
    fn the_archive_holds_every_file_of_templates() {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, root, out);
                } else {
                    let name = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.insert(name, fs::read_to_string(&p).unwrap());
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
        let mut expected = BTreeMap::new();
        walk(&root, &root, &mut expected);
        assert_eq!(templates().unwrap(), &expected);
        assert!(read_archive(Path::new("gone.tar.zst")).unwrap_err().contains("gone.tar.zst"));
    }
}
