INSERT INTO album (title, title_sortable, artist_id, image, thumb)
    VALUES ($1, $2, $3, $4, $5)
    ON CONFLICT (title, artist_id) DO NOTHING -- TODO: ideally we should have some way of updating this
    RETURNING id;
