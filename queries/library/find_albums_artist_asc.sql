SELECT
    id,
    title_sortable
FROM
    (
        SELECT
            p.id,
            p.title_sortable,
            p.release_date,
            p.release_year,
            a.name_sortable
        FROM
            album p
            JOIN artist a ON p.artist_id = a.id
        ORDER BY
            a.name_sortable COLLATE NOCASE ASC,
            COALESCE(p.release_date, printf('%04d-01-01', p.release_year)) ASC
    );
