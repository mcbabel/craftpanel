-- Operations gain the kind `archive`: packing files of a server into a ZIP.
--
-- The kinds are a CHECK, and SQLite cannot alter a CHECK in place, so the table
-- is built anew exactly as 0016 left it, with one more word in that CHECK. The
-- order of the steps is the one 0016 explains: the rename carries the
-- self-reference along, the rows come over with parent_operation_id empty and
-- get it back afterwards, and the old table goes when nothing points at it.

ALTER TABLE operations RENAME TO operations_before_archive;

CREATE TABLE operations (
    id                   TEXT    PRIMARY KEY CHECK (length(id) = 26),
    server_id            TEXT    NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    kind                 TEXT    NOT NULL CHECK (kind IN (
                                     'server_create', 'server_delete', 'install_loader',
                                     'repair_content', 'reset_server', 'install_modpack',
                                     'install_content', 'update_content', 'change_game_version',
                                     'install_java', 'backup_create', 'backup_restore',
                                     'unarchive', 'archive')),
    state                TEXT    NOT NULL CHECK (state IN (
                                     'queued', 'ongoing', 'done', 'failed', 'cancelled')),
    phase                TEXT    CHECK (phase IS NULL OR phase IN (
                                     'analyzing', 'installing_loader', 'verifying',
                                     'running_installer', 'installing_java', 'installing_pack',
                                     'addons', 'writing_config')),
    progress             REAL    NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 1),
    message              TEXT,
    src                  TEXT,
    bytes_processed      INTEGER,
    files_processed      INTEGER,
    current_file         TEXT,
    error_code           TEXT,
    error_message        TEXT,
    error_step           TEXT    CHECK (error_step IS NULL OR error_step IN (
                                     'modloader', 'modpack', 'download', 'filesystem',
                                     'internal')),
    cancellable          INTEGER NOT NULL DEFAULT 0 CHECK (cancellable IN (0, 1)),
    target_id            TEXT    REFERENCES backups(id) ON DELETE SET NULL,
    parent_operation_id  TEXT    REFERENCES operations(id) ON DELETE SET NULL,
    started_by           TEXT    REFERENCES users(id) ON DELETE SET NULL,
    input                TEXT    CHECK (input IS NULL OR json_valid(input)),
    payload              TEXT    NOT NULL DEFAULT 'none'
                                 CHECK (payload IN ('none', 'expected', 'delivered')),
    applied_at           TEXT,
    created_at           TEXT    NOT NULL,
    started_at           TEXT,
    finished_at          TEXT,
    dismissed_at         TEXT,
    cancel_requested     INTEGER NOT NULL DEFAULT 0 CHECK (cancel_requested IN (0, 1)),
    progressed_at        TEXT,
    CHECK ((error_code IS NULL) = (error_step IS NULL)
           AND (error_code IS NULL) = (error_message IS NULL))
);

INSERT INTO operations (
    id, server_id, kind, state, phase, progress, message, src, bytes_processed,
    files_processed, current_file, error_code, error_message, error_step, cancellable,
    target_id, parent_operation_id, started_by, input, payload, applied_at, created_at,
    started_at, finished_at, dismissed_at, cancel_requested, progressed_at
)
SELECT
    id, server_id, kind, state, phase, progress, message, src, bytes_processed,
    files_processed, current_file, error_code, error_message, error_step, cancellable,
    target_id, NULL, started_by, input, payload, applied_at, created_at,
    started_at, finished_at, dismissed_at, cancel_requested, progressed_at
FROM operations_before_archive;

UPDATE operations
   SET parent_operation_id = (
       SELECT parent_operation_id FROM operations_before_archive AS before
        WHERE before.id = operations.id
   );

DROP TABLE operations_before_archive;

CREATE INDEX operations_server_id ON operations(server_id, id DESC);
CREATE INDEX operations_open ON operations(server_id) WHERE state IN ('queued', 'ongoing');
CREATE INDEX operations_target_id ON operations(target_id);

CREATE UNIQUE INDEX operations_one_open_backup_create
    ON operations(server_id) WHERE kind = 'backup_create' AND state IN ('queued', 'ongoing');
CREATE UNIQUE INDEX operations_one_open_backup_restore
    ON operations(server_id) WHERE kind = 'backup_restore' AND state IN ('queued', 'ongoing');
CREATE UNIQUE INDEX operations_one_open_archive
    ON operations(server_id) WHERE kind = 'archive' AND state IN ('queued', 'ongoing');
