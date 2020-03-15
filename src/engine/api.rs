use super::super::models;

use futures::executor::block_on;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Api {
    compendium: RwLock<Arc<models::Compendium>>,
}

impl Api {
    pub fn new<'a, I>(
        compendium_cards_iter: I
    ) -> Api
    where
        I: Iterator<Item = &'a models::Card>
    {
        let api = Api {
            compendium: RwLock::new(Default::default()),
        };

        let populate_compendium_future = async {
            let mut compendium = models::Compendium::new();
            for card in compendium_cards_iter {
                compendium.cards.push(card.clone());
            }
            *api.compendium.write().await = Arc::new(compendium);
        };

        block_on(populate_compendium_future);

        api
    }

    pub async fn get_random_card(&self) -> models::Card {
        let compendium = self.compendium.read().await;
        let cards = &compendium.cards;
        cards[rand::thread_rng().gen_range(0, cards.len() - 1)].clone()
    }
}
