CREATE TABLE IF NOT EXISTS app_user (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT,
    password_hash TEXT,
    oidc_issuer TEXT,
    oidc_subject TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_oidc ON app_user (oidc_issuer, oidc_subject);

-- Add user_id to playlist (nullable for backward compat with existing "Liked Songs")
ALTER TABLE playlist ADD COLUMN user_id INTEGER REFERENCES app_user(id);
CREATE INDEX IF NOT EXISTS idx_playlist_user ON playlist (user_id);
