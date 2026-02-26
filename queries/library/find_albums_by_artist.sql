SELECT id, title FROM album
WHERE artist_id = $1
ORDER BY sort_date ASC;
