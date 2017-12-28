extern crate chrono;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate iron;
#[macro_use]
extern crate juniper;
extern crate juniper_iron;
extern crate mount;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate serde;
extern crate time;
extern crate tokio_core;

mod errors;
mod model;

// Generated file: skip rustfmt
#[cfg_attr(rustfmt, rustfmt_skip)]
mod schema;

#[cfg(test)]
mod test_helpers;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use hyper::client::HttpConnector;
use iron::prelude::*;
use iron::{typemap, AfterMiddleware, BeforeMiddleware};
use juniper::FieldResult;
use juniper_iron::{GraphQLHandler, GraphiQLHandler};
use mount::Mount;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use schema::{directories_podcasts, episodes, podcasts};
use self::errors::*;
use std::env;
use std::str::FromStr;
use time::precise_time_ns;
use tokio_core::reactor::Core;

type DieselConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

//
// Model
//

struct Context {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl Context {
    fn get_conn(&self) -> Result<DieselConnection> {
        self.pool
            .get()
            .chain_err(|| "Error acquiring connection from database pool")
    }
}

impl juniper::Context for Context {}

//
// GraphQL
//

struct Mutation;

graphql_object!(
    Mutation: Context | &self | {

    description: "The root mutation object of the schema."

        //field createHuman(&executor, new_human: NewHuman) -> FieldResult<Human> {
        //    let db = executor.context().pool.get_connection()?;
        //    let human: Human = db.insert_human(&new_human)?;
        //    Ok(human)
        //}
    }
);

#[derive(GraphQLObject)]
struct EpisodeObject {
    #[graphql(description = "The episode's ID.")]
    pub id: String,

    #[graphql(description = "The episode's description.")]
    pub description: String,

    #[graphql(description = "Whether the episode is considered explicit.")]
    pub explicit: bool,

    #[graphql(description = "The episode's web link.")]
    pub link_url: String,

    #[graphql(description = "The episode's media link (i.e. where the audio can be found).")]
    pub media_url: String,

    #[graphql(description = "The episode's podcast's ID.")]
    pub podcast_id: String,

    #[graphql(description = "The episode's publishing date and time.")]
    pub published_at: DateTime<Utc>,

    #[graphql(description = "The episode's title.")]
    pub title: String,
}

impl<'a> From<&'a model::Episode> for EpisodeObject {
    fn from(e: &model::Episode) -> Self {
        EpisodeObject {
            id:           e.id.to_string(),
            description:  e.description.to_string(),
            explicit:     e.explicit,
            link_url:     e.link_url.to_owned(),
            media_url:    e.media_url.to_owned(),
            podcast_id:   e.podcast_id.to_string(),
            published_at: e.published_at,
            title:        e.title.to_owned(),
        }
    }
}

#[derive(GraphQLObject)]
struct PodcastObject {
    // IDs are exposed as strings because JS cannot store a fully 64-bit integer. This should be
    // okay because clients should be treating them as opaque tokens anyway.
    #[graphql(description = "The podcast's ID.")]
    pub id: String,

    #[graphql(description = "The podcast's image URL.")]
    pub image_url: String,

    #[graphql(description = "The podcast's language.")]
    pub language: String,

    #[graphql(description = "The podcast's RSS link URL.")]
    pub link_url: String,

    #[graphql(description = "The podcast's title.")]
    pub title: String,
}

impl<'a> From<&'a model::Podcast> for PodcastObject {
    fn from(p: &model::Podcast) -> Self {
        PodcastObject {
            id:        p.id.to_string(),
            image_url: p.image_url.to_owned(),
            language:  p.language.to_owned(),
            link_url:  p.link_url.to_owned(),
            title:     p.title.to_owned(),
        }
    }
}

struct Query;

graphql_object!(Query: Context |&self| {
    description: "The root query object of the schema."

    field apiVersion() -> &str {
        "1.0"
    }

    field episodes(&executor, podcast_id: String as "The podcast's ID.") ->
            FieldResult<Vec<EpisodeObject>> as "A collection episodes for a podcast." {
        let id = i64::from_str(podcast_id.as_str()).
            chain_err(|| "Error parsing podcast ID")?;

        let context = executor.context();
        let results = episodes::table
            .filter(episodes::podcast_id.eq(id))
            .order(episodes::published_at.desc())
            .limit(20)
            .load::<model::Episode>(&*context.get_conn()?)
            .chain_err(|| "Error loading episodes from the database")?
            .iter()
            .map(|p| EpisodeObject::from(p) )
            .collect::<Vec<_>>();
        Ok(results)
    }

    field podcasts(&executor) -> FieldResult<Vec<PodcastObject>> as "A collection of podcasts." {
        let context = executor.context();
        let results = podcasts::table
            .order(podcasts::title.asc())
            .limit(5)
            .load::<model::Podcast>(&*context.get_conn()?)
            .chain_err(|| "Error loading podcasts from the database")?
            .iter()
            .map(|p| PodcastObject::from(p) )
            .collect::<Vec<_>>();
        Ok(results)
    }
});

//
// HTTP abstractions
//

struct ResponseTime;

impl typemap::Key for ResponseTime {
    type Value = u64;
}

impl BeforeMiddleware for ResponseTime {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<ResponseTime>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<ResponseTime>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }
}

//
// Mediators
//

struct DirectoryPodcastUpdater<'a> {
    pub client:      &'a hyper::Client<HttpConnector, hyper::Body>,
    pub conn:        &'a PgConnection,
    pub core:        &'a mut Core,
    pub dir_podcast: &'a mut model::DirectoryPodcast,
}

impl<'a> DirectoryPodcastUpdater<'a> {
    pub fn run(&mut self) -> Result<()> {
        self.conn
            .transaction::<_, Error, _>(|| self.run_inner())
            .chain_err(|| "Error with database transaction")
    }

    fn run_inner(&mut self) -> Result<()> {
        let raw_url = self.dir_podcast.feed_url.clone().unwrap();
        let feed_url = hyper::Uri::from_str(raw_url.as_str())
            .chain_err(|| format!("Error parsing feed URL: {}", raw_url))?;
        let res = self.core
            .run(self.client.get(feed_url))
            .chain_err(|| format!("Error fetching feed URL: {}", raw_url))?;
        println!("Response: {}", res.status());

        self.dir_podcast.feed_url = None;
        self.dir_podcast
            .save_changes::<model::DirectoryPodcast>(&self.conn)
            .chain_err(|| "Error saving changes to directory podcast")?;
        Ok(())
    }
}

#[test]
fn test_run() {
    let conn = test_helpers::connection();
    let mut core = Core::new().unwrap();
    let client = hyper::Client::new(&core.handle());

    let itunes = model::Directory::itunes(&conn).unwrap();
    let mut dir_podcast = model::DirectoryPodcast {
        id:           0,
        directory_id: itunes.id,
        feed_url:     Some("http://feeds.feedburner.com/RoderickOnTheLine".to_owned()),
        podcast_id:   None,
        vendor_id:    "471418144".to_owned(),
    };
    diesel::insert_into(directories_podcasts::table)
        .values(&dir_podcast)
        .execute(&conn)
        .unwrap();

    let mut updater = DirectoryPodcastUpdater {
        client:      &client,
        conn:        &conn,
        core:        &mut core,
        dir_podcast: &mut dir_podcast,
    };
    updater.run().unwrap();
}

/*
struct PodcastUpdater {
    pub podcast: &Podcast,
}

impl PodcastUpdater {
    fn run(&self) {}
}
*/

//
// Main
//

fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let graphql_endpoint = GraphQLHandler::new(
        move |_: &mut Request| -> Context { Context { pool: pool.clone() } },
        Query {},
        Mutation {},
    );
    let graphiql_endpoint = GraphiQLHandler::new("/graphql");

    let mut mount = Mount::new();
    mount.mount("/", graphiql_endpoint);
    mount.mount("/graphql", graphql_endpoint);

    let mut chain = Chain::new(mount);
    chain.link_before(ResponseTime);
    chain.link_after(ResponseTime);

    let port = env::var("PORT").unwrap_or("8080".to_owned());
    let host = format!("0.0.0.0:{}", port);
    println!("GraphQL server started on {}", host);
    Iron::new(chain).http(host.as_str()).unwrap();
}
