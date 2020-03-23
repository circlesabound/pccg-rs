use crate::engine;
use crate::models::{
    Card, Compendium, CompendiumError, CompendiumReadError, User, UserRegistry, UserRegistryError,
};

use chrono::{DateTime, TimeZone, Utc};
use dashmap::mapref::entry::Entry::*;
use rand::Rng;
use std::convert::Infallible;
use std::error;
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

    pub async fn add_new_user(&self, id: Uuid) -> Result<(), Box<dyn error::Error>> {
        let user = User::new(id);
        match self.user_registry.add_user(user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn add_card_to_user(
        &self,
        user_id: Uuid,
        card_id: Uuid,
    ) -> Result<(), Box<dyn error::Error>> {
        // Check card exists
        match self.compendium.current.entry(card_id) {
            Vacant(_) => Err(Box::new(CompendiumError::Read(
                CompendiumReadError::CardNotFound(card_id),
            ))),
            Occupied(_o) => {
                match self
                    .user_registry
                    .mutate_user_with(user_id, |u| Ok(u.cards.push(card_id)))
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    pub async fn get_owned_card_ids(
        &self,
        user_id: Uuid
    ) -> engine::Result<Vec<Uuid>> {
        match self.user_registry.current.entry(user_id) {
            Vacant(_) => Err(engine::Error::new(
                engine::ErrorCode::UserNotFound,
                None
            )),
            Occupied(o) => {
                Ok(o.get().cards.clone())
            }
        }
    }

    pub async fn claim_daily_for_user(&self, user_id: Uuid) -> engine::Result<u32> {
        match self
            .user_registry
            .mutate_user_if(
                user_id,
                |u| u.daily_last_claimed.date() < Utc::now().date(),
                |u| {
                    u.currency += 200;
                    u.daily_last_claimed = Utc::now();
                    Ok(u.currency)
                },
            )
            .await
        {
            Ok(currency) => Ok(currency),
            Err(e) => {
                if let UserRegistryError::FailedPrecondition = e {
                    Err(engine::Error::new(
                        engine::ErrorCode::DailyAlreadyClaimed,
                        Some(e.into()),
                    ))
                } else {
                    Err(engine::Error::from(e))
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

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, Box<dyn error::Error>> {
        Ok(self.user_registry.current.get(&id).map(|u| u.clone()))
    }

    pub async fn get_random_card(&self) -> Result<Option<Card>, Box<dyn error::Error>> {
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

    pub async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>, Box<dyn error::Error>> {
        Ok(self.compendium.current.get(&id).map(|c| c.clone()))
    }

    pub async fn add_or_update_card_in_compendium(
        &self,
        card: Card,
    ) -> Result<AddOrUpdateOperation, Box<dyn error::Error>> {
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
