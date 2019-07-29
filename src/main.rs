#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate rss;
extern crate chrono;
extern crate serde;
extern crate percent_encoding;
extern crate hyper;
extern crate futures;
extern crate hotwatch;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod sources;
mod common;

use serde_json::from_reader;
use std::fs::File;
use std::collections::HashMap;
use hyper::*;
use hyper::service::*;
use rss::*;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use common::*;
use futures::future::*;
use lazy_static::initialize;
use hotwatch::*;

fn antique() -> Option<Instant> {
    None
}

fn default_cache_life() -> Duration {
    Duration::from_secs(600)
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub sources: Vec<sources::Source>,
    #[serde(skip, default = "antique")]
    pub cache_ts: Option<Instant>,
    #[serde(skip)]
    pub cache: Vec<rss::Item>,
    #[serde(default = "default_cache_life")]
    pub cache_life: Duration,
}

lazy_static! {
    static ref PROFILES: RwLock<HashMap<String, Profile>> = {
        let file = File::open("profiles.json").unwrap();
        RwLock::new(from_reader::<_, _>(file).unwrap())
    };
}

fn serve_rss(req: Request<Body>) -> impl Future<Item = Response<Body>, Error = hyper::Error> {
    let mut response = Response::builder();

    let path = req.uri().path()[1..].to_string();

    PROFILES.write().into_future().map_err(|err| (format!("{:?}", err), 500)).and_then(|profiles| {
        profiles.get_mut(&path).ok_or(("profile not found".to_string(), 404))
    }).and_then(|profile| {
        if profile.cache_ts.is_none() || profile.cache_ts.unwrap().elapsed() > profile.cache_life {
            Either::A(
                join_all(profile.sources.iter().map(|s| s.get_items()))
                .map(|results: Vec<Vec<rss::Item>>| {
                    results.into_iter().flat_map(|v| v.into_iter()).collect()
                }).map(|results: Vec<rss::Item>| {
                    profile.cache = results.clone();
                    results
                }).map_err(|err| {
                    (format!("{:?}", err), 500)
                })
            )
        } else {
            Either::B(ok(profile.cache.clone()))
        }
    }).and_then(|items| {
        ChannelBuilder::default().items(items).build().map_err(|err| (err, 500))
    }).and_then(|channel| {
        response.status(200).body(Body::from(channel.to_string())).map_err(|err| (format!("{:?}", err), 500))
    }).or_else(|(err, code)| {
        response.status(code).body(Body::from(format!("{:#?}", err)))
    }).map_err(|err| err.into())
}

fn main() {
    pretty_env_logger::init();

    trace!("Initializing profiles...");
    initialize(&PROFILES);

    trace!("Watching file...");
    let mut hotwatch = Hotwatch::new().expect("hotwatch failed to initialize");
    hotwatch.watch("./", |event: Event| {
        trace!("Hotwatch event: {:?}", event);
        match event {
            Event::Write(path) | Event::Create(path) => {
                if path.file_name().unwrap().to_string_lossy() != "profiles.json" {
                    return;
                }

                let profiles = PROFILES.write();
                if profiles.is_err() {
                    error!("Error while locking PROFILES for writing. Aborting rehash.");
                    return;
                }

                let file = File::open(path);
                if file.is_err() {
                    error!("Error while opening profiles.json for reading. Aborting rehash.");
                    return;
                }
                let file = file.unwrap();

                let new_profiles = from_reader::<_, _>(file);
                if new_profiles.is_err() {
                    error!("Error while reading profiles.json. Aborting rehash.");
                    return;
                }
                let new_profiles = new_profiles.unwrap();

                let mut profiles = profiles.unwrap();
                *profiles = new_profiles;
                info!("profiles successfully reloaded");
            },
            _ => {}
        }
    }).expect("failed to watch profiles");

    let addr = ([127, 0, 0, 1], 3020).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(serve_rss));

    hyper::rt::run(server);

    info!("Everything has started successfully.");
}
