SELECT
    t.id,
    t.title_sortable
FROM
    track t
ORDER BY
    t.title_sortable COLLATE NOCASE ASC;
