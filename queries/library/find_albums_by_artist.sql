SELECT id, title FROM album
WHERE artist_id = $1
ORDER BY release_date ASC;
