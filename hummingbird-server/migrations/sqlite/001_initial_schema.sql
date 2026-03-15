-- Consolidated schema from upstream Hummingbird migrations

CREATE TABLE IF NOT EXISTS artist (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    name_sortable TEXT NOT NULL,
    bio TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    image BLOB,
    image_mime TEXT,
    tags TEXT
);

CREATE TABLE IF NOT EXISTS album (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    artist_id INTEGER,
    release_date DATE,
    date_precision INTEGER,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    image BLOB,
    thumb BLOB,
    image_mime TEXT,
    tags TEXT,
    label TEXT,
    catalog_number TEXT,
    isrc TEXT,
    mbid TEXT DEFAULT 'none' NOT NULL,
    vinyl_numbering INTEGER DEFAULT 0 NOT NULL,
    FOREIGN KEY (artist_id) REFERENCES artist (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS album_title_artist_mbid ON album (title, artist_id, mbid);
CREATE INDEX IF NOT EXISTS album_release_date_idx ON album (release_date);
CREATE INDEX IF NOT EXISTS idx_artist_album_id ON album (artist_id, id);

CREATE TABLE IF NOT EXISTS track (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    album_id INTEGER,
    track_number INTEGER,
    disc_number INTEGER,
    duration INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    genres TEXT,
    tags TEXT,
    location TEXT NOT NULL UNIQUE,
    artist_names TEXT,
    folder TEXT,
    FOREIGN KEY (album_id) REFERENCES album (id)
);

CREATE INDEX IF NOT EXISTS idx_track_album_id ON track (album_id, id);

CREATE TABLE IF NOT EXISTS album_path (
    album_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    disc_num INTEGER DEFAULT -1 NOT NULL,
    FOREIGN KEY (album_id) REFERENCES album (id),
    PRIMARY KEY (album_id, disc_num)
);

CREATE TABLE IF NOT EXISTS playlist (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    type INTEGER NOT NULL CHECK(type IN (0, 1)) DEFAULT 0
);

CREATE TABLE IF NOT EXISTS playlist_item (
    id INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL,
    track_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    position INTEGER NOT NULL,
    FOREIGN KEY (playlist_id) REFERENCES playlist(id),
    FOREIGN KEY (track_id) REFERENCES track(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS playlist_item_playlist_id_track_id ON playlist_item(playlist_id, track_id);

-- Cascade triggers
CREATE TRIGGER IF NOT EXISTS delete_album_trigger AFTER DELETE ON track
BEGIN
    DELETE FROM album
    WHERE album.id = OLD.album_id
    AND NOT EXISTS (
        SELECT 1 FROM track WHERE track.album_id = OLD.album_id
    );
END;

CREATE TRIGGER IF NOT EXISTS delete_artist_trigger AFTER DELETE ON album
BEGIN
    DELETE FROM artist
    WHERE artist.id = OLD.artist_id
    AND NOT EXISTS (
        SELECT 1 FROM album WHERE album.artist_id = OLD.artist_id
    );
END;

CREATE TRIGGER IF NOT EXISTS delete_album_paths AFTER DELETE ON album
BEGIN
    DELETE FROM album_path WHERE album_path.album_id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS delete_album_path_trigger AFTER DELETE ON track
BEGIN
    DELETE FROM album_path
    WHERE album_path.path = OLD.folder
        AND album_path.disc_num = IFNULL(OLD.disc_number, -1)
        AND album_path.album_id = OLD.album_id
        AND NOT EXISTS (
            SELECT 1 FROM track
            WHERE track.folder = OLD.folder
                AND track.disc_number = OLD.disc_number
                AND track.album_id = OLD.album_id
        );
END;

CREATE TRIGGER IF NOT EXISTS update_track_album_cleanup AFTER UPDATE OF album_id ON track
WHEN OLD.album_id IS NOT NULL AND (NEW.album_id IS NULL OR OLD.album_id != NEW.album_id)
BEGIN
    DELETE FROM album_path
    WHERE album_path.path = OLD.folder
        AND album_path.disc_num = IFNULL(OLD.disc_number, -1)
        AND album_path.album_id = OLD.album_id
        AND NOT EXISTS (
            SELECT 1 FROM track
            WHERE track.folder = OLD.folder
              AND IFNULL(track.disc_number, -1) = IFNULL(OLD.disc_number, -1)
              AND track.album_id = OLD.album_id
        );

    DELETE FROM album
    WHERE album.id = OLD.album_id
    AND NOT EXISTS (
        SELECT 1 FROM track WHERE track.album_id = OLD.album_id
    );

    DELETE FROM artist
    WHERE NOT EXISTS (
        SELECT 1 FROM album WHERE album.artist_id = artist.id
    );
END;

-- System playlists
INSERT INTO playlist (name, type) VALUES ('Liked Songs', 1);
