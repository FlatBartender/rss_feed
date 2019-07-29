use crate::common::*;

fn default_limit() -> usize {
    10
}

#[derive(Deserialize, Debug, Clone)]
pub struct DummyFeedGenerator {
    #[serde(default = "default_limit")]
    limit: usize,
}

impl FeedGenerator for DummyFeedGenerator {
    fn get_items(&self) -> Box<Future<Item = Vec<rss::Item>, Error = RssError>> {
        use RssError::*;

        trace!("DummyFeedGenerator used");
        let item = rss::ItemBuilder::default()
            .title("dummy".to_string())
            .link("dummy.link".to_string())
            .description("dummy description".to_string())
            .build()
            .map_err(StringError).unwrap();

        let res: Vec<rss::Item> = std::iter::repeat(item).take(self.limit).collect();

        Box::new(futures::future::result(Ok(res)))
    }
}
