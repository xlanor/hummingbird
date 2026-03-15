CREATE TABLE IF NOT EXISTS app_user (
    id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT,
    password_hash TEXT,
    oidc_issuer TEXT,
    oidc_subject TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_oidc ON app_user (oidc_issuer, oidc_subject)
    WHERE oidc_issuer IS NOT NULL;

-- Add user_id to playlist
DO $$ BEGIN
    ALTER TABLE playlist ADD COLUMN user_id BIGINT REFERENCES app_user(id);
EXCEPTION WHEN duplicate_column THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_playlist_user ON playlist (user_id);
