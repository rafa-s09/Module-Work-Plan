"""Builds two clean workspaces for the exercises, with no answer key in sight:

    python evaluation/make_workspaces.py <empty folder outside this repository>

    with-mwp/     the project as it is, minus this folder and the README
    without-mwp/  the same code, with the same rules in one ARCHITECTURE.md and the tasks without manifests
"""

import re
import shutil
import sys
from pathlib import Path

EXAMPLE = Path(__file__).resolve().parents[1]
REPOSITORY = EXAMPLE.parents[1]
JUNK = ("node_modules", ".venv", "data", "dist", ".angular", "__pycache__", ".pytest_cache", ".ruff_cache")
# Constitution sections that are about rules of the project, not about working in MWP.
KEPT_SECTIONS = ("0.", "2.", "3.", "5.", "8.", "9.")
GITIGNORE = "\n".join(JUNK) + "\n"
TASKS = ("T-0008", "T-0009", "T-0010", "T-0011", "T-0012")


def body(path: Path) -> str:
    return re.sub(r"\A---\n.*?\n---\n", "", path.read_text(encoding="utf-8"), flags=re.S).strip()


def without_mwp_words(text: str) -> str:
    text = text.replace("mwp/content/", "content/")
    text = text.replace("unless the task lists it in `modifies_tests:`", "unless the task says so")
    return "\n".join(
        line for line in text.splitlines() if not re.search(r"\bmwp\b|`modules:`|tasks/done|modifies_tests", line)
    )


def constitution_rules() -> str:
    sections = body(EXAMPLE / "mwp/constitution.md").split("\n## ")
    return "\n\n".join("## " + s for s in sections[1:] if s.startswith(KEPT_SECTIONS))


def architecture() -> str:
    mwp = EXAMPLE / "mwp"
    parts = ["# Architecture", constitution_rules(), body(mwp / "PROJECT.md"), body(mwp / "glossary.md")]
    parts += [body(m) for m in sorted((mwp / "modules").glob("*.md")) if m.name not in ("README.md", "_template.md")]
    parts += [body(d) for d in sorted((mwp / "decisions").glob("ADR-*.md"))]
    return without_mwp_words("\n\n".join(parts)) + "\n"


def plain_task(path: Path) -> str:
    """Goal, scope and criteria only: the manifest and the closure are what MWP adds."""
    return without_mwp_words(body(path).split("\n## Plan")[0]) + "\n"


def copy(src: Path, dst: Path) -> None:
    shutil.copytree(src, dst, ignore=shutil.ignore_patterns(*JUNK))


def main(out: Path) -> None:
    if out.is_relative_to(REPOSITORY):
        raise SystemExit("Choose a folder outside this repository, or the agent can walk up to the answer key.")
    if out.exists() and any(out.iterdir()):
        raise SystemExit(f"{out} is not empty.")

    with_mwp = out / "with-mwp"
    copy(EXAMPLE, with_mwp)
    shutil.rmtree(with_mwp / "evaluation")
    shutil.rmtree(with_mwp / "docs")
    (with_mwp / "README.md").unlink()

    plain = out / "without-mwp"
    copy(EXAMPLE / "backend", plain / "backend")
    copy(EXAMPLE / "frontend", plain / "frontend")
    copy(EXAMPLE / "mwp/content", plain / "content")
    for workspace in (with_mwp, plain):
        (workspace / ".gitignore").write_text(GITIGNORE, encoding="utf-8", newline="\n")
    text = architecture()
    (plain / "ARCHITECTURE.md").write_text(text, encoding="utf-8", newline="\n")
    pointer = "# Agent instructions\n\nBefore any work, read [`ARCHITECTURE.md`](ARCHITECTURE.md). Tasks are in `tasks/`.\n"
    (plain / "AGENTS.md").write_text(pointer, encoding="utf-8", newline="\n")
    (plain / "CLAUDE.md").write_text(pointer, encoding="utf-8", newline="\n")
    (plain / "tasks").mkdir()
    for task in TASKS:
        source = next((EXAMPLE / "mwp/tasks/open").glob(f"{task}-*.md"))
        (plain / "tasks" / source.name).write_text(plain_task(source), encoding="utf-8", newline="\n")

    # A word of the format left behind would make the comparison unfair.
    assert not re.search(r"\bmwp\b", text)
    assert "**FR-05**" in text and "ADR-0001" in text and "Davy Jones" in text
    print(f"Done: {with_mwp} and {plain}")
    print(f"ARCHITECTURE.md is about {len(text.encode()) // 4} tokens (same estimate as `mwp context`), read for every task.")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    main(Path(sys.argv[1]).resolve())
