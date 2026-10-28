CREATE TABLE servers_with_more_loaders (
    id                   TEXT    PRIMARY KEY CHECK (length(id) = 26),
    name                 TEXT    NOT NULL CHECK (length(name) BETWEEN 1 AND 64),
    owner_id             TEXT    NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    status               TEXT    NOT NULL CHECK (status IN (
                                     'installing', 'available', 'broken', 'deleting')),
    loader               TEXT    CHECK (loader IS NULL OR loader IN (
                                     'vanilla', 'paper', 'folia', 'purpur', 'leaf',
                                     'fabric', 'velocity', 'neoforge', 'quilt', 'forge',
                                     'craftbukkit', 'limbo')),
    loader_version       TEXT,
    game_version         TEXT,
    memory_mib           INTEGER NOT NULL CHECK (memory_mib >= 512),
    update_channel       TEXT    NOT NULL DEFAULT 'release'
                                 CHECK (update_channel IN ('release', 'beta', 'alpha')),
    flows_intro          INTEGER NOT NULL DEFAULT 1 CHECK (flows_intro IN (0, 1)),
    java_major           INTEGER CHECK (java_major IS NULL OR java_major > 0),
    jre_vendor           TEXT    CHECK (jre_vendor IS NULL
                                        OR jre_vendor IN ('temurin', 'corretto', 'graal')),
    extra_flags          TEXT    NOT NULL DEFAULT '[]' CHECK (json_valid(extra_flags)),
    stripped_flags       TEXT    NOT NULL DEFAULT '[]' CHECK (json_valid(stripped_flags)),
    restart_required     INTEGER NOT NULL DEFAULT 0 CHECK (restart_required IN (0, 1)),
    supervisor_token     TEXT,
    console_seq          INTEGER NOT NULL DEFAULT 0,
    operations_revision  INTEGER NOT NULL DEFAULT 0,
    updates_checked_at   TEXT,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL,
    power_state          TEXT    NOT NULL DEFAULT 'stopped'
                                 CHECK (power_state IN ('stopped', 'starting', 'running',
                                                        'stopping', 'crashed')),
    running_since        TEXT,
    exit_code            INTEGER,
    oom_killed           INTEGER NOT NULL DEFAULT 0 CHECK (oom_killed IN (0, 1))
);

INSERT INTO servers_with_more_loaders (
    id, name, owner_id, status, loader, loader_version, game_version, memory_mib,
    update_channel, flows_intro, java_major, jre_vendor, extra_flags, stripped_flags,
    restart_required, supervisor_token, console_seq, operations_revision, updates_checked_at,
    created_at, updated_at, power_state, running_since, exit_code, oom_killed
)
SELECT
    id, name, owner_id, status, loader, loader_version, game_version, memory_mib,
    update_channel, flows_intro, java_major, jre_vendor, extra_flags, stripped_flags,
    restart_required, supervisor_token, console_seq, operations_revision, updates_checked_at,
    created_at, updated_at, power_state, running_since, exit_code, oom_killed
FROM servers;

DROP TABLE servers;

ALTER TABLE servers_with_more_loaders RENAME TO servers;

CREATE INDEX servers_owner_id ON servers(owner_id);
CREATE INDEX servers_owner_memory ON servers(owner_id, status);
