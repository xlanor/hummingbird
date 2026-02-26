SELECT a.id, a.name,
       (SELECT COUNT(*) FROM album WHERE artist_id = a.id) AS album_count,
       (SELECT COUNT(*) FROM track t JOIN album al ON t.album_id = al.id WHERE al.artist_id = a.id) AS track_count
FROM artist a
WHERE a.id = $1;
