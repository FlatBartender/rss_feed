pub use futures::*;

pub trait FeedGenerator: Send {
    fn get_items(&self) -> Box<Future<Item = Vec<rss::Item>, Error = RssError>>;
}

#[derive(Debug)]
pub enum RssError {
    ReqwestError(reqwest::Error),
    StringError(String),
}

