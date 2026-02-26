INSERT INTO album (title, title_sortable, artist_id, image, thumb, release_date, date_precision, label, catalog_number, isrc, mbid, vinyl_numbering)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
    ON CONFLICT (title, artist_id, mbid) DO UPDATE SET
        title = EXCLUDED.title,
        title_sortable = EXCLUDED.title_sortable,
        artist_id = EXCLUDED.artist_id,
        image = EXCLUDED.image,
        thumb = EXCLUDED.thumb,
        release_date = EXCLUDED.release_date,
        date_precision = EXCLUDED.date_precision,
        label = EXCLUDED.label,
        catalog_number = EXCLUDED.catalog_number,
        isrc = EXCLUDED.isrc,
        mbid = EXCLUDED.mbid,
        vinyl_numbering = vinyl_numbering OR EXCLUDED.vinyl_numbering
    RETURNING id;

