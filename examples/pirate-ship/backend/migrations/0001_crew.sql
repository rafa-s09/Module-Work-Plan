CREATE TABLE crew_members (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    rank TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'aboard',
    signed_at TEXT NOT NULL
);
CREATE UNIQUE INDEX crew_one_name_aboard ON crew_members (name COLLATE NOCASE) WHERE status = 'aboard';
CREATE UNIQUE INDEX crew_one_officer_aboard ON crew_members (rank)
    WHERE status = 'aboard' AND rank IN ('captain', 'quartermaster');

CREATE TABLE crew_changes (
    id INTEGER PRIMARY KEY,
    crew_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    field TEXT NOT NULL,
    old_value TEXT NOT NULL,
    new_value TEXT NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);
