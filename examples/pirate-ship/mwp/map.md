# Module map

> One line per module, generated from module front matter. Do not edit the table by hand:
> run `mwp remap` (or `mwp map --write`) to regenerate it and `mwp map --check` to verify it.
> Text outside the markers is preserved.

<!-- mwp:map:begin -->
| ID | Type | Summary | Code | Tests | Depends on |
|---|---|---|---|---|---|
| api | infra | The speaking trumpet. FastAPI endpoints over the domain, with one shape for every answer and every error. | backend/app/api, backend/app/main.py, backend/app/errors.py | backend/tests/api | auth, crew, voyage, hold, loot |
| auth | domain | The gangplank. Username and password login, sessions of two watches, and who may give which orders. | backend/app/auth, backend/migrations/0002_auth.sql | backend/tests/auth | crew, db |
| crew | domain | The ship's articles. Who sails, at what rank, for how many shares, and what became of them. | backend/app/crew, backend/migrations/0001_crew.sql | backend/tests/crew | db |
| db | infra | The strongroom. One SQLite file, numbered migrations, nothing ever deleted. | backend/app/db, backend/migrations | backend/tests/db |  |
| deck | ui | The deck. Angular screens that look like an old sea chart and a captain's logbook, with no business rules of their own. | frontend/src/app, frontend/src/styles, frontend/src/styles.css, frontend/public/theme | frontend/src/app/app.spec.ts | api |
| hold | domain | Below deck. Rum, powder, rations and water, in whole units, never past what the ship can carry. | backend/app/hold, backend/migrations/0003_hold.sql | backend/tests/hold | db |
| loot | domain | The plunder. Money in reales, compensation for the wounded, and the split by shares. | backend/app/loot, backend/migrations/0005_loot.sql | backend/tests/loot | voyage, db |
| voyage | domain | The course. A voyage's states from port to plunder and back, and who sails on it. | backend/app/voyage, backend/migrations/0004_voyage.sql | backend/tests/voyage | crew, hold, db |
<!-- mwp:map:end -->
