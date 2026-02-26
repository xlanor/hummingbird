SELECT
    id,
    title_sortable,
    release_date,
    release_year
FROM
    album
ORDER BY
    sort_date DESC,
    title_sortable COLLATE NOCASE ASC;
