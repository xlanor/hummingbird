SELECT
    id,
    title_sortable
FROM
    album
ORDER BY
    catalog_number COLLATE NOCASE DESC,
    sort_date ASC;
