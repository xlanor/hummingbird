SELECT
    id,
    title_sortable
FROM
    album
ORDER BY
    label COLLATE NOCASE DESC,
    catalog_number COLLATE NOCASE ASC,
    sort_date ASC;
