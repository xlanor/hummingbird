SELECT id, title FROM album
WHERE artist_id = $1
ORDER BY COALESCE(release_date, '9999-12-31') ASC, release_year ASC, title_sortable ASC;
