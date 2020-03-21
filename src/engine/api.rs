use crate::models::{Card, Compendium};

use std::error::Error;

pub struct Api {
    compendium: Compendium,
}

impl Api {
    pub async fn new(compendium: Compendium) -> Api {
        Api { compendium }
    }

    pub async fn get_random_card(&self) -> Result<Option<Card>, Box<dyn Error>> {
        match self.compendium.get_random_card().await {
            Ok(Some(c)) => Ok(Some(c)),
            Ok(None) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn get_card_by_id(&self, id: uuid::Uuid) -> Result<Option<Card>, Box<dyn Error>> {
        match self.compendium.get_card_by_id(id).await {
            Ok(Some(c)) => Ok(Some(c)),
            Ok(None) => Ok(None),
            Err(e) => Err(Box::new(e)),
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
