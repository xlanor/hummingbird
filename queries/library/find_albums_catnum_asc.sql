SELECT
    id,
    title_sortable
FROM
    album
ORDER BY
    catalog_number COLLATE NOCASE ASC,
    p.sort_date ASC;
