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
extern crate parking_lot;

mod sources;
mod common;

use serde_json::from_reader;
use std::fs::File;
use std::collections::HashMap;
use hyper::*;
use hyper::service::*;
use rss::*;
use std::time::{Duration, Instant};
use common::*;
use futures::future::*;
use lazy_static::initialize;
use hotwatch::*;
use std::sync::RwLock;

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

fn vecvec_into_vec((path, vecvec): (String, Vec<Vec<rss::Item>>)) -> (String, Vec<rss::Item>) {
    (path, vecvec.into_iter().flat_map(|v| v.into_iter()).collect())
}

fn actually_update_cache((path, vec): (String, Vec<rss::Item>)) -> String {
    let profile = PROFILES.write().unwrap().get_mut(&path).unwrap();
    profile.cache = vec;
    path
}

fn cache_need_refresh(path: String) -> bool {
    let (cache_ts, cache_life) = {
        let profile = PROFILES.read().unwrap().get(&path).unwrap();
        (profile.cache_ts, profile.cache_life)
    };

    cache_ts.is_none() || cache_ts.unwrap().elapsed() > cache_life
}

fn refresh_cache(path: String) -> impl Future<Item = String, Error = (String, u16)> {
    let (cache_ts, cache_life) = {
        let profile = PROFILES.read().unwrap().get(&path).unwrap();
        (profile.cache_ts, profile.cache_life)
    };

    match cache_ts.is_none() || cache_ts.unwrap().elapsed() > cache_life {
        false => Either::A(ok(path)),
        true => {
            let sources = {
                PROFILES.read().unwrap().get(&path).unwrap().sources
            };

            Either::B(
                ok(path.clone()).join(join_all(sources.iter().map(sources::Source::get_items)))
                .map(vecvec_into_vec)
                .map(actually_update_cache)
                .map_err(|err| {
                    (format!("{:?}", err), 500)
                })
            )
        },
    }
}

fn get_cache(path: String) -> Vec<rss::Item> {
    PROFILES.read().unwrap().get(&path).unwrap().cache.clone()
}

fn make_channel(items: Vec<rss::Item>) -> impl Future<Item = rss::Channel, Error = (String, u16)> {
    ChannelBuilder::default().items(items).build().map_err(|err| (err, 500)).into_future()
}

fn make_response(channel: rss::Channel) -> impl Future<Item = Response<Body>, Error = (String, u16)> {
    Response::builder().status(200).body(Body::from(channel.to_string())).map_err(|err| (format!("{:?}", err), 500)).into_future()
}

fn response_error((err, code): (String, u16)) -> impl Future<Item = Response<Body>, Error = hyper::http::Error> {
    Response::builder().status(code).body(Body::from(format!("{:#?}", err))).into_future()
}

fn serve_rss(req: Request<Body>) -> impl Future<Item = Response<Body>, Error = hyper::http::Error> {
    let path = req.uri().path()[1..].to_string();

    let profile_exists = {
        PROFILES.read().unwrap().contains_key(&path)
    };
    
    let fut = match profile_exists {
        true => ok(path),
        false => err(("profile not found".to_string(), 404)),
    };

    if cache_need_refresh(path) {
        let sources = {
            PROFILES.read().unwrap().get(&path).unwrap().sources
        };
        
         Either::A(fut.join(join_all(sources.iter().map(sources::Source::get_items))
            .map_err(|err| (format!("{:?}", err), 500)))
            .map(vecvec_into_vec)
            .map(actually_update_cache))
    } else {
        Either::B(fut)
    }.and_then(refresh_cache)
    .map(get_cache)
    .and_then(make_channel)
    .and_then(make_response)
    .or_else(response_error)
    .from_err()
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

                let profiles = PROFILES.write().unwrap();
                *profiles = new_profiles;
                info!("profiles successfully reloaded");
            },
            _ => {}
        }
    }).expect("failed to watch profiles");

    let addr = ([127, 0, 0, 1], 3020).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(serve_rss))
        .map_err(|err| error!("{}", err));

    hyper::rt::run(server);

    info!("Everything has started successfully.");
}
