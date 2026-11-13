//! `mwp init`: the files a new project gets, and where each agent looks for them.
//! Everything is embedded in the binary: once installed, `mwp` needs no checkout and no git.

use std::fs;
use std::path::Path;

/// Files every project gets, whatever the agent.
const DOCS: &[(&str, &str)] = &[
    ("AGENTS.md", POINTER),
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
    ("mwp/content/.gitkeep", ""),
];

/// The pointer to the constitution; the same text goes into each agent's instruction file.
const POINTER: &str = include_str!("../../../templates/AGENTS.md");

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

pub fn template(path: &str) -> &'static str {
    DOCS.iter().find(|t| t.0 == path).unwrap().1
}

/// Creates the MWP structure in `dir` for the chosen agents. Never overwrites a file.
pub fn init(dir: &Path, name: Option<&str>, agents: &[&Agent], skills: bool) -> Result<Vec<String>, String> {
    let mut files: Vec<(String, &str)> = DOCS.iter().map(|(p, c)| (p.to_string(), *c)).collect();
    for a in agents {
        files.push((a.pointer.to_string(), POINTER));
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
        let content = match name {
            Some(n) if path == "mwp/PROJECT.md" => content.replace("<Project name>", n),
            _ => content.to_string(),
        };
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
}
