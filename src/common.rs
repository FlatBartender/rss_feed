pub use futures::{
    stream::Stream,
    future::Future,
};

pub type RssResult<T> = Result<T, RssError>;

pub trait FeedGenerator {
    fn get_items(&self, number: u32) -> Box<Future<Item = Vec<rss::Item>, Error = RssError>>;
}

#[derive(Debug)]
pub enum RssError {
    ReqwestError(reqwest::Error),
    StringError(String),
}

