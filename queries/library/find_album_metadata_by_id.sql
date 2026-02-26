SELECT
    id,
    title,
    title_sortable,
    artist_id,
    release_date,
    date_precision,
    created_at,
    label,
    catalog_number,
    isrc,
    vinyl_numbering
FROM album
WHERE id = $1;
