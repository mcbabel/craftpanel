DROP TRIGGER playit_tunnel_released;

CREATE TABLE playit_tunnels_new (
    id          TEXT PRIMARY KEY CHECK (length(id) = 26),
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tunnel_id   TEXT UNIQUE,
    kind        TEXT NOT NULL DEFAULT 'minecraft-java'
                     CHECK (kind IN ('minecraft-java', 'minecraft-bedrock', 'tcp', 'udp')),
    label       TEXT CHECK (label IS NULL OR length(label) BETWEEN 1 AND 32),
    local_port  INTEGER NOT NULL CHECK (local_port BETWEEN 1024 AND 65535),
    state       TEXT NOT NULL CHECK (state IN ('pending', 'online', 'offline',
                                               'missing', 'failed')),
    addresses   TEXT NOT NULL DEFAULT '[]',
    detail      TEXT,
    created_at  TEXT NOT NULL,
    checked_at  TEXT,
    UNIQUE (server_id, kind, local_port)
);

INSERT INTO playit_tunnels_new
    (id, server_id, user_id, tunnel_id, kind, label, local_port, state, addresses, detail,
     created_at, checked_at)
SELECT server_id, server_id, user_id, tunnel_id, 'minecraft-java', NULL, local_port, state,
       addresses, detail, created_at, checked_at
  FROM playit_tunnels;

DROP TABLE playit_tunnels;
ALTER TABLE playit_tunnels_new RENAME TO playit_tunnels;
CREATE INDEX playit_tunnels_user ON playit_tunnels(user_id);
CREATE INDEX playit_tunnels_server ON playit_tunnels(server_id);

ALTER TABLE playit_released ADD COLUMN local_port INTEGER;

CREATE TRIGGER playit_tunnel_released
AFTER DELETE ON playit_tunnels
WHEN OLD.tunnel_id IS NOT NULL
BEGIN
    INSERT OR IGNORE INTO playit_released (user_id, tunnel_id, released_at, local_port)
    VALUES (OLD.user_id, OLD.tunnel_id, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), OLD.local_port);
END;

CREATE VIEW ports_in_use AS
    SELECT port FROM allocations
    UNION SELECT local_port FROM playit_tunnels
    UNION SELECT local_port FROM playit_released WHERE local_port IS NOT NULL;

CREATE TRIGGER playit_tunnel_needs_its_port
BEFORE INSERT ON playit_tunnels
WHEN NEW.id <> NEW.server_id
 AND NOT EXISTS (SELECT 1 FROM allocations
                  WHERE server_id = NEW.server_id AND port = NEW.local_port)
BEGIN
    SELECT RAISE(ABORT, 'playit_port_not_held');
END;

CREATE TRIGGER allocation_not_on_a_published_port
BEFORE INSERT ON allocations
WHEN EXISTS (SELECT 1 FROM playit_tunnels
              WHERE local_port = NEW.port AND server_id <> NEW.server_id)
  OR EXISTS (SELECT 1 FROM playit_released WHERE local_port = NEW.port)
BEGIN
    SELECT RAISE(ABORT, 'port_still_published');
END;
