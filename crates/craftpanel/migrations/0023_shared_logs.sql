-- Logs put on mclo.gs from this panel, so the people who shared them can take them down
-- again. mclo.gs hands out the token that deletes a log exactly once, with the upload;
-- it is kept here and never leaves the panel.

CREATE TABLE shared_logs (
    id          TEXT PRIMARY KEY CHECK (length(id) = 26),
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    shared_by   TEXT REFERENCES users(id) ON DELETE SET NULL,
    mclogs_id   TEXT NOT NULL,
    url         TEXT NOT NULL,
    token       TEXT,
    -- The log file's path, or 'console' for what was on screen.
    source      TEXT NOT NULL CHECK (length(source) BETWEEN 1 AND 512),
    created_at  TEXT NOT NULL
);

CREATE INDEX shared_logs_server_created_at ON shared_logs(server_id, created_at DESC);
