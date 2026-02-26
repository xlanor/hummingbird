SELECT a.id FROM artist a
LEFT JOIN album al ON al.artist_id = a.id
GROUP BY a.id
ORDER BY COUNT(al.id) ASC, a.name_sortable ASC;
