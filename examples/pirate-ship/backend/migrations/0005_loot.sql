CREATE TABLE plunder (
    id INTEGER PRIMARY KEY,
    voyage_id INTEGER NOT NULL REFERENCES voyages (id),
    amount_reales INTEGER NOT NULL CHECK (amount_reales > 0),
    description TEXT NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);

CREATE TABLE compensations (
    id INTEGER PRIMARY KEY,
    voyage_id INTEGER NOT NULL REFERENCES voyages (id),
    crew_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    injury TEXT NOT NULL,
    amount_reales INTEGER NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);

CREATE TABLE splits (
    id INTEGER PRIMARY KEY,
    voyage_id INTEGER NOT NULL UNIQUE REFERENCES voyages (id),
    plunder_reales INTEGER NOT NULL,
    compensation_reales INTEGER NOT NULL,
    leftover_reales INTEGER NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);

CREATE TABLE split_lines (
    split_id INTEGER NOT NULL REFERENCES splits (id),
    crew_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    quarter_shares INTEGER NOT NULL,
    compensation_reales INTEGER NOT NULL,
    share_reales INTEGER NOT NULL,
    total_reales INTEGER NOT NULL,
    paid_to TEXT NOT NULL CHECK (paid_to IN ('member', 'chest')),
    PRIMARY KEY (split_id, crew_member_id)
);

CREATE TABLE chest (
    id INTEGER PRIMARY KEY,
    split_id INTEGER NOT NULL REFERENCES splits (id),
    amount_reales INTEGER NOT NULL,
    at TEXT NOT NULL
);
