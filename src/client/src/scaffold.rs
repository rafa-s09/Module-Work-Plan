//! `mwp init`: the files a new project gets, and where each agent looks for them.
//! The templates come from template packages next to the binary (`<name>.tar.zst`, packed from
//! `templates/<name>/` by build.rs). `default` is the full project; another package only carries
//! the files it changes (PROJECT.md, the constitution...), laid over the default. The skills are
//! embedded in the binary.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The package every project starts from.
pub const DEFAULT: &str = "default";

/// Files that describe a package and never go into a project.
pub const META: [&str; 2] = ["manifest.json", "thumbs.png"];

/// This mwp's version: what `mwp_version:` in PROJECT.md says a project is written for.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The pointer to the constitution; the same text goes into each agent's instruction file.
const POINTER: &str = "AGENTS.md";

/// The files of a package by their path inside it (`mwp/PROJECT.md`).
pub type Files = BTreeMap<String, Vec<u8>>;

/// The default package, read once.
fn default_files() -> Result<&'static Files, String> {
    static FILES: OnceLock<Result<Files, String>> = OnceLock::new();
    FILES.get_or_init(|| package(DEFAULT)).as_ref().map_err(Clone::clone)
}

/// The folder with the packages: next to the binary or one folder up (tests run from
/// `target/<profile>/deps/`).
pub fn packages_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.ancestors().skip(1).take(2).find(|d| d.join(format!("{DEFAULT}.tar.zst")).is_file()).map(Path::to_path_buf)
        .ok_or_else(|| format!("{DEFAULT}.tar.zst not found next to {}: reinstall mwp", exe.display()))
}

/// The installed packages by name, `default` first.
pub fn packages() -> Vec<String> {
    let Ok(dir) = packages_dir() else { return Vec::new() };
    let mut names: Vec<String> = fs::read_dir(dir).into_iter().flatten().flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".tar.zst").map(String::from))
        .collect();
    names.sort_by_key(|n| (n != DEFAULT, n.clone()));
    names
}

/// Every file of the package `name`, read from its archive.
pub fn package(name: &str) -> Result<Files, String> {
    let path = packages_dir()?.join(format!("{name}.tar.zst"));
    if name.is_empty() || name.contains(['/', '\\']) || !path.is_file() {
        return Err(format!("no template `{name}`; installed: {}", packages().join(", ")));
    }
    read_archive(&path)
}

pub fn read_archive(path: &Path) -> Result<Files, String> {
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
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).map_err(|e| err(&format!("{name}: {e}")))?;
        files.insert(name.trim_start_matches("./").to_string(), bytes);
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

/// A file of the default package (task, module, SPEC and ADR templates are the same in every package).
pub fn template(path: &str) -> Result<&'static str, String> {
    let bytes = default_files()?.get(path).ok_or_else(|| format!("{DEFAULT}.tar.zst has no {path}: reinstall mwp"))?;
    std::str::from_utf8(bytes).map_err(|_| format!("{DEFAULT}.tar.zst: {path} is not text"))
}

/// The project files of a package: the default's, with the package's own laid over them.
pub fn project_files(package_name: &str) -> Result<BTreeMap<String, String>, String> {
    let mut files = default_files()?.clone();
    if package_name != DEFAULT {
        files.extend(package(package_name)?);
    }
    files.into_iter()
        .filter(|(p, _)| !META.contains(&p.as_str()))
        .map(|(p, b)| String::from_utf8(b).map(|t| (p.clone(), t)).map_err(|_| format!("template {package_name}: {p} is not text")))
        .collect()
}

/// Creates the MWP structure in `dir` from a template package, for the chosen agents. Never
/// overwrites a file.
pub fn init(dir: &Path, name: Option<&str>, agents: &[&Agent], skills: bool, package_name: &str) -> Result<Vec<String>, String> {
    let all = project_files(package_name)?;
    let pointer = all.get(POINTER).cloned().ok_or(format!("template {package_name} has no {POINTER}"))?;
    let mut files: Vec<(String, String)> = all.into_iter().collect();
    for a in agents {
        files.push((a.pointer.to_string(), pointer.clone()));
        if skills {
            files.extend(SKILLS.iter().map(|(p, c)| (format!("{}/{p}", a.skills), c.to_string())));
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
        if path == "mwp/PROJECT.md" {
            content = crate::doc::set_field(&content, "mwp_version", VERSION);
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
    fn each_package_holds_every_file_of_its_folder() {
        fn walk(dir: &Path, root: &Path, out: &mut Files) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, root, out);
                } else {
                    let name = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.insert(name, fs::read(&p).unwrap());
                }
            }
        }
        let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
        for dir in fs::read_dir(&templates).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            let mut expected = Files::new();
            walk(&dir, &dir, &mut expected);
            assert_eq!(package(&name).unwrap(), expected, "{name}");
            assert!(packages().contains(&name));
        }
        assert_eq!(packages()[0], DEFAULT);
        assert!(package("gone").unwrap_err().contains("default"), "an unknown package lists the installed ones");
        assert!(read_archive(Path::new("gone.tar.zst")).unwrap_err().contains("gone.tar.zst"));
        // A project never gets the package's description, and PROJECT.md says which mwp wrote it.
        let files = project_files(DEFAULT).unwrap();
        assert!(!files.contains_key("manifest.json") && !files.contains_key("thumbs.png") && files.contains_key("mwp/constitution.md"));
    }

    #[test]
    fn a_package_is_laid_over_the_default() {
        let mut tar = tar::Builder::new(Vec::new());
        for (path, text) in [("manifest.json", "{}"), ("mwp/PROJECT.md", "---
kind: project
mwp_version: 1
name: <Project name>
---
# Engine game
")] {
            let mut h = tar::Header::new_gnu();
            h.set_size(text.len() as u64);
            h.set_cksum();
            tar.append_data(&mut h, path, text.as_bytes()).unwrap();
        }
        let packed = ruzstd::encoding::compress_to_vec(&tar.into_inner().unwrap()[..], ruzstd::encoding::CompressionLevel::Fastest);
        let file = packages_dir().unwrap().join("zz-overlay-test.tar.zst");
        fs::write(&file, packed).unwrap();
        let files = project_files("zz-overlay-test");
        let _ = fs::remove_file(&file);
        let files = files.unwrap();
        assert!(files["mwp/PROJECT.md"].contains("# Engine game"), "the package's file wins");
        assert_eq!(files["mwp/constitution.md"], template("mwp/constitution.md").unwrap(), "the rest comes from the default");
        assert!(!files.contains_key("manifest.json"));
        let dir = std::env::temp_dir().join(format!("mwp-overlay-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(init(&dir, Some("X"), &[&AGENTS[0]], false, "nope").unwrap_err().contains("no template `nope`"));
        init(&dir, Some("X"), &[&AGENTS[0]], false, DEFAULT).unwrap();
        assert!(fs::read_to_string(dir.join("mwp/PROJECT.md")).unwrap().contains(&format!("mwp_version: {VERSION}")));
        let _ = fs::remove_dir_all(&dir);
    }
}
