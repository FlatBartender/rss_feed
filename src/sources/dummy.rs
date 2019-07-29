use crate::common::*;

fn default_limit() -> usize {
    10
}

#[derive(Deserialize, Debug)]
pub struct DummyFeedGenerator {
    #[serde(default = "default_limit")]
    limit: usize,
}

impl FeedGenerator for DummyFeedGenerator {
    fn get_items(&self) -> RssResult<Vec<rss::Item>> {
        use RssError::*;

        trace!("DummyFeedGenerator used");
        let item = rss::ItemBuilder::default()
            .title("dummy".to_string())
            .link("dummy.link".to_string())
            .description("dummy description".to_string())
            .build()
            .map_err(StringError)?;

        let res: Vec<rss::Item> = std::iter::repeat(item).take(self.limit).collect();

        Ok(res)
    }
}
