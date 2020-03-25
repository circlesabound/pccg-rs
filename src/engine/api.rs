use crate::engine;
use crate::models::{Card, Compendium, CompendiumError, User, UserRegistry, UserRegistryError};

use chrono::Utc;
use dashmap::mapref::entry::Entry::*;
use rand::Rng;
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

    pub async fn add_new_user(&self, id: Uuid) -> engine::Result<()> {
        let user = User::new(id);
        match self.user_registry.add_user(user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn add_card_to_user(&self, user_id: Uuid, card_id: Uuid) -> engine::Result<()> {
        // Check card exists
        match self.compendium.current.entry(card_id) {
            Vacant(_) => Err(CompendiumError::NotFound.into()),
            Occupied(_o) => {
                match self
                    .user_registry
                    .mutate_user_with(user_id, |u| Ok(u.cards.push(card_id)))
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    pub async fn get_owned_card_ids(&self, user_id: Uuid) -> engine::Result<Vec<Uuid>> {
        match self.user_registry.current.entry(user_id) {
            Vacant(_) => Err(UserRegistryError::NotFound.into()),
            Occupied(o) => Ok(o.get().cards.clone()),
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

    pub async fn get_user_ids(&self) -> engine::Result<Vec<Uuid>> {
        Ok(self
            .user_registry
            .current
            .iter()
            .map(|kvp| kvp.key().clone())
            .collect())
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> engine::Result<Option<User>> {
        Ok(self.user_registry.current.get(&id).map(|u| u.clone()))
    }

    pub async fn get_random_card(&self) -> engine::Result<Option<Card>> {
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

    pub async fn get_card_ids(&self) -> engine::Result<Vec<Uuid>> {
        Ok(self
            .compendium
            .current
            .iter()
            .map(|kvp| kvp.key().clone())
            .collect())
    }

    pub async fn get_card_by_id(&self, id: Uuid) -> engine::Result<Option<Card>> {
        Ok(self.compendium.current.get(&id).map(|c| c.clone()))
    }

    pub async fn add_or_update_card_in_compendium(
        &self,
        card: Card,
    ) -> engine::Result<AddOrUpdateOperation> {
        match self.compendium.upsert_card(card).await {
            Ok(None) => Ok(AddOrUpdateOperation::Add),
            Ok(Some(_)) => Ok(AddOrUpdateOperation::Update),
            Err(e) => Err(e.into()),
        }
    }
}

pub enum AddOrUpdateOperation {
    Add,
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStore;
    use futures::future;

    #[tokio::test]
    async fn claim_daily_increases_currency_only_once() {
        let api = Arc::new(new_in_memory_api().await);

        // Add a new user
        let user_id = Uuid::new_v4();
        api.add_new_user(user_id).await.unwrap();

        // Save the starting currency amount
        let user = api.get_user_by_id(user_id).await.unwrap().unwrap();
        let starting_currency = user.currency;

        // Spawn 20 tasks to claim daily
        let tasks: Vec<_> = std::iter::repeat(())
            .take(20)
            .map(|_| {
                let api = Arc::clone(&api);
                tokio::spawn(async move { api.claim_daily_for_user(user_id).await.is_ok() })
            })
            .collect();

        // Await all 20 tasks, assert that only 1 succeeded
        let completed_tasks = future::join_all(tasks).await;
        assert_eq!(
            completed_tasks
                .iter()
                .filter(|b| *b.as_ref().unwrap())
                .count(),
            1
        );

        // Fetch the updated currency amount, assert that it only increased once
        let user = api.get_user_by_id(user_id).await.unwrap().unwrap();
        assert!(user.currency > starting_currency);
        assert_eq!(user.currency - starting_currency, 200);
    }

    async fn new_in_memory_api() -> Api {
        let compendium = Compendium::from_storage(Arc::new(InMemoryStore::new().unwrap()))
            .await
            .unwrap();
        let user_registry = UserRegistry::from_storage(Arc::new(InMemoryStore::new().unwrap()))
            .await
            .unwrap();
        Api::new(compendium, user_registry).await
    }
}
