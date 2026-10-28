ALTER TABLE panel_settings ADD COLUMN user_ports TEXT NOT NULL DEFAULT 'pool'
    CHECK (user_ports IN ('locked', 'pool', 'choose'));
ALTER TABLE panel_settings ADD COLUMN network_needs_playit INTEGER NOT NULL DEFAULT 0
    CHECK (network_needs_playit IN (0, 1));
