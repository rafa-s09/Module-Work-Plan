# Modules

One file per part of the system, describing how it works **today**: rules (`FR-01`, `NFR-01`), the tests that guard them, contracts and known pitfalls.

- Start from `_template.md`. The `id:` in the front matter is how tasks and other modules cite it.
- Subfolders are fine (`domain/order.md`, `ui/checkout.md`).
- Update the module in the same task that changes the behavior. No history here: the changelog row points to the task.
- After adding or changing a module, run `mwp remap` to check it and refresh `map.md`.
