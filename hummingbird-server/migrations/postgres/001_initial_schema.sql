-- Consolidated schema ported from upstream SQLite migrations to PostgreSQL

CREATE TABLE IF NOT EXISTS artist (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    name_sortable TEXT NOT NULL,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    image BYTEA,
    image_mime TEXT,
    tags TEXT
);

CREATE TABLE IF NOT EXISTS album (
    id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    artist_id BIGINT REFERENCES artist(id),
    release_date DATE,
    date_precision INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    image BYTEA,
    thumb BYTEA,
    image_mime TEXT,
    tags TEXT,
    label TEXT,
    catalog_number TEXT,
    isrc TEXT,
    mbid TEXT NOT NULL DEFAULT 'none',
    vinyl_numbering BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE UNIQUE INDEX IF NOT EXISTS album_title_artist_mbid ON album (title, artist_id, mbid);
CREATE INDEX IF NOT EXISTS album_release_date_idx ON album (release_date);
CREATE INDEX IF NOT EXISTS idx_artist_album_id ON album (artist_id, id);

CREATE TABLE IF NOT EXISTS track (
    id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    title_sortable TEXT NOT NULL,
    album_id BIGINT REFERENCES album(id),
    track_number INTEGER,
    disc_number INTEGER,
    duration BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    genres TEXT,
    tags TEXT,
    location TEXT NOT NULL UNIQUE,
    artist_names TEXT,
    folder TEXT
);

CREATE INDEX IF NOT EXISTS idx_track_album_id ON track (album_id, id);

CREATE TABLE IF NOT EXISTS album_path (
    album_id BIGINT NOT NULL REFERENCES album(id),
    path TEXT NOT NULL,
    disc_num INTEGER NOT NULL DEFAULT -1,
    PRIMARY KEY (album_id, disc_num)
);

CREATE TABLE IF NOT EXISTS playlist (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    type INTEGER NOT NULL DEFAULT 0 CHECK (type IN (0, 1))
);

CREATE TABLE IF NOT EXISTS playlist_item (
    id BIGSERIAL PRIMARY KEY,
    playlist_id BIGINT NOT NULL REFERENCES playlist(id),
    track_id BIGINT NOT NULL REFERENCES track(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    position INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS playlist_item_playlist_id_track_id ON playlist_item(playlist_id, track_id);

-- Postgres uses functions + triggers instead of SQLite's simple trigger syntax.
-- Cascade cleanup: when a track is deleted, remove empty albums and artists.

CREATE OR REPLACE FUNCTION cleanup_after_track_delete() RETURNS TRIGGER AS $$
BEGIN
    -- Clean up album_path for this track's folder/disc
    DELETE FROM album_path
    WHERE path = OLD.folder
      AND disc_num = COALESCE(OLD.disc_number, -1)
      AND album_id = OLD.album_id
      AND NOT EXISTS (
          SELECT 1 FROM track
          WHERE folder = OLD.folder
            AND disc_number IS NOT DISTINCT FROM OLD.disc_number
            AND album_id = OLD.album_id
      );

    -- Clean up empty album
    IF OLD.album_id IS NOT NULL THEN
        DELETE FROM album
        WHERE id = OLD.album_id
          AND NOT EXISTS (SELECT 1 FROM track WHERE album_id = OLD.album_id);
    END IF;

    -- Clean up orphaned artists (via album trigger below)
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_after_album_delete() RETURNS TRIGGER AS $$
BEGIN
    -- Clean up album_path entries
    DELETE FROM album_path WHERE album_id = OLD.id;

    -- Clean up empty artist
    IF OLD.artist_id IS NOT NULL THEN
        DELETE FROM artist
        WHERE id = OLD.artist_id
          AND NOT EXISTS (SELECT 1 FROM album WHERE artist_id = OLD.artist_id);
    END IF;

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_after_track_album_update() RETURNS TRIGGER AS $$
BEGIN
    IF OLD.album_id IS NOT NULL AND (NEW.album_id IS NULL OR OLD.album_id != NEW.album_id) THEN
        DELETE FROM album_path
        WHERE path = OLD.folder
          AND disc_num = COALESCE(OLD.disc_number, -1)
          AND album_id = OLD.album_id
          AND NOT EXISTS (
              SELECT 1 FROM track
              WHERE folder = OLD.folder
                AND COALESCE(disc_number, -1) = COALESCE(OLD.disc_number, -1)
                AND album_id = OLD.album_id
          );

        DELETE FROM album
        WHERE id = OLD.album_id
          AND NOT EXISTS (SELECT 1 FROM track WHERE album_id = OLD.album_id);

        DELETE FROM artist
        WHERE NOT EXISTS (SELECT 1 FROM album WHERE artist_id = artist.id);
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_track_delete ON track;
CREATE TRIGGER trg_track_delete AFTER DELETE ON track
    FOR EACH ROW EXECUTE FUNCTION cleanup_after_track_delete();

DROP TRIGGER IF EXISTS trg_album_delete ON album;
CREATE TRIGGER trg_album_delete AFTER DELETE ON album
    FOR EACH ROW EXECUTE FUNCTION cleanup_after_album_delete();

DROP TRIGGER IF EXISTS trg_track_album_update ON track;
CREATE TRIGGER trg_track_album_update AFTER UPDATE OF album_id ON track
    FOR EACH ROW EXECUTE FUNCTION cleanup_after_track_album_update();

-- System playlists
INSERT INTO playlist (name, type) VALUES ('Liked Songs', 1) ON CONFLICT (name) DO NOTHING;
