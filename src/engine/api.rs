use crate::models::{Card, Compendium, User, UserRegistry};

use rand::Rng;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

pub struct Api {
    compendium: Compendium,
    user_registry: UserRegistry,
}

impl Api {
    pub async fn new(compendium: Compendium, user_registry: UserRegistry) -> Api {
        Api {
            compendium,
            user_registry,
        }
    }

    pub async fn add_new_user(&self, id: Uuid) -> Result<(), Box<dyn Error>> {
        let user = User {
            id,
            cards: Vec::new(),
        };
        match self.user_registry.add_user(user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn get_random_card(&self) -> Result<Option<Card>, Box<dyn Error>> {
        let cards = Arc::clone(&self.compendium.current);
        if cards.is_empty() {
            return Ok(None);
        }

        // TODO find another way to do this
        let rnd = rand::thread_rng().gen_range(0, cards.len());
        let mut iter = cards.iter();
        for _ in 0..rnd {
            iter.next();
        }

        // Some funky borrow semantics here
        let ret = match iter.next() {
            Some(c) => Ok(Some(c.value().clone())),
            None => unreachable!(),
        };
        ret
    }

    pub async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>, Box<dyn Error>> {
        let cards = Arc::clone(&self.compendium.current);
        Ok(cards.get(&id).map(|c| c.clone()))
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
