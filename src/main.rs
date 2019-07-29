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
use hyper::rt::Future;

fn default_cache_ts() -> Instant {
    Instant::now() - Duration::from_secs(600)
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub sources: Vec<sources::Source>,
    #[serde(skip, default = "default_cache_ts")]
    pub cache_ts: Instant,
    #[serde(skip)]
    pub cache: Vec<rss::Item>,
}

lazy_static! {
    static ref PROFILES: RwLock<HashMap<String, Profile>> = {
        let file = File::open("profiles.json").unwrap();
        RwLock::new(from_reader::<_, _>(file).unwrap())
    };
}

fn serve_rss(req: Request<Body>) -> Response<Body> {
    let mut response = Response::builder();

    let path = req.uri().path()[1..].to_string();

    let (items, renew) = {
        let profiles = PROFILES.read();
        if let Err(error) = profiles {
            return response.status(500).body(Body::from(format!("{:#?}", error))).unwrap()
        }
        let profiles = profiles.unwrap();

        let profile = profiles.get(&path);

        if profile.is_none() {
            return response.status(404).body(Body::empty()).unwrap()
        }
        let profile = profile.unwrap();

        let mut renew = false;
        let items = if profile.cache_ts.elapsed() > Duration::from_secs(600) {
            renew = true;
            let results: RssResult<Vec<Vec<rss::Item>>> = profile.sources.iter()
                .map(|s| s.get_items(10))
                .collect();
            if let Err(error) = results {
                return response.status(500).body(format!("{:#?}", error).into()).unwrap()
            }
            let results = results.unwrap();
            results.into_iter()
                .flat_map(|v| v.into_iter())
                .collect()
        } else {
            profile.cache.clone()
        };

        (items, renew)
    };

    if renew {
        let profiles = PROFILES.write();
        if let Err(error) = profiles {
            return response.status(500).body(Body::from(format!("{:#?}", error))).unwrap()
        }
        let mut profiles = profiles.unwrap();

        let mut profile = profiles.get_mut(&path).unwrap();

        profile.cache = items.clone();
        profile.cache_ts = Instant::now();
    }

    let channel = ChannelBuilder::default()
        .items(items)
        .build()
        .unwrap();

    response.status(200)
        .body(Body::from(channel.to_string()))
        .unwrap()
}

fn main() {
    let addr = ([127, 0, 0, 1], 3020).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn_ok(serve_rss));

    hyper::rt::run(server.map_err(|e| {
        eprintln!("server error: {}", e);
    }));
}
