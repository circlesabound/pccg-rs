use crate::models::{Card, Compendium};

use rand::Rng;
use std::error::Error;

pub struct Api {
    compendium: Compendium,
}

impl Api {
    pub async fn new(compendium: Compendium) -> Api {
        Api { compendium }
    }

    pub async fn get_random_card(&self) -> Option<Card> {
        let cards = self.compendium.current.read().await;
        if cards.len() == 0 {
            None
        } else {
            Some(cards[rand::thread_rng().gen_range(0, cards.len())].clone())
        }
    }

    pub async fn add_or_update_card_in_compendium(
        &self,
        card: Card,
    ) -> Result<AddOrUpdateOperation, Box<dyn Error>> {
        match self.compendium.upsert_card(card).await {
            Ok(None) => Ok(AddOrUpdateOperation::Add),
            Ok(Some(_)) => Ok(AddOrUpdateOperation::Update),
            Err(e) => Err(Box::new(e)),
        }
    }
}

pub enum AddOrUpdateOperation {
    Add,
    Update,
}