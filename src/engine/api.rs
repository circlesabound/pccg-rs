use crate::models;

use rand::Rng;

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
}
