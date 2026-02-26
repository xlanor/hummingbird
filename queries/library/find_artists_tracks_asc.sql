SELECT a.id FROM artist a
LEFT JOIN album al ON al.artist_id = a.id
LEFT JOIN track t ON t.album_id = al.id
GROUP BY a.id
ORDER BY COUNT(t.id) ASC, a.name_sortable ASC;
