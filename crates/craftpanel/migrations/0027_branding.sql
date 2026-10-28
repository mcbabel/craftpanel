CREATE TABLE branding (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    name        TEXT CHECK (name IS NULL OR length(trim(name)) BETWEEN 1 AND 40),
    logo        BLOB,
    logo_type   TEXT CHECK (logo_type IS NULL OR logo_type IN ('image/png', 'image/jpeg', 'image/webp')),
    updated_at  TEXT NOT NULL,
    CHECK ((logo IS NULL) = (logo_type IS NULL))
);
