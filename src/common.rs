pub type RssResult<T> = Box<futures::future::Future<Item = T, Error = RssError>>;

pub trait FeedGenerator {
    fn get_items(&self, number: u32) -> RssResult<Vec<rss::Item>>;
}

#[derive(Debug)]
pub enum RssError {
    ReqwestError(reqwest::Error),
    StringError(String),
}


