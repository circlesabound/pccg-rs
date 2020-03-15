use super::super::models;

use rand::Rng;
use std::sync::Arc;
use tokio::stream::{self, StreamExt};
use tokio::sync::RwLock;

pub struct Api {
    compendium: RwLock<Arc<models::Compendium>>,
}

impl Api {
    pub async fn new<'a, I>(
        mut compendium_cards_stream: I
    ) -> Api
    where
        I: stream::Stream<Item = &'a models::Card> + Unpin
    {
        let api = Api {
            compendium: RwLock::new(Default::default()),
        };

        let populate_compendium_future = async {
            let mut compendium = models::Compendium::new();
            while let Some(card) = compendium_cards_stream.next().await {
                compendium.cards.push(card.clone());
            }
            *api.compendium.write().await = Arc::new(compendium);
        };

        tokio::join!(
            populate_compendium_future
        );

        api
    }

    pub async fn get_random_card(&self) -> models::Card {
        let compendium = self.compendium.read().await;
        let cards = &compendium.cards;
        cards[rand::thread_rng().gen_range(0, cards.len() - 1)].clone()
    }
}
