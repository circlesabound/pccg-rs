use crate::{
    engine::{self, constants, ErrorCode, job_board::JobBoard, job_board::JobTier},
    models::{Card, Character, CharacterEx, Job, JobPrototype, User},
    storage::firestore::FirestoreClient,
};
use chrono::Utc;
use futures::future;
use rand::Rng;
use std::{sync::Arc, convert::TryInto};
use uuid::Uuid;

pub struct Api {
    cards: FirestoreClient,
    job_board: JobBoard,
    users: FirestoreClient,
}

impl Api {
    pub async fn new(cards: FirestoreClient, job_board: JobBoard, users: FirestoreClient) -> Api {
        Api { cards, job_board, users }
    }

    pub async fn add_new_user(&self, user_id: &Uuid) -> engine::Result<()> {
        let mut user = User::new(*user_id);
        user.currency = constants::USER_STARTING_CURRENCY;
        match self.users.insert(user_id, user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
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

    pub async fn claim_daily_for_user(&self, user_id: &Uuid) -> engine::Result<u32> {
        match self.users.get::<User>(user_id).await? {
            Some(mut user) => {
                if user.daily_last_claimed.date() < Utc::now().date() {
                    let new_currency_amount = user.currency + constants::DAILY;
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

    pub async fn delete_user(&self, user_id: &Uuid) -> engine::Result<()> {
        // Check user exists
        if let Some(_) = self.users.get::<User>(user_id).await? {
            Ok(self.users.delete::<User>(user_id).await?)
        } else {
            Err(engine::Error::new(ErrorCode::UserNotFound, None))
        }
    }

    pub async fn draw_card_to_stage_for_user(&self, user_id: &Uuid) -> engine::Result<u32> {
        // Get user
        let mut user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or(engine::Error::new(ErrorCode::UserNotFound, None))?;

        // Check preconditions
        if user.currency < constants::DRAW_COST {
            Err(engine::Error::new(ErrorCode::InsufficientFunds, None))
        } else if let Some(_) = user.staged_card {
            Err(engine::Error::new(ErrorCode::DrawStagePopulated, None))
        } else {
            // Subtract funds
            let new_currency_amount = user.currency - constants::DRAW_COST;
            user.currency = new_currency_amount;

            // Draw random card
            let card = self.get_random_card().await?;

            // Add to stage
            user.staged_card = Some(card.id);

            // Commit to storage
            self.users.upsert(user_id, user).await?;

            Ok(new_currency_amount)
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

    pub async fn get_characters_for_user(
        &self,
        user_id: &Uuid,
    ) -> engine::Result<Vec<CharacterEx>> {
        match self.get_user_by_id(user_id).await? {
            Some(_) => {
                let fs = FirestoreClient::new_for_subcollection(
                    &self.users,
                    user_id.to_string(),
                    "characters".to_owned(),
                );

                let characters = fs.list::<Character>().await?;
                for ch in characters.iter() {
                    let prototype = self.cards.get(&ch.prototype_id).await?.unwrap();
                    ch.expand(prototype).await;
                }

                Ok(characters
                    .into_iter()
                    .map(|ch| ch.try_into().unwrap())
                    .collect())
            }
            None => Err(engine::Error::new(ErrorCode::UserNotFound, None)),
        }
    }

    pub async fn get_character_for_user(
        &self,
        user_id: &Uuid,
        character_id: &Uuid,
    ) -> engine::Result<Option<CharacterEx>> {
        let fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "characters".to_owned(),
        );

        let character = fs.get::<Character>(character_id).await?;
        if let Some(character) = character {
            match self.users.get::<Card>(&character.prototype_id).await? {
                Some(prototype) => Ok(Some(CharacterEx::new(character, prototype).await)),
                None => {
                    error!("prototype with id {} not found", character.prototype_id);
                    Err(engine::Error::new(ErrorCode::Other, None))
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn list_available_jobs(
        &self,
        tier: &JobTier
    ) -> engine::Result<Vec<JobPrototype>> {
        Ok(self.job_board.list_available_jobs(tier).await)
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

    pub async fn get_staged_card(&self, user_id: &Uuid) -> engine::Result<Option<Card>> {
        if let Some(user) = self.users.get::<User>(user_id).await? {
            if let Some(staged_card_id) = user.staged_card {
                if let Some(card) = self.cards.get::<Card>(&staged_card_id).await? {
                    Ok(Some(card))
                } else {
                    // ID of staged card does not match a card in compendium
                    // Maybe it was removed?
                    error!(
                        "Staged card with id {} for user {} not found in compendium!",
                        staged_card_id, user_id
                    );
                    Err(engine::Error::new(ErrorCode::CardNotFound, None))
                }
            } else {
                Ok(None)
            }
        } else {
            Err(engine::Error::new(ErrorCode::UserNotFound, None))
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

    pub async fn promote_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<Card> {
        if let Some(mut user) = self.users.get::<User>(user_id).await? {
            if let Some(staged_card_id) = user.staged_card {
                if staged_card_id == *requested_card_id {
                    if let Some(card) = self.cards.get::<Card>(&staged_card_id).await? {
                        let character_id = Uuid::new_v4();
                        let character = Character::new(character_id, staged_card_id);
                        let fs = FirestoreClient::new_for_subcollection(
                            &self.users,
                            user_id.to_string(),
                            "characters".to_owned(),
                        );
                        fs.insert(&character_id, character).await?;
                        user.staged_card = None;
                        self.users.upsert(user_id, user).await?;
                        Ok(card)
                    } else {
                        // ID of staged card does not match a card in compendium
                        // Maybe it was removed?
                        error!(
                            "Staged card with id {} for user {} not found in compendium!",
                            staged_card_id, user_id
                        );
                        Err(engine::Error::new(ErrorCode::CardNotFound, None))
                    }
                } else {
                    // Requested card ID does not match the currently staged card ID
                    // Enforcing ID match mitigates the race condition caused by concurrent draws
                    Err(engine::Error::new(ErrorCode::IdMismatch, None))
                }
            } else {
                Err(engine::Error::new(ErrorCode::DrawStageEmpty, None))
            }
        } else {
            Err(engine::Error::new(ErrorCode::UserNotFound, None))
        }
    }

    pub async fn take_job(
        &self,
        user_id: Uuid,
        job_prototype_id: &Uuid,
        character_ids: Vec<Uuid>,
    ) -> engine::Result<Job> {
        // Check valid user id
        if let None = self.users.get::<User>(&user_id).await? {
            return Err(engine::Error::new(ErrorCode::UserNotFound, None));
        }

        // Check valid character ids
        let char_fs = Arc::new(FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "characters".to_owned(),
        ));

        let tasks: Vec<_> = character_ids.clone() // TODO figure out why borrowing here requires a static lifetime
            .into_iter()
            .map(|c| {
                let char_fs = Arc::clone(&char_fs);
                tokio::spawn(async move {
                    match char_fs.get::<Character>(&c).await {
                        Ok(o) => {
                            o.is_some()
                        },
                        Err(e) => {
                            error!("Error validating character id: {:?}", e);
                            false
                        }
                    }
                })
            })
            .collect();

        let all_valid = future::join_all(tasks).await
            .into_iter()
            .all(|r| r.unwrap());

        if !all_valid {
            return Err(engine::Error::new(ErrorCode::CardNotFound, None));
        }

        let job = self.job_board.create_job(job_prototype_id, user_id, character_ids).await?;

        let job_fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        );

        match job_fs.insert(&job.id, job.clone()).await {
            Ok(_) => Ok(job),
            Err(e) => Err(engine::Error::from(e)),
        }
    }

    pub async fn scrap_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<u32> {
        if let Some(mut user) = self.users.get::<User>(user_id).await? {
            if let Some(staged_card_id) = user.staged_card {
                if staged_card_id == *requested_card_id {
                    // Partial refund
                    let new_currency_amount = user.currency + constants::SCRAP_REFUND;
                    user.currency = new_currency_amount;
                    user.staged_card = None;
                    self.users.upsert(user_id, user).await?;
                    Ok(new_currency_amount)
                } else {
                    // Requested card ID does not match the currently staged card ID
                    // Enforcing ID match mitigates the race condition caused by concurrent draws
                    Err(engine::Error::new(ErrorCode::IdMismatch, None))
                }
            } else {
                Err(engine::Error::new(ErrorCode::DrawStageEmpty, None))
            }
        } else {
            Err(engine::Error::new(ErrorCode::UserNotFound, None))
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
    use crate::storage::firestore::Firestore;
    use std::sync::Arc;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn claim_daily_increases_currency_only_once() {
        tokio::spawn(async {
            let fs = Arc::new(Firestore::new(JSON_KEY_PATH).await.unwrap());
            let cards = FirestoreClient::new(
                Arc::clone(&fs),
                None,
                "_test_cards".to_owned()
            );
            let users = FirestoreClient::new(
                Arc::clone(&fs),
                None,
                "_test_users".to_owned()
            );
            let job_board = JobBoard::new(FirestoreClient::new(
                Arc::clone(&fs),
                None,
                "_test_jobs".to_owned(),
            )).await;
            let api = Arc::new(Api::new(cards, job_board, users).await);

            // Add a new user
            let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
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

            // Await all 20 tasks
            let completed_tasks = future::join_all(tasks).await;

            // Fetch the updated currency amount
            let user = api.get_user_by_id(&user_id).await.unwrap().unwrap();

            // Clean up before running assertions
            api.delete_user(&user_id).await.unwrap();

            // Assert that out of 20 tasks, at least 1 succceeded
            assert!(
                completed_tasks
                    .iter()
                    .filter(|b| *b.as_ref().unwrap())
                    .count()
                    >= 1
            );

            // Assert that the currency amount increased once
            assert!(user.currency > starting_currency);
            assert_eq!(user.currency - starting_currency, constants::DAILY);
        })
        .await
        .unwrap();
    }
}
