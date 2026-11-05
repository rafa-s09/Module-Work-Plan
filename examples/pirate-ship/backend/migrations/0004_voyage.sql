CREATE TABLE voyages (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    destination TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'docked'
);

CREATE TABLE voyage_events (
    id INTEGER PRIMARY KEY,
    voyage_id INTEGER NOT NULL REFERENCES voyages (id),
    state TEXT NOT NULL,
    by_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    at TEXT NOT NULL
);

CREATE TABLE voyage_roster (
    voyage_id INTEGER NOT NULL REFERENCES voyages (id),
    crew_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    quarter_shares INTEGER NOT NULL,
    PRIMARY KEY (voyage_id, crew_member_id)
);
