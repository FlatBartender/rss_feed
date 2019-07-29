pub mod gelbooru;
pub mod dummy;

pub use gelbooru::*;
pub use dummy::*;

use crate::common::*;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Source {
    Gelbooru(gelbooru::GelbooruFeedGenerator),
    Dummy(dummy::DummyFeedGenerator),
}

impl FeedGenerator for Source {
    fn get_items(&self) -> RssResult<Vec<rss::Item>> {
        match self {
            Source::Gelbooru(s) => s.get_items(),
            Source::Dummy(s) => s.get_items(),
        }
    }
}
