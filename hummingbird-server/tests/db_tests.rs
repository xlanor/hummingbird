use std::sync::Arc;

use hummingbird_server::domain::library::*;
use hummingbird_server::domain::scanner::{ScannedAlbum, ScannedTrack};
use hummingbird_server::infrastructure::persistence::Database;

async fn seed_data(db: &dyn Database) -> (i64, i64, i64) {
    let artist_id = db.upsert_artist("Test Artist").await.unwrap();
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
    let album_id = db.upsert_album(&album).await.unwrap();
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
    let track_id = db.upsert_track(&track).await.unwrap();
    (artist_id, album_id, track_id)
}

async fn create_test_user(db: &dyn Database) -> i64 {
    db.create_user("testuser", Some("Test User"), Some("$2b$12$fakehashfakehash"), "user")
        .await
        .unwrap()
}

mod repository_tests {
    use super::*;

    pub async fn test_upsert_artist_creates_new(db: &dyn Database) {
        let id = db.upsert_artist("New Artist").await.unwrap();
        assert!(id > 0);
    }

    pub async fn test_upsert_artist_idempotent(db: &dyn Database) {
        let id1 = db.upsert_artist("Same Artist").await.unwrap();
        let id2 = db.upsert_artist("Same Artist").await.unwrap();
        assert_eq!(id1, id2);
    }

    pub async fn test_get_artist(db: &dyn Database) {
        let (artist_id, _, _) = seed_data(db).await;
        let artist = db.get_artist(artist_id).await.unwrap();
        assert_eq!(artist.name, "Test Artist");
        assert_eq!(artist.name_sortable, "test artist");
    }

    pub async fn test_get_artist_not_found(db: &dyn Database) {
        let result = db.get_artist(99999).await;
        assert!(result.is_err());
    }

    pub async fn test_list_artists_sorted_by_name(db: &dyn Database) {
        db.upsert_artist("Zebra").await.unwrap();
        db.upsert_artist("Alpha").await.unwrap();
        db.upsert_artist("Middle").await.unwrap();

        let artists = db.list_artists(ArtistSort::Name, SortOrder::Asc).await.unwrap();
        assert_eq!(artists.len(), 3);
        assert_eq!(artists[0].name, "Alpha");
        assert_eq!(artists[2].name, "Zebra");

        let artists_desc = db.list_artists(ArtistSort::Name, SortOrder::Desc).await.unwrap();
        assert_eq!(artists_desc[0].name, "Zebra");
    }

    pub async fn test_get_artist_albums(db: &dyn Database) {
        let (artist_id, _, _) = seed_data(db).await;
        let albums = db.get_artist_albums(artist_id).await.unwrap();
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].title, "Test Album");
    }

    pub async fn test_upsert_album_creates_new(db: &dyn Database) {
        let artist_id = db.upsert_artist("Artist").await.unwrap();
        let album = ScannedAlbum {
            title: "Album".into(), title_sortable: "album".into(), artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let id = db.upsert_album(&album).await.unwrap();
        assert!(id > 0);
    }

    pub async fn test_upsert_album_conflict_updates(db: &dyn Database) {
        let artist_id = db.upsert_artist("Artist").await.unwrap();
        let album = ScannedAlbum {
            title: "Album".into(), title_sortable: "album".into(), artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: Some("Label 1".into()), catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let id1 = db.upsert_album(&album).await.unwrap();

        let album2 = ScannedAlbum {
            label: Some("Label 2".into()),
            ..album
        };
        let id2 = db.upsert_album(&album2).await.unwrap();

        assert_eq!(id1, id2);
        let fetched = db.get_album(id1).await.unwrap();
        assert_eq!(fetched.label.as_deref(), Some("Label 2"));
    }

    pub async fn test_get_album(db: &dyn Database) {
        let (_, album_id, _) = seed_data(db).await;
        let album = db.get_album(album_id).await.unwrap();
        assert_eq!(album.title, "Test Album");
        assert_eq!(album.date_precision, Some(1));
        assert_eq!(album.label.as_deref(), Some("Test Label"));
    }

    pub async fn test_list_albums_sorted(db: &dyn Database) {
        let artist_id = db.upsert_artist("Artist").await.unwrap();
        for title in ["Zebra Album", "Alpha Album", "Middle Album"] {
            let album = ScannedAlbum {
                title: title.into(), title_sortable: title.to_lowercase(), artist_id,
                image: None, thumb: None, release_date: None, date_precision: None,
                label: None, catalog_number: None, isrc: None,
                mbid: "none".into(), vinyl_numbering: false,
            };
            db.upsert_album(&album).await.unwrap();
        }

        let albums = db.list_albums(AlbumSort::Title, SortOrder::Asc).await.unwrap();
        assert_eq!(albums.len(), 3);
        assert_eq!(albums[0].title, "Alpha Album");
        assert_eq!(albums[2].title, "Zebra Album");
    }

    pub async fn test_get_album_tracks(db: &dyn Database) {
        let (_, album_id, _) = seed_data(db).await;
        let tracks = db.get_album_tracks(album_id).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Test Track");
    }

    pub async fn test_album_art_none_when_no_image(db: &dyn Database) {
        let (_, album_id, _) = seed_data(db).await;
        let art = db.get_album_art(album_id).await.unwrap();
        assert!(art.is_none());
    }

    pub async fn test_album_art_with_image(db: &dyn Database) {
        let artist_id = db.upsert_artist("Artist").await.unwrap();
        let album = ScannedAlbum {
            title: "Art Album".into(), title_sortable: "art album".into(), artist_id,
            image: Some(vec![0xFF, 0xD8, 0xFF]), thumb: Some(vec![0x42, 0x4D]),
            release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let album_id = db.upsert_album(&album).await.unwrap();

        let art = db.get_album_art(album_id).await.unwrap();
        assert!(art.is_some());
        assert_eq!(art.unwrap().data, vec![0xFF, 0xD8, 0xFF]);

        let thumb = db.get_album_thumb(album_id).await.unwrap();
        assert!(thumb.is_some());
        assert_eq!(thumb.unwrap().data, vec![0x42, 0x4D]);
    }

    pub async fn test_upsert_track(db: &dyn Database) {
        let (_, album_id, _) = seed_data(db).await;
        let track = ScannedTrack {
            title: "Track 2".into(), title_sortable: "track 2".into(),
            album_id: Some(album_id), track_number: Some(2), disc_number: Some(1),
            duration: 180000, location: "/music/test/track2.flac".into(),
            genres: None, artist_names: Some("Test Artist".into()),
            folder: Some("/music/test".into()),
        };
        let id = db.upsert_track(&track).await.unwrap();
        assert!(id > 0);
    }

    pub async fn test_upsert_track_conflict_updates(db: &dyn Database) {
        let (_, album_id, track_id) = seed_data(db).await;
        let track = ScannedTrack {
            title: "Updated Title".into(), title_sortable: "updated title".into(),
            album_id: Some(album_id), track_number: Some(1), disc_number: Some(1),
            duration: 300000, location: "/music/test/track1.flac".into(),
            genres: None, artist_names: Some("Test Artist".into()),
            folder: Some("/music/test".into()),
        };
        let id2 = db.upsert_track(&track).await.unwrap();
        assert_eq!(track_id, id2);

        let fetched = db.get_track(track_id).await.unwrap();
        assert_eq!(fetched.title, "Updated Title");
        assert_eq!(fetched.duration, 300000);
    }

    pub async fn test_get_track(db: &dyn Database) {
        let (_, _, track_id) = seed_data(db).await;
        let track = db.get_track(track_id).await.unwrap();
        assert_eq!(track.title, "Test Track");
        assert_eq!(track.duration, 240000);
        assert_eq!(track.location, "/music/test/track1.flac");
    }

    pub async fn test_list_tracks(db: &dyn Database) {
        seed_data(db).await;
        let tracks = db.list_tracks(TrackSort::Title, SortOrder::Asc).await.unwrap();
        assert_eq!(tracks.len(), 1);
    }

    pub async fn test_get_track_by_path(db: &dyn Database) {
        seed_data(db).await;
        let track = db.get_track_by_path("/music/test/track1.flac").await.unwrap();
        assert!(track.is_some());
        assert_eq!(track.unwrap().title, "Test Track");

        let missing = db.get_track_by_path("/nonexistent.flac").await.unwrap();
        assert!(missing.is_none());
    }

    pub async fn test_delete_track(db: &dyn Database) {
        seed_data(db).await;
        db.delete_track("/music/test/track1.flac").await.unwrap();
        let track = db.get_track_by_path("/music/test/track1.flac").await.unwrap();
        assert!(track.is_none());
    }

    pub async fn test_search_finds_results(db: &dyn Database) {
        seed_data(db).await;
        let results = db.search("Test Art").await.unwrap();
        assert_eq!(results.artists.len(), 1);
        assert_eq!(results.artists[0].name, "Test Artist");

        let results = db.search("Test Alb").await.unwrap();
        assert_eq!(results.albums.len(), 1);

        let results = db.search("Test Tra").await.unwrap();
        assert_eq!(results.tracks.len(), 1);
    }

    pub async fn test_search_no_results(db: &dyn Database) {
        seed_data(db).await;
        let results = db.search("xyznonexistent").await.unwrap();
        assert!(results.artists.is_empty());
        assert!(results.albums.is_empty());
        assert!(results.tracks.is_empty());
    }

    pub async fn test_search_case_insensitive(db: &dyn Database) {
        seed_data(db).await;
        let results = db.search("test artist").await.unwrap();
        assert_eq!(results.artists.len(), 1);
    }

    pub async fn test_list_playlists_includes_system(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let playlists = db.list_playlists(user_id).await.unwrap();
        assert!(playlists.iter().any(|p| p.name == "Liked Songs" && p.playlist_type == 1));
    }

    pub async fn test_create_and_get_playlist(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let id = db.create_playlist("My Playlist", user_id).await.unwrap();
        assert!(id > 0);
        let detail = db.get_playlist(id).await.unwrap();
        assert_eq!(detail.playlist.name, "My Playlist");
        assert_eq!(detail.playlist.playlist_type, 0);
        assert!(detail.tracks.is_empty());
    }

    pub async fn test_add_track_to_playlist(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let (_, _, track_id) = seed_data(db).await;
        let playlist_id = db.create_playlist("Test PL", user_id).await.unwrap();
        let item_id = db.add_to_playlist(playlist_id, track_id).await.unwrap();
        assert!(item_id > 0);

        let detail = db.get_playlist(playlist_id).await.unwrap();
        assert_eq!(detail.tracks.len(), 1);
        assert_eq!(detail.tracks[0].track_id, track_id);
        assert_eq!(detail.tracks[0].position, 1);
    }

    pub async fn test_add_multiple_tracks_increments_position(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let artist_id = db.upsert_artist("A").await.unwrap();
        let album = ScannedAlbum {
            title: "Al".into(), title_sortable: "al".into(), artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let album_id = db.upsert_album(&album).await.unwrap();

        let mut track_ids = Vec::new();
        for i in 1..=3 {
            let track = ScannedTrack {
                title: format!("Track {i}"), title_sortable: format!("track {i}"),
                album_id: Some(album_id), track_number: Some(i), disc_number: Some(1),
                duration: 100000, location: format!("/music/track{i}.flac"),
                genres: None, artist_names: None, folder: None,
            };
            track_ids.push(db.upsert_track(&track).await.unwrap());
        }

        let pl_id = db.create_playlist("Ordered PL", user_id).await.unwrap();
        for &tid in &track_ids {
            db.add_to_playlist(pl_id, tid).await.unwrap();
        }

        let detail = db.get_playlist(pl_id).await.unwrap();
        assert_eq!(detail.tracks.len(), 3);
        assert_eq!(detail.tracks[0].position, 1);
        assert_eq!(detail.tracks[1].position, 2);
        assert_eq!(detail.tracks[2].position, 3);
    }

    pub async fn test_remove_track_from_playlist(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let (_, _, track_id) = seed_data(db).await;
        let pl_id = db.create_playlist("Remove PL", user_id).await.unwrap();
        let item_id = db.add_to_playlist(pl_id, track_id).await.unwrap();
        db.remove_from_playlist(item_id).await.unwrap();
        let detail = db.get_playlist(pl_id).await.unwrap();
        assert!(detail.tracks.is_empty());
    }

    pub async fn test_move_playlist_item(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let artist_id = db.upsert_artist("A").await.unwrap();
        let album = ScannedAlbum {
            title: "Al".into(), title_sortable: "al".into(), artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let album_id = db.upsert_album(&album).await.unwrap();

        let mut track_ids = Vec::new();
        for i in 1..=3 {
            let track = ScannedTrack {
                title: format!("T{i}"), title_sortable: format!("t{i}"),
                album_id: Some(album_id), track_number: Some(i), disc_number: Some(1),
                duration: 100000, location: format!("/music/move{i}.flac"),
                genres: None, artist_names: None, folder: None,
            };
            track_ids.push(db.upsert_track(&track).await.unwrap());
        }

        let pl_id = db.create_playlist("Move PL", user_id).await.unwrap();
        let mut item_ids = Vec::new();
        for &tid in &track_ids {
            item_ids.push(db.add_to_playlist(pl_id, tid).await.unwrap());
        }

        db.move_playlist_item(item_ids[2], 1).await.unwrap();
        let detail = db.get_playlist(pl_id).await.unwrap();
        assert_eq!(detail.tracks[0].track_id, track_ids[2]);
        assert_eq!(detail.tracks[0].position, 1);
    }

    pub async fn test_delete_playlist(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let (_, _, track_id) = seed_data(db).await;
        let pl_id = db.create_playlist("Delete PL", user_id).await.unwrap();
        db.add_to_playlist(pl_id, track_id).await.unwrap();
        db.delete_playlist(pl_id).await.unwrap();
        let result = db.get_playlist(pl_id).await;
        assert!(result.is_err());
    }

    pub async fn test_playlist_scoped_to_user(db: &dyn Database) {
        let user1 = db.create_user("user1", None, None, "user").await.unwrap();
        let user2 = db.create_user("user2", None, None, "user").await.unwrap();

        db.create_playlist("User1 PL", user1).await.unwrap();
        db.create_playlist("User2 PL", user2).await.unwrap();

        let u1_playlists = db.list_playlists(user1).await.unwrap();
        let u1_names: Vec<&str> = u1_playlists.iter().map(|p| p.name.as_str()).collect();
        assert!(u1_names.contains(&"User1 PL"));
        assert!(!u1_names.contains(&"User2 PL"));

        let u2_playlists = db.list_playlists(user2).await.unwrap();
        let u2_names: Vec<&str> = u2_playlists.iter().map(|p| p.name.as_str()).collect();
        assert!(u2_names.contains(&"User2 PL"));
        assert!(!u2_names.contains(&"User1 PL"));
    }

    pub async fn test_playlist_owner(db: &dyn Database) {
        let user_id = create_test_user(db).await;
        let pl_id = db.create_playlist("Owned PL", user_id).await.unwrap();
        let owner = db.get_playlist_owner(pl_id).await.unwrap();
        assert_eq!(owner, user_id);
    }

    pub async fn test_stats_empty_db(db: &dyn Database) {
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.track_count, 0);
        assert_eq!(stats.total_duration, 0);
    }

    pub async fn test_stats_with_data(db: &dyn Database) {
        seed_data(db).await;
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.track_count, 1);
        assert_eq!(stats.total_duration, 240000);
    }

    pub async fn test_upsert_album_path(db: &dyn Database) {
        let (_, album_id, _) = seed_data(db).await;
        db.upsert_album_path(album_id, "/music/test", 1).await.unwrap();
        db.upsert_album_path(album_id, "/music/test", 1).await.unwrap();
    }

    pub async fn test_delete_track_cascades_album_and_artist(db: &dyn Database) {
        let (artist_id, album_id, _) = seed_data(db).await;
        db.delete_track("/music/test/track1.flac").await.unwrap();

        let album_result = db.get_album(album_id).await;
        assert!(album_result.is_err(), "album should be deleted by cascade");

        let artist_result = db.get_artist(artist_id).await;
        assert!(artist_result.is_err(), "artist should be deleted by cascade");
    }

    pub async fn test_list_albums_by_release_date(db: &dyn Database) {
        let artist_id = db.upsert_artist("Artist").await.unwrap();
        for (title, date) in [("Old Album", "2000-01-01"), ("New Album", "2024-01-01")] {
            let album = ScannedAlbum {
                title: title.into(), title_sortable: title.to_lowercase(), artist_id,
                image: None, thumb: None,
                release_date: Some(date.into()), date_precision: Some(1),
                label: None, catalog_number: None, isrc: None,
                mbid: "none".into(), vinyl_numbering: false,
            };
            db.upsert_album(&album).await.unwrap();
        }

        let asc = db.list_albums(AlbumSort::Release, SortOrder::Asc).await.unwrap();
        assert_eq!(asc[0].title, "Old Album");

        let desc = db.list_albums(AlbumSort::Release, SortOrder::Desc).await.unwrap();
        assert_eq!(desc[0].title, "New Album");
    }

    pub async fn test_list_tracks_by_duration(db: &dyn Database) {
        let artist_id = db.upsert_artist("A").await.unwrap();
        let album = ScannedAlbum {
            title: "Al".into(), title_sortable: "al".into(), artist_id,
            image: None, thumb: None, release_date: None, date_precision: None,
            label: None, catalog_number: None, isrc: None,
            mbid: "none".into(), vinyl_numbering: false,
        };
        let album_id = db.upsert_album(&album).await.unwrap();

        for (i, dur) in [(1, 60000i64), (2, 300000), (3, 180000)] {
            let track = ScannedTrack {
                title: format!("T{i}"), title_sortable: format!("t{i}"),
                album_id: Some(album_id), track_number: Some(i), disc_number: Some(1),
                duration: dur, location: format!("/music/dur{i}.flac"),
                genres: None, artist_names: None, folder: None,
            };
            db.upsert_track(&track).await.unwrap();
        }

        let asc = db.list_tracks(TrackSort::Duration, SortOrder::Asc).await.unwrap();
        assert_eq!(asc[0].duration, 60000);
        assert_eq!(asc[2].duration, 300000);
    }

    pub async fn test_create_user(db: &dyn Database) {
        let id = db.create_user("alice", Some("Alice"), Some("hash"), "user").await.unwrap();
        assert!(id > 0);
        let user = db.get_user_by_id(id).await.unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.display_name.as_deref(), Some("Alice"));
        assert_eq!(user.role, "user");
    }

    pub async fn test_get_user_by_username(db: &dyn Database) {
        db.create_user("bob", None, Some("hash"), "admin").await.unwrap();
        let user = db.get_user_by_username("bob").await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().role, "admin");

        let missing = db.get_user_by_username("nobody").await.unwrap();
        assert!(missing.is_none());
    }

    pub async fn test_create_or_get_oidc_user(db: &dyn Database) {
        let user = db
            .create_or_get_oidc_user("https://issuer.example", "sub123", "oidcuser", Some("OIDC User"))
            .await
            .unwrap();
        assert_eq!(user.oidc_subject.as_deref(), Some("sub123"));

        let same = db
            .create_or_get_oidc_user("https://issuer.example", "sub123", "oidcuser", None)
            .await
            .unwrap();
        assert_eq!(user.id, same.id);
    }

    pub async fn test_list_users(db: &dyn Database) {
        db.create_user("u1", None, None, "user").await.unwrap();
        db.create_user("u2", None, None, "admin").await.unwrap();
        let users = db.list_users().await.unwrap();
        assert!(users.len() >= 2);
    }

    pub async fn test_delete_user(db: &dyn Database) {
        let id = db.create_user("todelete", None, Some("h"), "user").await.unwrap();
        db.delete_user(id).await.unwrap();
        let result = db.get_user_by_id(id).await;
        assert!(result.is_err());
    }

    pub async fn test_delete_user_cascades_playlists(db: &dyn Database) {
        let user_id = db.create_user("cascade", None, None, "user").await.unwrap();
        let pl_id = db.create_playlist("Cascade PL", user_id).await.unwrap();
        db.delete_user(user_id).await.unwrap();
        let result = db.get_playlist(pl_id).await;
        assert!(result.is_err());
    }

    pub async fn test_update_user_password(db: &dyn Database) {
        let id = db.create_user("pwuser", None, Some("old"), "user").await.unwrap();
        db.update_user_password(id, "new_hash").await.unwrap();
        let user = db.get_user_by_id(id).await.unwrap();
        assert_eq!(user.password_hash.as_deref(), Some("new_hash"));
    }
}

macro_rules! backend_tests {
    ($setup:expr $(, #[$attr:meta])* ) => {
        $(#[$attr])* #[tokio::test] async fn test_upsert_artist_creates_new() { let r = $setup.await; repository_tests::test_upsert_artist_creates_new(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_artist_idempotent() { let r = $setup.await; repository_tests::test_upsert_artist_idempotent(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_artist() { let r = $setup.await; repository_tests::test_get_artist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_artist_not_found() { let r = $setup.await; repository_tests::test_get_artist_not_found(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_artists_sorted_by_name() { let r = $setup.await; repository_tests::test_list_artists_sorted_by_name(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_artist_albums() { let r = $setup.await; repository_tests::test_get_artist_albums(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_album_creates_new() { let r = $setup.await; repository_tests::test_upsert_album_creates_new(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_album_conflict_updates() { let r = $setup.await; repository_tests::test_upsert_album_conflict_updates(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_album() { let r = $setup.await; repository_tests::test_get_album(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_albums_sorted() { let r = $setup.await; repository_tests::test_list_albums_sorted(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_album_tracks() { let r = $setup.await; repository_tests::test_get_album_tracks(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_album_art_none_when_no_image() { let r = $setup.await; repository_tests::test_album_art_none_when_no_image(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_album_art_with_image() { let r = $setup.await; repository_tests::test_album_art_with_image(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_track() { let r = $setup.await; repository_tests::test_upsert_track(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_track_conflict_updates() { let r = $setup.await; repository_tests::test_upsert_track_conflict_updates(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_track() { let r = $setup.await; repository_tests::test_get_track(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_tracks() { let r = $setup.await; repository_tests::test_list_tracks(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_track_by_path() { let r = $setup.await; repository_tests::test_get_track_by_path(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_delete_track() { let r = $setup.await; repository_tests::test_delete_track(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_search_finds_results() { let r = $setup.await; repository_tests::test_search_finds_results(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_search_no_results() { let r = $setup.await; repository_tests::test_search_no_results(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_search_case_insensitive() { let r = $setup.await; repository_tests::test_search_case_insensitive(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_playlists_includes_system() { let r = $setup.await; repository_tests::test_list_playlists_includes_system(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_create_and_get_playlist() { let r = $setup.await; repository_tests::test_create_and_get_playlist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_add_track_to_playlist() { let r = $setup.await; repository_tests::test_add_track_to_playlist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_add_multiple_tracks_increments_position() { let r = $setup.await; repository_tests::test_add_multiple_tracks_increments_position(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_remove_track_from_playlist() { let r = $setup.await; repository_tests::test_remove_track_from_playlist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_move_playlist_item() { let r = $setup.await; repository_tests::test_move_playlist_item(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_delete_playlist() { let r = $setup.await; repository_tests::test_delete_playlist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_playlist_scoped_to_user() { let r = $setup.await; repository_tests::test_playlist_scoped_to_user(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_playlist_owner() { let r = $setup.await; repository_tests::test_playlist_owner(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_stats_empty_db() { let r = $setup.await; repository_tests::test_stats_empty_db(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_stats_with_data() { let r = $setup.await; repository_tests::test_stats_with_data(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_upsert_album_path() { let r = $setup.await; repository_tests::test_upsert_album_path(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_delete_track_cascades_album_and_artist() { let r = $setup.await; repository_tests::test_delete_track_cascades_album_and_artist(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_albums_by_release_date() { let r = $setup.await; repository_tests::test_list_albums_by_release_date(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_tracks_by_duration() { let r = $setup.await; repository_tests::test_list_tracks_by_duration(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_create_user() { let r = $setup.await; repository_tests::test_create_user(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_get_user_by_username() { let r = $setup.await; repository_tests::test_get_user_by_username(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_create_or_get_oidc_user() { let r = $setup.await; repository_tests::test_create_or_get_oidc_user(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_list_users() { let r = $setup.await; repository_tests::test_list_users(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_delete_user() { let r = $setup.await; repository_tests::test_delete_user(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_delete_user_cascades_playlists() { let r = $setup.await; repository_tests::test_delete_user_cascades_playlists(r.as_ref()).await; }
        $(#[$attr])* #[tokio::test] async fn test_update_user_password() { let r = $setup.await; repository_tests::test_update_user_password(r.as_ref()).await; }
    };
}

mod sqlite {
    use super::*;
    use hummingbird_server::infrastructure::persistence::sqlite::SqliteDatabase;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> Arc<dyn Database> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("failed to create SQLite pool");

        let db = SqliteDatabase::new(pool);
        db.run_migrations().await.expect("failed to run SQLite migrations");
        Arc::new(db)
    }

    backend_tests!(setup());
}

mod postgres {
    use super::*;
    use hummingbird_server::infrastructure::persistence::postgres::PostgresDatabase;
    use sqlx::postgres::PgPoolOptions;

    async fn setup() -> Arc<dyn Database> {
        let url = std::env::var("TEST_POSTGRES_URL")
            .expect("TEST_POSTGRES_URL must be set to run Postgres tests");

        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("failed to connect to Postgres");

        sqlx::raw_sql(
            "DROP TABLE IF EXISTS playlist_item CASCADE; \
             DROP TABLE IF EXISTS playlist CASCADE; \
             DROP TABLE IF EXISTS album_path CASCADE; \
             DROP TABLE IF EXISTS track CASCADE; \
             DROP TABLE IF EXISTS album CASCADE; \
             DROP TABLE IF EXISTS artist CASCADE; \
             DROP TABLE IF EXISTS app_user CASCADE; \
             DROP FUNCTION IF EXISTS cleanup_after_track_delete CASCADE; \
             DROP FUNCTION IF EXISTS cleanup_after_album_delete CASCADE; \
             DROP FUNCTION IF EXISTS cleanup_after_track_album_update CASCADE;"
        )
        .execute(&pool)
        .await
        .expect("failed to clean Postgres database");

        let db = PostgresDatabase::new(pool);
        db.run_migrations().await.expect("failed to run Postgres migrations");
        Arc::new(db)
    }

    backend_tests!(setup(), #[ignore]);
}

mod mariadb {
    use super::*;
    use hummingbird_server::infrastructure::persistence::mariadb::MariaDbDatabase;
    use sqlx::mysql::MySqlPoolOptions;

    async fn setup() -> Arc<dyn Database> {
        let url = std::env::var("TEST_MARIADB_URL")
            .expect("TEST_MARIADB_URL must be set to run MariaDB tests");

        let pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("failed to connect to MariaDB");

        for table in ["playlist_item", "playlist", "album_path", "track", "album", "artist", "app_user"] {
            sqlx::raw_sql(&format!("DROP TABLE IF EXISTS {table}"))
                .execute(&pool)
                .await
                .expect(&format!("failed to drop table {table}"));
        }

        let db = MariaDbDatabase::new(pool);
        db.run_migrations().await.expect("failed to run MariaDB migrations");
        Arc::new(db)
    }

    backend_tests!(setup(), #[ignore]);
}
