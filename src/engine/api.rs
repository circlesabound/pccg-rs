use crate::{
    engine::{self, ErrorCode},
    models::{Card, User},
    storage::firestore::Firestore,
};
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

pub struct Api {
    cards: Firestore,
    users: Firestore,
}

impl Api {
    pub async fn new(cards: Firestore, users: Firestore) -> Api {
        Api { cards, users }
    }

    pub async fn add_new_user(&self, user_id: &Uuid) -> engine::Result<()> {
        let user = User::new(*user_id);
        match self.users.insert(user_id, user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn add_card_to_user(&self, user_id: &Uuid, card_id: &Uuid) -> engine::Result<Card> {
        // Check user exists
        match self.users.get::<User>(user_id).await? {
            Some(mut user) => {
                // Check card exists
                match self.cards.get::<Card>(card_id).await? {
                    Some(card) => {
                        user.cards.push(*card_id);
                        self.users.upsert(user_id, user).await?;
                        Ok(card)
                    }
                    None => Err(engine::Error::new(ErrorCode::CardNotFound, None)),
                }
            }
            None => Err(engine::Error::new(ErrorCode::UserNotFound, None)),
        }
    }

    pub async fn add_random_card_to_user(&self, user_id: &Uuid) -> engine::Result<Card> {
        let card = self.get_random_card().await?;
        self.add_card_to_user(user_id, &card.id).await
    }

    pub async fn get_owned_card_ids(&self, user_id: &Uuid) -> engine::Result<Vec<Uuid>> {
        match self.users.get::<User>(user_id).await? {
            Some(user) => Ok(user.cards),
            None => Err(engine::Error::new(ErrorCode::UserNotFound, None)),
        }
    }

    pub async fn claim_daily_for_user(&self, user_id: &Uuid) -> engine::Result<u32> {
        match self.users.get::<User>(user_id).await? {
            Some(mut user) => {
                if user.daily_last_claimed.date() < Utc::now().date() {
                    let new_currency_amount = user.currency + 200;
                    user.currency = new_currency_amount;
                    user.daily_last_claimed = Utc::now();

                    self.users.upsert(user_id, user).await?;
                    Ok(new_currency_amount)
                } else {
                    Err(engine::Error::new(ErrorCode::DailyAlreadyClaimed, None))
                }
            }
            None => Err(engine::Error::new(ErrorCode::UserNotFound, None)),
        }
    }

    pub async fn get_user_ids(&self) -> engine::Result<Vec<Uuid>> {
        // TODO find a way to do this without a full db enumeration
        Ok(self
            .users
            .list::<User>()
            .await?
            .into_iter()
            .map(|u| u.id)
            .collect())
    }

    pub async fn get_user_by_id(&self, user_id: &Uuid) -> engine::Result<Option<User>> {
        Ok(self.users.get::<User>(user_id).await?)
    }

    pub async fn get_random_card(&self) -> engine::Result<Card> {
        let mut cards = self.cards.list::<Card>().await?;
        if cards.is_empty() {
            Err(engine::Error::new(ErrorCode::CompendiumEmpty, None))
        } else {
            let rnd = rand::thread_rng().gen_range(0, cards.len());
            Ok(cards.swap_remove(rnd))
        }
    }

    pub async fn get_card_ids(&self) -> engine::Result<Vec<Uuid>> {
        // TODO find a way to do this without a full db enumeration
        Ok(self
            .cards
            .list::<Card>()
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect())
    }

    pub async fn get_card_by_id(&self, card_id: &Uuid) -> engine::Result<Option<Card>> {
        Ok(self.cards.get::<Card>(card_id).await?)
    }

    pub async fn add_or_update_card_in_compendium(
        &self,
        card: Card,
    ) -> engine::Result<AddOrUpdateOperation> {
        // TODO figure out how to do this without 2 firestore calls
        let ret = match self.cards.get::<Card>(&card.id).await? {
            Some(_) => AddOrUpdateOperation::Update,
            None => AddOrUpdateOperation::Add,
        };
        self.cards.upsert(&card.id.clone(), card).await?;
        Ok(ret)
    }
}

pub enum AddOrUpdateOperation {
    Add,
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future;
    use std::sync::Arc;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn claim_daily_increases_currency_only_once() {
        tokio::spawn(async {
            pretty_env_logger::init();
            let cards = Firestore::new(JSON_KEY_PATH, "cards".to_owned())
                .await
                .unwrap();
            let users = Firestore::new(JSON_KEY_PATH, "users".to_owned())
                .await
                .unwrap();
            let api = Arc::new(Api::new(cards, users).await);

            // Add a new user
            let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
            api.add_new_user(&user_id).await.unwrap();

            // Save the starting currency amount
            let user = api.get_user_by_id(&user_id).await.unwrap().unwrap();
            let starting_currency = user.currency;

            // Spawn 20 tasks to claim daily
            let tasks: Vec<_> = std::iter::repeat(())
                .take(20)
                .map(|_| {
                    let api = Arc::clone(&api);
                    tokio::spawn(async move { api.claim_daily_for_user(&user_id).await.is_ok() })
                })
                .collect();

            // Await all 20 tasks, assert that at least 1 succeeded
            let completed_tasks = future::join_all(tasks).await;
            assert!(
                completed_tasks
                    .iter()
                    .filter(|b| *b.as_ref().unwrap())
                    .count()
                    >= 1
            );

            // Fetch the updated currency amount, assert that it only increased once
            let user = api.get_user_by_id(&user_id).await.unwrap().unwrap();
            assert!(user.currency > starting_currency);
            assert_eq!(user.currency - starting_currency, 200);
        })
        .await
        .unwrap();
    }
}
