use hummingbird_server::db::sqlite::SqliteRepository;
use hummingbird_server::db::Repository;
use hummingbird_server::models::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

async fn setup_repo() -> SqliteRepository {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("failed to create pool");

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.expect("failed to run migrations");
    repo
}

/// Insert a test artist + album + track and return their IDs.
async fn seed_data(repo: &SqliteRepository) -> (i64, i64, i64) {
    let artist_id = repo.upsert_artist("Test Artist").await.unwrap();
    let album = ScannedAlbum {
        title: "Test Album".into(),
        title_sortable: "test album".into(),
        artist_id,
        image: None,
        thumb: None,
        release_date: Some("2023-01-15".into()),
        date_precision: Some(1),
        label: Some("Test Label".into()),
        catalog_number: Some("CAT-001".into()),
        isrc: None,
        mbid: "none".into(),
        vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();
    let track = ScannedTrack {
        title: "Test Track".into(),
        title_sortable: "test track".into(),
        album_id: Some(album_id),
        track_number: Some(1),
        disc_number: Some(1),
        duration: 240000,
        location: "/music/test/track1.flac".into(),
        genres: Some("Rock".into()),
        artist_names: Some("Test Artist".into()),
        folder: Some("/music/test".into()),
    };
    let track_id = repo.upsert_track(&track).await.unwrap();
    (artist_id, album_id, track_id)
}

// ─── Artist Tests ───

#[tokio::test]
async fn test_upsert_artist_creates_new() {
    let repo = setup_repo().await;
    let id = repo.upsert_artist("New Artist").await.unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_upsert_artist_idempotent() {
    let repo = setup_repo().await;
    let id1 = repo.upsert_artist("Same Artist").await.unwrap();
    let id2 = repo.upsert_artist("Same Artist").await.unwrap();
    assert_eq!(id1, id2);
}

#[tokio::test]
async fn test_get_artist() {
    let repo = setup_repo().await;
    let (artist_id, _, _) = seed_data(&repo).await;
    let artist = repo.get_artist(artist_id).await.unwrap();
    assert_eq!(artist.name, "Test Artist");
    assert_eq!(artist.name_sortable, "test artist");
}

#[tokio::test]
async fn test_get_artist_not_found() {
    let repo = setup_repo().await;
    let result = repo.get_artist(99999).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_artists_sorted_by_name() {
    let repo = setup_repo().await;
    repo.upsert_artist("Zebra").await.unwrap();
    repo.upsert_artist("Alpha").await.unwrap();
    repo.upsert_artist("Middle").await.unwrap();

    let artists = repo.list_artists(ArtistSort::Name, SortOrder::Asc).await.unwrap();
    assert_eq!(artists.len(), 3);
    assert_eq!(artists[0].name, "Alpha");
    assert_eq!(artists[2].name, "Zebra");

    let artists_desc = repo.list_artists(ArtistSort::Name, SortOrder::Desc).await.unwrap();
    assert_eq!(artists_desc[0].name, "Zebra");
}

#[tokio::test]
async fn test_get_artist_albums() {
    let repo = setup_repo().await;
    let (artist_id, _, _) = seed_data(&repo).await;
    let albums = repo.get_artist_albums(artist_id).await.unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].title, "Test Album");
}

// ─── Album Tests ───

#[tokio::test]
async fn test_upsert_album_creates_new() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("Artist").await.unwrap();
    let album = ScannedAlbum {
        title: "Album".into(),
        title_sortable: "album".into(),
        artist_id,
        image: None,
        thumb: None,
        release_date: None,
        date_precision: None,
        label: None,
        catalog_number: None,
        isrc: None,
        mbid: "none".into(),
        vinyl_numbering: false,
    };
    let id = repo.upsert_album(&album).await.unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_upsert_album_conflict_updates() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("Artist").await.unwrap();

    let album = ScannedAlbum {
        title: "Album".into(),
        title_sortable: "album".into(),
        artist_id,
        image: None,
        thumb: None,
        release_date: None,
        date_precision: None,
        label: Some("Label 1".into()),
        catalog_number: None,
        isrc: None,
        mbid: "none".into(),
        vinyl_numbering: false,
    };
    let id1 = repo.upsert_album(&album).await.unwrap();

    let album2 = ScannedAlbum {
        label: Some("Label 2".into()),
        ..album
    };
    let id2 = repo.upsert_album(&album2).await.unwrap();

    assert_eq!(id1, id2);
    let fetched = repo.get_album(id1).await.unwrap();
    assert_eq!(fetched.label.as_deref(), Some("Label 2"));
}

#[tokio::test]
async fn test_get_album() {
    let repo = setup_repo().await;
    let (_, album_id, _) = seed_data(&repo).await;
    let album = repo.get_album(album_id).await.unwrap();
    assert_eq!(album.title, "Test Album");
    assert_eq!(album.release_date.as_deref(), Some("2023-01-15"));
    assert_eq!(album.date_precision, Some(1));
    assert_eq!(album.label.as_deref(), Some("Test Label"));
}

#[tokio::test]
async fn test_list_albums_sorted() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("Artist").await.unwrap();

    for title in ["Zebra Album", "Alpha Album", "Middle Album"] {
        let album = ScannedAlbum {
            title: title.into(),
            title_sortable: title.to_lowercase(),
            artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        repo.upsert_album(&album).await.unwrap();
    }

    let albums = repo.list_albums(AlbumSort::Title, SortOrder::Asc).await.unwrap();
    assert_eq!(albums.len(), 3);
    assert_eq!(albums[0].title, "Alpha Album");
    assert_eq!(albums[2].title, "Zebra Album");
}

#[tokio::test]
async fn test_get_album_tracks() {
    let repo = setup_repo().await;
    let (_, album_id, _) = seed_data(&repo).await;
    let tracks = repo.get_album_tracks(album_id).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Test Track");
}

#[tokio::test]
async fn test_album_art_none_when_no_image() {
    let repo = setup_repo().await;
    let (_, album_id, _) = seed_data(&repo).await;
    let art = repo.get_album_art(album_id).await.unwrap();
    assert!(art.is_none());
}

#[tokio::test]
async fn test_album_art_with_image() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("Artist").await.unwrap();
    let album = ScannedAlbum {
        title: "Art Album".into(),
        title_sortable: "art album".into(),
        artist_id,
        image: Some(vec![0xFF, 0xD8, 0xFF]), // fake JPEG header
        thumb: Some(vec![0x42, 0x4D]),        // fake BMP header
        release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();

    let art = repo.get_album_art(album_id).await.unwrap();
    assert!(art.is_some());
    assert_eq!(art.unwrap().data, vec![0xFF, 0xD8, 0xFF]);

    let thumb = repo.get_album_thumb(album_id).await.unwrap();
    assert!(thumb.is_some());
    assert_eq!(thumb.unwrap().data, vec![0x42, 0x4D]);
}

// ─── Track Tests ───

#[tokio::test]
async fn test_upsert_track() {
    let repo = setup_repo().await;
    let (_, album_id, _) = seed_data(&repo).await;
    let track = ScannedTrack {
        title: "Track 2".into(),
        title_sortable: "track 2".into(),
        album_id: Some(album_id),
        track_number: Some(2),
        disc_number: Some(1),
        duration: 180000,
        location: "/music/test/track2.flac".into(),
        genres: None,
        artist_names: Some("Test Artist".into()),
        folder: Some("/music/test".into()),
    };
    let id = repo.upsert_track(&track).await.unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_upsert_track_conflict_updates() {
    let repo = setup_repo().await;
    let (_, album_id, track_id) = seed_data(&repo).await;

    let track = ScannedTrack {
        title: "Updated Title".into(),
        title_sortable: "updated title".into(),
        album_id: Some(album_id),
        track_number: Some(1),
        disc_number: Some(1),
        duration: 300000,
        location: "/music/test/track1.flac".into(),
        genres: None,
        artist_names: Some("Test Artist".into()),
        folder: Some("/music/test".into()),
    };
    let id2 = repo.upsert_track(&track).await.unwrap();
    assert_eq!(track_id, id2);

    let fetched = repo.get_track(track_id).await.unwrap();
    assert_eq!(fetched.title, "Updated Title");
    assert_eq!(fetched.duration, 300000);
}

#[tokio::test]
async fn test_get_track() {
    let repo = setup_repo().await;
    let (_, _, track_id) = seed_data(&repo).await;
    let track = repo.get_track(track_id).await.unwrap();
    assert_eq!(track.title, "Test Track");
    assert_eq!(track.duration, 240000);
    assert_eq!(track.location, "/music/test/track1.flac");
}

#[tokio::test]
async fn test_list_tracks() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let tracks = repo.list_tracks(TrackSort::Title, SortOrder::Asc).await.unwrap();
    assert_eq!(tracks.len(), 1);
}

#[tokio::test]
async fn test_get_track_by_path() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let track = repo.get_track_by_path("/music/test/track1.flac").await.unwrap();
    assert!(track.is_some());
    assert_eq!(track.unwrap().title, "Test Track");

    let missing = repo.get_track_by_path("/nonexistent.flac").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_delete_track() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    repo.delete_track("/music/test/track1.flac").await.unwrap();
    let track = repo.get_track_by_path("/music/test/track1.flac").await.unwrap();
    assert!(track.is_none());
}

// ─── Search Tests ───

#[tokio::test]
async fn test_search_artists() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let results = repo.search("Test Art").await.unwrap();
    assert_eq!(results.artists.len(), 1);
    assert_eq!(results.artists[0].name, "Test Artist");
}

#[tokio::test]
async fn test_search_albums() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let results = repo.search("Test Alb").await.unwrap();
    assert_eq!(results.albums.len(), 1);
    assert_eq!(results.albums[0].title, "Test Album");
}

#[tokio::test]
async fn test_search_tracks() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let results = repo.search("Test Tra").await.unwrap();
    assert_eq!(results.tracks.len(), 1);
    assert_eq!(results.tracks[0].title, "Test Track");
}

#[tokio::test]
async fn test_search_no_results() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let results = repo.search("xyznonexistent").await.unwrap();
    assert!(results.artists.is_empty());
    assert!(results.albums.is_empty());
    assert!(results.tracks.is_empty());
}

#[tokio::test]
async fn test_search_case_insensitive() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let results = repo.search("test artist").await.unwrap();
    assert_eq!(results.artists.len(), 1);
}

// ─── Playlist Tests ───

#[tokio::test]
async fn test_list_playlists_includes_system() {
    let repo = setup_repo().await;
    let playlists = repo.list_playlists().await.unwrap();
    // Migration inserts "Liked Songs" system playlist
    assert!(playlists.iter().any(|p| p.name == "Liked Songs" && p.playlist_type == 1));
}

#[tokio::test]
async fn test_create_and_get_playlist() {
    let repo = setup_repo().await;
    let id = repo.create_playlist("My Playlist").await.unwrap();
    assert!(id > 0);

    let detail = repo.get_playlist(id).await.unwrap();
    assert_eq!(detail.playlist.name, "My Playlist");
    assert_eq!(detail.playlist.playlist_type, 0);
    assert!(detail.tracks.is_empty());
}

#[tokio::test]
async fn test_add_track_to_playlist() {
    let repo = setup_repo().await;
    let (_, _, track_id) = seed_data(&repo).await;
    let playlist_id = repo.create_playlist("Test PL").await.unwrap();

    let item_id = repo.add_to_playlist(playlist_id, track_id).await.unwrap();
    assert!(item_id > 0);

    let detail = repo.get_playlist(playlist_id).await.unwrap();
    assert_eq!(detail.tracks.len(), 1);
    assert_eq!(detail.tracks[0].track_id, track_id);
    assert_eq!(detail.tracks[0].position, 1);
}

#[tokio::test]
async fn test_add_multiple_tracks_increments_position() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();

    let mut track_ids = Vec::new();
    for i in 1..=3 {
        let track = ScannedTrack {
            title: format!("Track {i}"),
            title_sortable: format!("track {i}"),
            album_id: Some(album_id),
            track_number: Some(i),
            disc_number: Some(1),
            duration: 100000,
            location: format!("/music/track{i}.flac"),
            genres: None, artist_names: None, folder: None,
        };
        track_ids.push(repo.upsert_track(&track).await.unwrap());
    }

    let pl_id = repo.create_playlist("Ordered PL").await.unwrap();
    for &tid in &track_ids {
        repo.add_to_playlist(pl_id, tid).await.unwrap();
    }

    let detail = repo.get_playlist(pl_id).await.unwrap();
    assert_eq!(detail.tracks.len(), 3);
    assert_eq!(detail.tracks[0].position, 1);
    assert_eq!(detail.tracks[1].position, 2);
    assert_eq!(detail.tracks[2].position, 3);
}

#[tokio::test]
async fn test_remove_track_from_playlist() {
    let repo = setup_repo().await;
    let (_, _, track_id) = seed_data(&repo).await;
    let pl_id = repo.create_playlist("Remove PL").await.unwrap();
    let item_id = repo.add_to_playlist(pl_id, track_id).await.unwrap();

    repo.remove_from_playlist(item_id).await.unwrap();

    let detail = repo.get_playlist(pl_id).await.unwrap();
    assert!(detail.tracks.is_empty());
}

#[tokio::test]
async fn test_move_playlist_item() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();

    let mut track_ids = Vec::new();
    for i in 1..=3 {
        let track = ScannedTrack {
            title: format!("T{i}"), title_sortable: format!("t{i}"),
            album_id: Some(album_id), track_number: Some(i), disc_number: Some(1),
            duration: 100000, location: format!("/music/move{i}.flac"),
            genres: None, artist_names: None, folder: None,
        };
        track_ids.push(repo.upsert_track(&track).await.unwrap());
    }

    let pl_id = repo.create_playlist("Move PL").await.unwrap();
    let mut item_ids = Vec::new();
    for &tid in &track_ids {
        item_ids.push(repo.add_to_playlist(pl_id, tid).await.unwrap());
    }

    // Move item at position 3 to position 1
    repo.move_playlist_item(item_ids[2], 1).await.unwrap();

    let detail = repo.get_playlist(pl_id).await.unwrap();
    // Track 3 should now be first
    assert_eq!(detail.tracks[0].track_id, track_ids[2]);
    assert_eq!(detail.tracks[0].position, 1);
}

#[tokio::test]
async fn test_delete_playlist() {
    let repo = setup_repo().await;
    let (_, _, track_id) = seed_data(&repo).await;
    let pl_id = repo.create_playlist("Delete PL").await.unwrap();
    repo.add_to_playlist(pl_id, track_id).await.unwrap();

    repo.delete_playlist(pl_id).await.unwrap();

    let result = repo.get_playlist(pl_id).await;
    assert!(result.is_err());
}

// ─── Stats Tests ───

#[tokio::test]
async fn test_stats_empty_db() {
    let repo = setup_repo().await;
    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.track_count, 0);
    assert_eq!(stats.total_duration, 0);
}

#[tokio::test]
async fn test_stats_with_data() {
    let repo = setup_repo().await;
    seed_data(&repo).await;
    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.track_count, 1);
    assert_eq!(stats.total_duration, 240000);
}

// ─── Album Path Tests ───

#[tokio::test]
async fn test_upsert_album_path() {
    let repo = setup_repo().await;
    let (_, album_id, _) = seed_data(&repo).await;
    // Should not error
    repo.upsert_album_path(album_id, "/music/test", 1).await.unwrap();
    // Idempotent
    repo.upsert_album_path(album_id, "/music/test", 1).await.unwrap();
}

// ─── Cascade Trigger Tests ───

#[tokio::test]
async fn test_delete_track_cascades_album_and_artist() {
    let repo = setup_repo().await;
    let (artist_id, album_id, _) = seed_data(&repo).await;

    // Delete the only track — should cascade-delete album and artist
    repo.delete_track("/music/test/track1.flac").await.unwrap();

    let album_result = repo.get_album(album_id).await;
    assert!(album_result.is_err(), "album should be deleted by trigger");

    let artist_result = repo.get_artist(artist_id).await;
    assert!(artist_result.is_err(), "artist should be deleted by trigger");
}

// ─── Multiple Sort Order Tests ───

#[tokio::test]
async fn test_list_albums_by_release_date() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("Artist").await.unwrap();

    let dates = [("Old Album", "2000-01-01"), ("New Album", "2024-01-01")];
    for (title, date) in &dates {
        let album = ScannedAlbum {
            title: title.to_string(), title_sortable: title.to_lowercase(),
            artist_id, image: None, thumb: None,
            release_date: Some(date.to_string()), date_precision: Some(1),
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        repo.upsert_album(&album).await.unwrap();
    }

    let asc = repo.list_albums(AlbumSort::Release, SortOrder::Asc).await.unwrap();
    assert_eq!(asc[0].title, "Old Album");

    let desc = repo.list_albums(AlbumSort::Release, SortOrder::Desc).await.unwrap();
    assert_eq!(desc[0].title, "New Album");
}

#[tokio::test]
async fn test_list_tracks_by_duration() {
    let repo = setup_repo().await;
    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();

    for (i, dur) in [(1, 60000i64), (2, 300000), (3, 180000)] {
        let track = ScannedTrack {
            title: format!("T{i}"), title_sortable: format!("t{i}"),
            album_id: Some(album_id), track_number: Some(i), disc_number: Some(1),
            duration: dur, location: format!("/music/dur{i}.flac"),
            genres: None, artist_names: None, folder: None,
        };
        repo.upsert_track(&track).await.unwrap();
    }

    let asc = repo.list_tracks(TrackSort::Duration, SortOrder::Asc).await.unwrap();
    assert_eq!(asc[0].duration, 60000);
    assert_eq!(asc[2].duration, 300000);
}
