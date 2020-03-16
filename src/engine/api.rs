use crate::models;

use rand::Rng;
use std::error::Error;

pub struct Api {
    compendium: models::Compendium,
}

impl Api {
    pub async fn new(compendium: models::Compendium) -> Api {
        Api { compendium }

        // let populate_compendium_future = async {
        //     let mut compendium = models::Compendium::new();
        //     while let Some(card) = compendium_cards_stream.next().await {
        //         compendium.cards.push(card.clone());
        //     }
        //     *api.compendium.write().await = Arc::new(compendium);
        // };

        // tokio::join!(populate_compendium_future);

        // api
    }

    pub async fn get_random_card(&self) -> models::Card {
        let cards = self.compendium.current.read().await;
        cards[rand::thread_rng().gen_range(0, cards.len() - 1)].clone()
    }

    pub async fn add_card_to_compendium(
        &self,
        card: models::Card,
    ) -> Result<models::Card, Box<dyn Error>> {
        match self.compendium.add_card(card).await {
            Ok(id) => {
                let cards = self.compendium.current.read().await;
                match cards.iter().find(|c| c.id == id) {
                    Some(c) => return Ok(c.clone()),
                    None => return Err("uh oh".into()),
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
}
