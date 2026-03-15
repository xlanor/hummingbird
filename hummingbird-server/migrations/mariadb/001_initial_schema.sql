-- Consolidated schema ported from upstream SQLite migrations to MariaDB

CREATE TABLE IF NOT EXISTS artist (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name TEXT NOT NULL,
    name_sortable TEXT NOT NULL,
    bio TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    image LONGBLOB,
    image_mime TEXT,
    tags TEXT,
    UNIQUE KEY uk_artist_name (name(255))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS album (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    artist_id BIGINT,
    release_date DATE,
    date_precision INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    image LONGBLOB,
    thumb LONGBLOB,
    image_mime TEXT,
    tags TEXT,
    label TEXT,
    catalog_number TEXT,
    isrc TEXT,
    mbid VARCHAR(255) NOT NULL DEFAULT 'none',
    vinyl_numbering BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (artist_id) REFERENCES artist(id),
    UNIQUE KEY album_title_artist_mbid (title(191), artist_id, mbid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE INDEX idx_album_release_date ON album (release_date);
CREATE INDEX idx_artist_album_id ON album (artist_id, id);

CREATE TABLE IF NOT EXISTS track (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    album_id BIGINT,
    track_number INTEGER,
    disc_number INTEGER,
    duration BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    genres TEXT,
    tags TEXT,
    location VARCHAR(1024) NOT NULL,
    artist_names TEXT,
    folder TEXT,
    UNIQUE KEY uk_track_location (location),
    FOREIGN KEY (album_id) REFERENCES album(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE INDEX idx_track_album_id ON track (album_id, id);

CREATE TABLE IF NOT EXISTS album_path (
    album_id BIGINT NOT NULL,
    path TEXT NOT NULL,
    disc_num INTEGER NOT NULL DEFAULT -1,
    PRIMARY KEY (album_id, disc_num),
    FOREIGN KEY (album_id) REFERENCES album(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS playlist (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    type INTEGER NOT NULL DEFAULT 0 CHECK (type IN (0, 1))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS playlist_item (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    playlist_id BIGINT NOT NULL,
    track_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    position INTEGER NOT NULL,
    UNIQUE KEY playlist_item_playlist_id_track_id (playlist_id, track_id),
    FOREIGN KEY (playlist_id) REFERENCES playlist(id),
    FOREIGN KEY (track_id) REFERENCES track(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Note: MariaDB does not support trigger-based cascade cleanup as cleanly as
-- SQLite/Postgres. The application layer handles orphan cleanup for MariaDB.
-- However, we add basic triggers for album_path cleanup.

-- System playlists
INSERT IGNORE INTO playlist (name, type) VALUES ('Liked Songs', 1);
