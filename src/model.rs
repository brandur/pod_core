use errors::*;
use schema;
use schema::directory_podcast;

use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;

// Database models for the application.
//
// Note that models are separately into `Queryable` and `Insertable` versions
// (with the latter located in the `insertable` module) so that we can insert
// rows with default values like we'd want to for an autoincrementing primary
// key. See here for details:
//
// https://github.com/diesel-rs/diesel/issues/1440
//

#[derive(Queryable)]
pub struct Account {
    pub id:           i64,
    pub created_at:   DateTime<Utc>,
    pub email:        Option<String>,
    pub ephemeral:    bool,
    pub last_ip:      String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct AccountPodcast {
    pub id:              i64,
    pub account_id:      i64,
    pub podcast_id:      i64,
    pub subscribed_at:   DateTime<Utc>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
}

#[derive(Queryable)]
pub struct AccountPodcastEpisode {
    pub id:                 i64,
    pub account_podcast_id: i64,
    pub episode_id:         i64,
    pub listened_seconds:   Option<i64>,
    pub played:             bool,
}

#[derive(Queryable)]
pub struct Directory {
    pub id:   i64,
    pub name: String,
}

impl Directory {
    pub fn itunes(conn: &PgConnection) -> Result<Self> {
        Self::load_dir(conn, "Apple iTunes")
    }

    fn load_dir(conn: &PgConnection, name: &str) -> Result<Self> {
        schema::directory::table
            .filter(schema::directory::name.eq(name))
            .first::<Directory>(conn)
            .chain_err(|| format!("Error loading {} directory record", name))
    }
}

#[changeset_options(treat_none_as_null = "true")]
#[derive(AsChangeset, Identifiable, Queryable)]
#[table_name = "directory_podcast"]
pub struct DirectoryPodcast {
    pub id:           i64,
    pub directory_id: i64,
    pub feed_url:     String,
    pub podcast_id:   Option<i64>,
    pub title:        String,
    pub vendor_id:    String,
}

#[derive(Queryable)]
pub struct DirectoryPodcastException {
    pub id:                   i64,
    pub directory_podcast_id: i64,
    pub errors:               Vec<String>,
    pub occurred_at:          DateTime<Utc>,
}

#[derive(Queryable)]
pub struct DirectoryPodcastDirectorySearch {
    pub id:                   i64,
    pub directory_podcast_id: i64,
    pub directory_search_id:  i64,
    pub position:             i32,
}

#[derive(Queryable)]
pub struct DirectorySearch {
    pub id:           i64,
    pub directory_id: i64,
    pub query:        String,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct Episode {
    pub id:           i64,
    pub description:  Option<String>,
    pub explicit:     Option<bool>,
    pub guid:         String,
    pub link_url:     Option<String>,
    pub media_type:   Option<String>,
    pub media_url:    String,
    pub podcast_id:   i64,
    pub published_at: DateTime<Utc>,
    pub title:        String,
}

#[derive(Queryable)]
pub struct Key {
    pub id:         i64,
    pub account_id: i64,
    pub created_at: DateTime<Utc>,
    pub expire_at:  Option<DateTime<Utc>>,
    pub secret:     String,
}

#[derive(Queryable)]
pub struct Podcast {
    pub id:                i64,
    pub image_url:         Option<String>,
    pub language:          Option<String>,
    pub last_retrieved_at: DateTime<Utc>,
    pub link_url:          Option<String>,
    pub title:             String,
}

#[allow(dead_code)]
#[derive(Queryable)]
pub struct PodcastException {
    pub id:          i64,
    pub podcast_id:  i64,
    pub errors:      Vec<String>,
    pub occurred_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Queryable)]
pub struct PodcastFeedContent {
    pub id:           i64,
    pub podcast_id:   i64,
    pub retrieved_at: DateTime<Utc>,
    pub sha256_hash:  String,
    pub content_gzip: Option<Vec<u8>>,
}

#[allow(dead_code)]
#[derive(Queryable)]
pub struct PodcastFeedLocation {
    pub id:                 i64,
    pub first_retrieved_at: DateTime<Utc>,
    pub feed_url:           String,
    pub last_retrieved_at:  DateTime<Utc>,
    pub podcast_id:         i64,
}

pub mod insertable {
    use schema::{account, account_podcast, account_podcast_episode, directory_podcast,
                 directory_podcast_directory_search, directory_podcast_exception,
                 directory_search, episode, key, podcast, podcast_exception, podcast_feed_content,
                 podcast_feed_location};

    use chrono::{DateTime, Utc};

    #[derive(Insertable)]
    #[table_name = "account"]
    pub struct Account {
        pub email:     Option<String>,
        pub ephemeral: bool,
        pub last_ip:   String,
    }

    #[derive(Insertable)]
    #[table_name = "account_podcast"]
    pub struct AccountPodcast {
        pub account_id:      i64,
        pub podcast_id:      i64,
        pub subscribed_at:   DateTime<Utc>,
        pub unsubscribed_at: Option<DateTime<Utc>>,
    }

    #[derive(Insertable)]
    #[table_name = "account_podcast_episode"]
    pub struct AccountPodcastEpisode {
        pub account_podcast_id: i64,
        pub episode_id:         i64,
        pub listened_seconds:   Option<i64>,
        pub played:             bool,
    }

    #[derive(Insertable)]
    #[table_name = "directory_podcast"]
    pub struct DirectoryPodcast {
        pub directory_id: i64,
        pub feed_url:     String,
        pub podcast_id:   Option<i64>,
        pub title:        String,
        pub vendor_id:    String,
    }

    #[derive(Insertable)]
    #[table_name = "directory_podcast_exception"]
    pub struct DirectoryPodcastException {
        pub directory_podcast_id: i64,
        pub errors:               Vec<String>,
        pub occurred_at:          DateTime<Utc>,
    }

    #[derive(Insertable)]
    #[table_name = "directory_podcast_directory_search"]
    pub struct DirectoryPodcastDirectorySearch {
        pub directory_podcast_id: i64,
        pub directory_search_id:  i64,
        pub position:             i32,
    }

    #[derive(Insertable)]
    #[table_name = "directory_search"]
    pub struct DirectorySearch {
        pub directory_id: i64,
        pub query:        String,
        pub retrieved_at: DateTime<Utc>,
    }

    #[derive(Insertable)]
    #[table_name = "episode"]
    pub struct Episode {
        pub description:  Option<String>,
        pub explicit:     Option<bool>,
        pub guid:         String,
        pub link_url:     Option<String>,
        pub media_type:   Option<String>,
        pub media_url:    String,
        pub podcast_id:   i64,
        pub published_at: DateTime<Utc>,
        pub title:        String,
    }

    #[derive(Insertable)]
    #[table_name = "key"]
    pub struct Key {
        pub account_id: i64,
        pub expire_at:  Option<DateTime<Utc>>,
        pub secret:     String,
    }

    #[changeset_options(treat_none_as_null = "true")]
    #[derive(AsChangeset, Insertable)]
    #[table_name = "podcast"]
    pub struct Podcast {
        pub image_url:         Option<String>,
        pub language:          Option<String>,
        pub last_retrieved_at: DateTime<Utc>,
        pub link_url:          Option<String>,
        pub title:             String,
    }

    #[derive(Insertable)]
    #[table_name = "podcast_exception"]
    pub struct PodcastException {
        pub podcast_id:  i64,
        pub errors:      Vec<String>,
        pub occurred_at: DateTime<Utc>,
    }

    #[derive(Insertable)]
    #[table_name = "podcast_feed_content"]
    pub struct PodcastFeedContent {
        pub content_gzip: Vec<u8>,
        pub podcast_id:   i64,
        pub retrieved_at: DateTime<Utc>,
        pub sha256_hash:  String,
    }

    #[derive(Insertable)]
    #[table_name = "podcast_feed_location"]
    pub struct PodcastFeedLocation {
        pub first_retrieved_at: DateTime<Utc>,
        pub feed_url:           String,
        pub last_retrieved_at:  DateTime<Utc>,
        pub podcast_id:         i64,
    }
}
