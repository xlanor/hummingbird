SELECT t.* FROM track t
JOIN album al ON t.album_id = al.id
WHERE al.artist_id = $1
ORDER BY COALESCE(al.release_date, printf('%04d-01-01', al.release_year)) ASC,
         al.id ASC, t.disc_number ASC, t.track_number ASC;
