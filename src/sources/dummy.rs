use crate::common::*;

#[derive(Deserialize, Debug)]
pub struct DummyFeedGenerator;

impl FeedGenerator for DummyFeedGenerator {
    fn get_items(&self, number: u32) -> Box<Future<Item = Vec<rss::Item>, Error = RssError>> {
        use RssError::*;

        let item = rss::ItemBuilder::default()
            .title("dummy".to_string())
            .link("dummy.link".to_string())
            .description("dummy description".to_string())
            .build()
            .map_err(StringError).unwrap();

        let res: Vec<rss::Item> = std::iter::repeat(item).take(number as usize).collect();

        Box::new(futures::future::result(Ok(res)))
    }
}
