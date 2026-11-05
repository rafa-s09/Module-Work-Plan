# Module map

> One line per module, generated from module front matter. Do not edit the table by hand:
> run `mwp remap` (or `mwp map --write`) to regenerate it and `mwp map --check` to verify it.
> Text outside the markers is preserved.

<!-- mwp:map:begin -->
| ID | Type | Summary | Code | Tests | Depends on |
|---|---|---|---|---|---|
| api | infra | The speaking trumpet. FastAPI endpoints over the domain, with one shape for every answer and every error. |  |  | auth, crew, voyage, hold, loot |
| auth | domain | The gangplank. Username and password login, sessions of two watches, and who may give which orders. |  |  | crew, db |
| crew | domain | The ship's articles. Who sails, at what rank, for how many shares, and what became of them. |  |  | db |
| db | infra | The strongroom. One SQLite file, numbered migrations, nothing ever deleted. |  |  |  |
| deck | ui | The deck. Angular screens that look like an old sea chart and a captain's logbook, with no business rules of their own. |  |  | api |
| hold | domain | Below deck. Rum, powder, rations and water, in whole units, never past what the ship can carry. |  |  | db |
| loot | domain | The plunder. Money in reales, compensation for the wounded, and the split by shares. |  |  | voyage, crew, db |
| voyage | domain | The course. A voyage's states from port to plunder and back, and who sails on it. |  |  | crew, hold, db |
<!-- mwp:map:end -->
