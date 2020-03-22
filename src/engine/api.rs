use crate::models::{Card, Compendium, CompendiumError, CompendiumReadError, User, UserRegistry};

use dashmap::mapref::entry::Entry::*;
use rand::Rng;
use std::convert::Infallible;
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

    pub async fn add_card_to_user(
        &self,
        user_id: Uuid,
        card_id: Uuid,
    ) -> Result<(), Box<dyn Error>> {
        // Check card exists
        match self.compendium.current.entry(card_id) {
            Vacant(_) => Err(Box::new(CompendiumError::Read(
                CompendiumReadError::CardNotFound(card_id),
            ))),
            Occupied(_o) => {
                // Add to user
                match self.user_registry.add_card_to_user(user_id, card_id).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    pub async fn get_user_ids(&self) -> Result<Vec<Uuid>, Infallible> {
        Ok(self
            .user_registry
            .current
            .iter()
            .map(|kvp| kvp.key().clone())
            .collect())
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, Box<dyn Error>> {
        Ok(self.user_registry.current.get(&id).map(|u| u.clone()))
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

    pub async fn get_card_ids(&self) -> Result<Vec<Uuid>, Infallible> {
        Ok(self
            .compendium
            .current
            .iter()
            .map(|kvp| kvp.key().clone())
            .collect())
    }

    pub async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>, Box<dyn Error>> {
        Ok(self.compendium.current.get(&id).map(|c| c.clone()))
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
