SELECT
    t.id,
    t.title_sortable
FROM
    track t
ORDER BY
    t.disc_number ASC,
    t.track_number ASC;
