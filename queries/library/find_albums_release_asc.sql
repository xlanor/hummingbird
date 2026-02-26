SELECT
    id,
    title_sortable,
    release_date,
    release_year
FROM
    album
ORDER BY
    sort_date ASC,
    title_sortable COLLATE NOCASE ASC;
