# Changelog: mwp-gui

What changed in each version of the desktop editor.

## Unreleased

- **Windows:** release builds no longer open a console window behind the GUI.

## 0.1.2 (2026-11-14)

- **Appearance:** theme (dark, light or the system's), text size and text font in Settings > Preferences. The file header no longer runs under its buttons with larger text.
- **Settings > Preferences** and **languages:** the GUI in English or Brazilian Portuguese, from JSON files in `<config>/mwp-gui/lang/` that anyone can copy and translate for another language.
- **Help > About:** the project art, the versions of the GUI and of the core, the description and the license.
- **Conflicts with the disk:** **Show changes** in the conflict bar lists only the lines that differ, marked `-` (only in your edits) and `+` (only on disk).
- **Deleted files:** a file deleted on disk gets a bar with **Save again** and **Close tab**; Ctrl+S no longer brings it back by itself.
- **File > Go to file** (Ctrl+P): find a file by name, ID or title as you type.
- **Replace in project:** Replace all in the Search tab, after asking; files open in a tab change there and stay unsaved, the others are written.
- **Board filters** are remembered per project, with the open tabs. A module, agent or size the project no longer has is dropped.

## 0.1.1 (2026-11-13)

- **Installers:** `install.ps1 -Gui` / `install.sh --gui` also install `mwp-gui`.
- **Editor:** Replace (Ctrl+H) and Replace all in the find bar; Find in project (Ctrl+Shift+F) in a Search tab of the bottom panel; IDs in the preview link to their files, and Ctrl+click opens them in the source.
- **Tabs:** Ctrl+W, Ctrl+Tab / Ctrl+Shift+Tab, Ctrl+PageDown / Ctrl+PageUp; Close others and Close to the right, asking once for all unsaved tabs.
- **Board:** drag a card to another column to start, release or close the task; filters by title, ID or work code, module, agent and size.
- **Explorer:** rename files and folders (right-click, or F2 for the open file) and move them by dragging onto a folder of Modules or Content.
- **Headers:** Context of a task (what an agent loads, with a token estimate, like `mwp context`); Accept and Reject on a proposed ADR.
- **File > Project agents:** add agents to a project and update the skills of the ones already set up.
- **Window:** remembers its size and place, the panels, the collapsed sections, the editor mode and each project's open tabs.
- **Module types** `api` and `lib`, next to `domain`, `ui` and `infra`.

## 0.1.0 (2026-11-13)

The first version: an explorer of the `mwp/` files with folders and icons, a Markdown editor with tabs, highlighted source, preview and find, the task board with the task lifecycle (Start, Status, Close), New / Delete for modules, tasks, SPECs, ADRs and content, Update map.md, a warning when an agent changes a file you are editing, and New / Open project with the recent projects.
