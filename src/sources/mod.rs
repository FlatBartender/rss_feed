pub mod gelbooru;

pub use gelbooru::*;

use crate::common::*;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Source {
    Gelbooru(gelbooru::GelbooruFeedGenerator),
}

impl FeedGenerator for Source {
    fn get_items(&self, number: u32) -> RssResult<Vec<rss::Item>> {
        match self {
            Source::Gelbooru(s) => s.get_items(number),
        }
    }
}
