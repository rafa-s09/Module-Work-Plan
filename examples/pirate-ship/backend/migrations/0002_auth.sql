CREATE TABLE accounts (
    crew_member_id INTEGER PRIMARY KEY REFERENCES crew_members (id),
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash BLOB NOT NULL,
    password_salt BLOB NOT NULL
);

CREATE TABLE sessions (
    token_sha256 TEXT PRIMARY KEY,
    crew_member_id INTEGER NOT NULL REFERENCES crew_members (id),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    ended_at TEXT
);

-- Keyed by the typed username, account or not, so a lock never reveals which names are real.
CREATE TABLE login_failures (
    username TEXT PRIMARY KEY,
    count INTEGER NOT NULL,
    locked_until TEXT
);
