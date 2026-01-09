SELECT
    t.id,
    t.title_sortable
FROM
    track t
ORDER BY
    t.disc_number DESC,
    t.track_number DESC;
