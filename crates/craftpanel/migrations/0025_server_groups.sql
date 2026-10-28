CREATE TABLE server_groups (
    id          TEXT PRIMARY KEY CHECK (length(id) = 26),
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 64),
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX server_groups_user_name ON server_groups(user_id, name COLLATE NOCASE);

CREATE TABLE server_group_members (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id  TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    group_id   TEXT NOT NULL REFERENCES server_groups(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX server_group_members_group ON server_group_members(group_id);

ALTER TABLE servers ADD COLUMN last_started_at TEXT;
