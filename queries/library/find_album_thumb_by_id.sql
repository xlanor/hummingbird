SELECT
    id,
    title,
    title_sortable,
    artist_id,
    release_date,
    release_year,
    created_at,
    thumb,
    label,
    catalog_number,
    isrc,
    vinyl_numbering
FROM album
WHERE id = $1;
