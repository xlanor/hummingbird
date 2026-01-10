SELECT
    t.id,
    t.title_sortable,
    t.album_id,
    t.location
FROM
    track t
    LEFT JOIN album al ON t.album_id = al.id
    LEFT JOIN artist ar ON al.artist_id = ar.id
ORDER BY
    ar.name_sortable COLLATE NOCASE ASC,
    al.title_sortable COLLATE NOCASE ASC,
    t.disc_number ASC,
    t.track_number ASC;
