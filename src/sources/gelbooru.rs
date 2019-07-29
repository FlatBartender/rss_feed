use crate::common::*;
use chrono::prelude::*;
use percent_encoding::*;
use reqwest::r#async::Client;
use futures::future::*;

lazy_static! {
    static ref CLIENT: Client = {
        Client::new()
    };
}

#[derive(Deserialize, Debug)]
pub struct GelbooruFeedGenerator {
    api_key: String,
    user_id: String,
    pub taglist: Vec<String>,
}

impl GelbooruFeedGenerator {
    pub fn new(api_key: String, user_id: String, taglist: Vec<String>) -> GelbooruFeedGenerator {
        GelbooruFeedGenerator {
            api_key,
            user_id,
            taglist,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct GelbooruItem {
    pub source: String,
    pub directory: String,
    pub hash: String,
    pub width: u32,
    pub height: u32,
    pub id: u32,
    pub image: String,
    pub change: i64,
    pub owner: String,
    pub parent_id: Option<u32>,
    pub rating: String,
    pub sample: bool,
    pub sample_height: u32,
    pub sample_width: u32,
    pub score: u32,
    pub tags: String,
    pub file_url: String,
    pub created_at: String,
}

pub fn gelbooru_to_rss(g_item: GelbooruItem) -> RssResult<rss::Item> {
    use RssError::*;

    let enclosure = rss::EnclosureBuilder::default()
        .url(g_item.file_url.clone())
        .build()
        .map_err(StringError)?;

    let item = rss::ItemBuilder::default()
        .title(format!("{tags}", tags = g_item.tags))
        .link(g_item.file_url)
        .description(g_item.tags)
        .enclosure(enclosure)
        .pub_date(DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(g_item.change, 0), Utc).to_rfc2822())
        .build()
        .map_err(StringError)?;

    Ok(item)
}

fn gelbooru_to_items(vec: Vec<GelbooruItem>) -> impl Future<Item = Vec<rss::Item>, Error = RssError> {
    result(vec.into_iter().map(gelbooru_to_rss).collect())
}

impl FeedGenerator for GelbooruFeedGenerator {
    fn get_items(&self, number: u32) -> Box<Future<Item = Vec<rss::Item>, Error = RssError>> {
        use RssError::*;
        
        let tags = &self.taglist.join(" ");
        let tags = utf8_percent_encode(tags, NON_ALPHANUMERIC);
    
        Box::new(CLIENT.get(&format!("https://gelbooru.com/index.php?json=1&page=dapi&s=post&q=index&tags={tags}&user_id={uid}&api_key={key}&limit={limit}", tags = tags, uid = self.user_id, key = self.api_key, limit = number))
            .send()
            .and_then(|mut res| res.json::<Vec<GelbooruItem>>())
            .map_err(ReqwestError)
            .and_then(gelbooru_to_items)
        )
    }
}


