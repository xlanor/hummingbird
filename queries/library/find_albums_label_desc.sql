SELECT
    id,
    title_sortable
FROM
    (
        SELECT
            id,
            title_sortable,
            label,
            catalog_number,
            release_date,
            release_year
        FROM
            album
        ORDER BY
            label COLLATE NOCASE DESC,
            catalog_number COLLATE NOCASE ASC,
            COALESCE(release_date, printf('%04d-01-01', release_year)) ASC
    );
