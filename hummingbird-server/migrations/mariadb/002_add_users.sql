CREATE TABLE IF NOT EXISTS app_user (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    display_name TEXT,
    password_hash TEXT,
    oidc_issuer VARCHAR(255),
    oidc_subject VARCHAR(255),
    role VARCHAR(32) NOT NULL DEFAULT 'user',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY idx_user_oidc (oidc_issuer, oidc_subject)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE playlist ADD COLUMN user_id BIGINT;
ALTER TABLE playlist ADD FOREIGN KEY (user_id) REFERENCES app_user(id);
CREATE INDEX idx_playlist_user ON playlist (user_id);
