SELECT
    p.id,
    p.title_sortable
FROM
    album p
    JOIN artist a ON p.artist_id = a.id
ORDER BY
    a.name_sortable COLLATE NOCASE DESC,
    p.sort_date ASC;
