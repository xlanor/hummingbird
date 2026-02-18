SELECT
    id,
    title_sortable
FROM
    (
        SELECT
            id,
            title_sortable,
            release_date,
            release_year
        FROM
            album
        ORDER BY
            COALESCE(release_date, printf('%04d-01-01', release_year)) DESC,
            title_sortable COLLATE NOCASE ASC
    );
