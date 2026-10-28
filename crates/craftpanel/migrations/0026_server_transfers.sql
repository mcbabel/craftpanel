CREATE TABLE server_transfers (
    id            TEXT PRIMARY KEY CHECK (length(id) = 26),
    server_id     TEXT NOT NULL UNIQUE REFERENCES servers(id) ON DELETE CASCADE,
    from_user_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    TEXT NOT NULL,
    CHECK (from_user_id <> to_user_id)
);

CREATE INDEX server_transfers_to ON server_transfers(to_user_id);
CREATE INDEX server_transfers_from ON server_transfers(from_user_id);
