CREATE TABLE hold_changes (
    id INTEGER PRIMARY KEY,
    item TEXT NOT NULL,
    amount INTEGER NOT NULL,
    reason TEXT NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);
