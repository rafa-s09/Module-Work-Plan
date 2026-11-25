# Changelog

What changed in each version of the CLI and core (`mwp`) and of the desktop editor (`mwp-gui`).

## mwp 0.3.1 · mwp-gui 0.1.5 (2026-11-25)

### CLI and core

### GUI
- **File > Open recent** and **File > Close project:** switch to a recent project in one click, or close the open one and go back to the start screen (New, Open, Recent).
- **Project menu:** Go to file, New task, New module, New SPEC, New ADR, Save, Save all, Close file, Delete file and Rebuild ID ledger moved here from the File menu, which keeps New project, Open project, Open recent, Close project and Quit.


## mwp 0.3.1 · mwp-gui 0.1.4 (2026-11-20)

### CLI and core
- **Task sizes:** XS, S, M, L, XL and XXL (`mwp task new --size`). XS (a trivial change, like bumping a version) uses the short template like S; L and up need an approved plan; XL puts the plan in a SPEC (also for tasks with many manual checks); an XXL task (too big, or far outside the project's field) has a plan that splits it into smaller tasks.
- **Replanning:** `mwp task close T-XXXX --replanned` closes a task whose plan cannot reach the goal: `status: replanned`, moved to `tasks/done/`. It needs `**Replanning:**` in `## Closure`; lint accepts a replanned task in `tasks/done/` and in `refs:`.
- **Plan approval:** L, XL and XXL tasks carry `plan_approved:`, which only a person sets (in the GUI). `mwp task close` refuses those sizes without it; lint reports a task in review without it, and warns while it is in progress.
- **`mwp task block` and `mwp task review`:** shortcuts for `mwp task release --status blocked` and `--status review`.
- **ID ledger:** `mwp next` (and every command or GUI action that takes a new ID) checks `.mwp/ledger.json` first. A missing or broken ledger is rebuilt from the highest ID of each prefix in the project; while two tasks, SPECs or ADRs share an ID it refuses, names the files and suggests a free ID, until one is renumbered by hand. New `mwp ledger` creates or rebuilds it, never below a reservation still in a ledger that reads. Lint warns about a ledger that does not read.
- **Template packages:** `templates.tar.zst` becomes `default.tar.zst`, and every folder of `templates/` is packed as `<folder>.tar.zst` with a `manifest.json` (name, created, creator, language, details, repository) and a 512×512 `thumbs.png`. A package other than `default` carries only the files it changes and takes the rest from `default`. `mwp init --template <name>` picks one; the installers copy every package and remove the old `templates.tar.zst`.
- **`mwp_version`:** a new project records the version of the mwp that made it (`0.3.1`). Lint reports a project written for a newer mwp (an error: update mwp) or an older one (a warning: run `mwp upgrade`).
- **`mwp upgrade [--dry-run]`:** brings an older project up to this mwp: `mwp_version`, `created:` (from the earliest task), the front matter of the constitution, map and glossary, the `_template` files and the agents' skills, keeping each file's line endings. The constitution's text is left alone; it lists the sections of the current rules it lacks.
- **`[-]` marks:** lint warns about a replanned task with no criterion marked `- [-]`, and a done task with one.
- **Task dates:** `mwp` and the GUI write `updated:` on each change of status (start, block, review, release, close, plan approval) and `closed:` when a task is closed. An edit by hand does not move them.
- **Lint:**
  - a module's `code:` or `tests:` path that does not exist yet is a warning, not an error; so is a registered TEST when none of the `tests` paths exists yet;
  - `map.md` with its `mwp:map` markers more than once is an error (the second table was never updated);
  - `PROJECT.md`, `constitution.md`, `map.md` and `glossary.md` carry `kind:` in front matter: a wrong kind or a date that is not `YYYY-MM-DD` is an error, a missing header (older projects) or a project without `created:` is a warning. `mwp init` fills `created:` and the constitution's `updated:` with today.
- **Constitution and skill:** the rule of 3 (stop after 3 failed fix attempts on a test or criterion), replanning, the size table and the stop rules as brakes, not punishments.
- **Fix:** the next free ID no longer counts the example IDs (`T-0043`) in the constitution, now that it has front matter.

### GUI
- **Task sizes** in New task and the board filter.
- **Replan:** a task in progress, blocked or in review can be closed as `replanned` (task header or card menu). It shows in Done with its own icon and can no longer be dragged.
- **New project** has a **Template** choice, with each package's name, details and thumbnail; the dialog scrolls when the window is short.
- **Settings > Reload project** (renamed from Reload; it reads the project from disk again, which also happens every 2 seconds) and **Settings > Project agents:** moved from the File menu.
- **Project > Rebuild ID ledger:** runs `mwp ledger` and shows the last ID of each prefix, or why it could not (duplicate IDs).
- **Approve plan:** the header of an L, XL or XXL task without approval has **Approve plan**, which sets `plan_approved:` to today once `## Plan` is written.

## mwp 0.3.0 · mwp-gui 0.1.3 (2026-11-16)

- **Templates:** new tasks, modules, SPECs, ADRs and projects take their templates from `templates.tar.zst` next to `mwp-gui`, which the installer puts there.
- **Windows:** release builds no longer open a console window behind the GUI.
- **Spanish, Japanese, Korean and Chinese:** the GUI comes in these languages too (`es.json`, `ja.json`, `ko.json`, `zh-CN.json` for Simplified Chinese, `zh-TW.json` for Traditional Chinese). Their text uses the system's fonts (Yu Gothic, Malgun Gothic, Microsoft YaHei and Microsoft JhengHei on Windows; Hiragino, Apple SD Gothic Neo and PingFang on macOS; Noto Sans CJK on Linux), with the chosen language's font first; without them those characters show as boxes.
- **Fonts:** choosing a text font or a language no longer drops the icons when egui applies the change inside a frame.

## mwp-gui 0.1.2 (2026-11-14)

- **Appearance:** theme (dark, light or the system's), text size and text font in Settings > Preferences. The file header no longer runs under its buttons with larger text.
- **Settings > Preferences** and **languages:** the GUI in English or Brazilian Portuguese, from JSON files in `<config>/mwp-gui/lang/` that anyone can copy and translate for another language.
- **Help > About:** the project art, the versions of the GUI and of the core, the description and the license.
- **Conflicts with the disk:** **Show changes** in the conflict bar lists only the lines that differ, marked `-` (only in your edits) and `+` (only on disk).
- **Deleted files:** a file deleted on disk gets a bar with **Save again** and **Close tab**; Ctrl+S no longer brings it back by itself.
- **File > Go to file** (Ctrl+P): find a file by name, ID or title as you type.
- **Replace in project:** Replace all in the Search tab, after asking; files open in a tab change there and stay unsaved, the others are written.
- **Board filters** are remembered per project, with the open tabs. A module, agent or size the project no longer has is dropped.

## mwp-gui 0.1.1 (2026-11-13)

- **Installers:** `install.ps1 -Gui` / `install.sh --gui` also install `mwp-gui`.
- **Editor:** Replace (Ctrl+H) and Replace all in the find bar; Find in project (Ctrl+Shift+F) in a Search tab of the bottom panel; IDs in the preview link to their files, and Ctrl+click opens them in the source.
- **Tabs:** Ctrl+W, Ctrl+Tab / Ctrl+Shift+Tab, Ctrl+PageDown / Ctrl+PageUp; Close others and Close to the right, asking once for all unsaved tabs.
- **Board:** drag a card to another column to start, release or close the task; filters by title, ID or work code, module, agent and size.
- **Explorer:** rename files and folders (right-click, or F2 for the open file) and move them by dragging onto a folder of Modules or Content.
- **Headers:** Context of a task (what an agent loads, with a token estimate, like `mwp context`); Accept and Reject on a proposed ADR.
- **File > Project agents:** add agents to a project and update the skills of the ones already set up.
- **Window:** remembers its size and place, the panels, the collapsed sections, the editor mode and each project's open tabs.
- **Module types** `api` and `lib`, next to `domain`, `ui` and `infra`.

## mwp-gui 0.1.0 (2026-11-13)

The first version: an explorer of the `mwp/` files with folders and icons, a Markdown editor with tabs, highlighted source, preview and find, the task board with the task lifecycle (Start, Status, Close), New / Delete for modules, tasks, SPECs, ADRs and content, Update map.md, a warning when an agent changes a file you are editing, and New / Open project with the recent projects.
