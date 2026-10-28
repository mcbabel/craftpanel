CREATE TABLE shared_logs (
    id          TEXT PRIMARY KEY CHECK (length(id) = 26),
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    shared_by   TEXT REFERENCES users(id) ON DELETE SET NULL,
    mclogs_id   TEXT NOT NULL,
    url         TEXT NOT NULL,
    token       TEXT,
    source      TEXT NOT NULL CHECK (length(source) BETWEEN 1 AND 512),
    created_at  TEXT NOT NULL
);

CREATE INDEX shared_logs_server_created_at ON shared_logs(server_id, created_at DESC);
