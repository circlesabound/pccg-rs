use crate as engine;
use chrono::Utc;
use engine::{constants, job_board::JobBoard, job_board::JobTier, ErrorCode};
use futures::future;
use pccg_rs_models::{Card, Character, CharacterEx, Job, JobPrototype, User};
use pccg_rs_storage::firestore::{FirestoreClient, TransactionType};
use rand::Rng;
use std::{convert::TryInto, sync::Arc};
use uuid::Uuid;

pub struct Api {
    cards: FirestoreClient,
    job_board: JobBoard,
    users: FirestoreClient,
}

impl Api {
    pub async fn new(cards: FirestoreClient, job_board: JobBoard, users: FirestoreClient) -> Api {
        Api {
            cards,
            job_board,
            users,
        }
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
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;
        let ret = match self.cards.get::<Card>(&card.id, Some(&t)).await? {
            Some(_) => AddOrUpdateOperation::Update,
            None => AddOrUpdateOperation::Add,
        };
        self.cards.upsert(&card.id.clone(), card, Some(&t)).await?;
        self.cards.commit_transaction(t).await?;
        Ok(ret)
    }

    pub async fn claim_daily_for_user(&self, user_id: &Uuid) -> engine::Result<u32> {
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;
        match self.users.get::<User>(user_id, Some(&t)).await? {
            Some(mut user) => {
                if user.daily_last_claimed.date() < Utc::now().date() {
                    let new_currency_amount = user.currency + constants::DAILY;
                    user.currency = new_currency_amount;
                    user.daily_last_claimed = Utc::now();

                    self.users.upsert(user_id, user, Some(&t)).await?;
                    self.users.commit_transaction(t).await?;
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
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;
        if let Some(_) = self.users.get::<User>(user_id, Some(&t)).await? {
            self.users.delete::<User>(user_id, Some(&t)).await?;
            Ok(self.users.commit_transaction(t).await?)
        } else {
            Err(engine::Error::new(ErrorCode::UserNotFound, None))
        }
    }

    pub async fn draw_card_to_stage_for_user(&self, user_id: &Uuid) -> engine::Result<u32> {
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;

        // Get user
        let mut user = self
            .users
            .get::<User>(user_id, Some(&t))
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
            self.users.upsert(user_id, user, Some(&t)).await?;
            self.users.commit_transaction(t).await?;

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
        Ok(self.cards.get::<Card>(card_id, None).await?)
    }

    pub async fn get_characters_for_user(
        &self,
        user_id: &Uuid,
    ) -> engine::Result<Vec<CharacterEx>> {
        let t = self
            .users
            .begin_transaction(TransactionType::ReadOnly)
            .await?;
        match self.users.get::<User>(user_id, Some(&t)).await? {
            Some(_) => {
                let fs = FirestoreClient::new_for_subcollection(
                    &self.users,
                    user_id.to_string(),
                    "characters".to_owned(),
                );

                let characters = fs.list::<Character>().await?;
                // TODO replace with batchget or join query
                for ch in characters.iter() {
                    let prototype = self.cards.get(&ch.prototype_id, Some(&t)).await?.unwrap();
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

        let t = fs.begin_transaction(TransactionType::ReadOnly).await?;

        let character = fs.get::<Character>(character_id, Some(&t)).await?;
        if let Some(character) = character {
            match self
                .cards
                .get::<Card>(&character.prototype_id, Some(&t))
                .await?
            {
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

    pub async fn list_available_jobs(&self, tier: &JobTier) -> engine::Result<Vec<JobPrototype>> {
        Ok(self.job_board.list_available_jobs(tier).await)
    }

    pub async fn get_current_job_for_character(
        &self,
        user_id: &Uuid,
        character_id: &Uuid,
    ) -> engine::Result<Option<Job>> {
        let fs = Arc::new(FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        ));

        // TODO replace with query
        let jobs = fs.list::<Job>().await?;
        Ok(jobs
            .into_iter()
            .find(|j| j.character_ids.contains(character_id)))
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
        let t = self
            .users
            .begin_transaction(TransactionType::ReadOnly)
            .await?;
        if let Some(user) = self.users.get::<User>(user_id, Some(&t)).await? {
            if let Some(staged_card_id) = user.staged_card {
                if let Some(card) = self.cards.get::<Card>(&staged_card_id, Some(&t)).await? {
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

    pub async fn list_jobs_for_user(&self, user_id: &Uuid) -> engine::Result<Vec<Job>> {
        match self.get_user_by_id(user_id).await? {
            Some(_) => {
                let fs = FirestoreClient::new_for_subcollection(
                    &self.users,
                    user_id.to_string(),
                    "jobs".to_owned(),
                );

                Ok(fs.list::<Job>().await?)
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
        Ok(self.users.get::<User>(user_id, None).await?)
    }

    pub async fn promote_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<Card> {
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;
        if let Some(mut user) = self.users.get::<User>(user_id, Some(&t)).await? {
            if let Some(staged_card_id) = user.staged_card {
                if staged_card_id == *requested_card_id {
                    if let Some(card) = self.cards.get::<Card>(&staged_card_id, Some(&t)).await? {
                        let character_id = Uuid::new_v4();
                        let character = Character::new(character_id, staged_card_id);
                        let fs = FirestoreClient::new_for_subcollection(
                            &self.users,
                            user_id.to_string(),
                            "characters".to_owned(),
                        );
                        fs.upsert(&character_id, character, Some(&t)).await?;
                        user.staged_card = None;
                        self.users.upsert(user_id, user, Some(&t)).await?;
                        self.users.commit_transaction(t).await?;
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
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;

        // Check valid user id
        if let None = self.users.get::<User>(&user_id, Some(&t)).await? {
            return Err(engine::Error::new(ErrorCode::UserNotFound, None));
        }

        let sw = std::time::Instant::now();

        // Check valid character ids
        let char_fs = Arc::new(FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "characters".to_owned(),
        ));

        let check_characters_exist_tasks: Vec<_> = character_ids
            .clone() // TODO figure out why borrowing here requires a static lifetime
            .into_iter()
            .map(|c| {
                let char_fs = Arc::clone(&char_fs);
                // BUG borrow restrictions prevent using transaction here
                tokio::spawn(async move {
                    // TODO replace with custom query
                    match char_fs.get::<Character>(&c, None).await {
                        Ok(o) => o.is_some(),
                        Err(e) => {
                            error!("Error validating character id: {:?}", e);
                            false
                        }
                    }
                })
            })
            .collect();

        let all_characters_exist = future::join_all(check_characters_exist_tasks)
            .await
            .into_iter()
            .all(|r| match r {
                Ok(r) => r,
                Err(e) => {
                    error!("Error joining future: {:?}", e);
                    false
                }
            });

        if !all_characters_exist {
            return Err(engine::Error::new(ErrorCode::CharacterNotFound, None));
        }

        // Check characters are not preoccupied with other jobs
        let ids_clone = character_ids.clone();
        let check_characters_not_preoccupied_tasks: Vec<_> = ids_clone
            .iter()
            .map(|c| self.get_current_job_for_character(&user_id, c))
            .collect();

        let all_characters_not_preoccupied =
            future::join_all(check_characters_not_preoccupied_tasks)
                .await
                .into_iter()
                .all(|r| match r {
                    Ok(r) => r.is_none(),
                    Err(e) => {
                        error!("Error joining future: {:?}", e);
                        false
                    }
                });

        if !all_characters_not_preoccupied {
            return Err(engine::Error::new(ErrorCode::CharacterPreoccupied, None));
        }

        debug!(
            "Completed precondition checks for take_job, took {:?}",
            sw.elapsed()
        );

        let job = self
            .job_board
            .create_job(job_prototype_id, user_id, character_ids)
            .await?;

        let job_fs = Arc::new(FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        ));

        job_fs.upsert(&job.id, job.clone(), Some(&t)).await?;
        match job_fs.commit_transaction(t).await {
            Ok(_) => Ok(job),
            Err(e) => Err(engine::Error::from(e)),
        }
    }

    pub async fn scrap_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<u32> {
        let t = self
            .users
            .begin_transaction(TransactionType::ReadWrite)
            .await?;
        if let Some(mut user) = self.users.get::<User>(user_id, Some(&t)).await? {
            if let Some(staged_card_id) = user.staged_card {
                if staged_card_id == *requested_card_id {
                    // Partial refund
                    let new_currency_amount = user.currency + constants::SCRAP_REFUND;
                    user.currency = new_currency_amount;
                    user.staged_card = None;
                    self.users.upsert(user_id, user, Some(&t)).await?;
                    self.users.commit_transaction(t).await?;
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
    use pccg_rs_storage::firestore::Firestore;
    use std::sync::Arc;
    use std::time::Duration;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[cfg(feature = "test_use_network")]
    #[tokio::test(threaded_scheduler)]
    async fn claim_daily_increases_currency_once() {
        tokio::spawn(async {
            let fs = Arc::new(Firestore::new(JSON_KEY_PATH).await.unwrap());
            let cards = FirestoreClient::new(Arc::clone(&fs), None, "_test_cards".to_owned());
            let users = FirestoreClient::new(Arc::clone(&fs), None, "_test_users".to_owned());
            let job_board = JobBoard::new(FirestoreClient::new(
                Arc::clone(&fs),
                None,
                "_test_jobs".to_owned(),
            ))
            .await;
            let api = Arc::new(Api::new(cards, job_board, users).await);

            tokio::time::delay_for(Duration::from_secs(2)).await;

            // Add a new user
            info!("[claim_daily_increases_currency_once] Deleting and adding new user");
            let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
            api.delete_user(&user_id).await.unwrap();
            tokio::time::delay_for(Duration::from_secs(2)).await;
            api.add_new_user(&user_id).await.unwrap();

            // Save the starting currency amount
            info!("[claim_daily_increases_currency_once] Fetching starting currency amount");
            let user = api.get_user_by_id(&user_id).await.unwrap().unwrap();
            let starting_currency = user.currency;

            tokio::time::delay_for(Duration::from_secs(2)).await;

            // Claim daily first time
            info!("[claim_daily_increases_currency_once] Claming daily once");
            let ret = api.claim_daily_for_user(&user_id).await;

            tokio::time::delay_for(Duration::from_secs(2)).await;

            // Claim daily second time
            info!("[claim_daily_increases_currency_once] Claming daily twice");
            let ret2 = api.claim_daily_for_user(&user_id).await;

            tokio::time::delay_for(Duration::from_secs(2)).await;

            // Fetch the updated currency amount
            info!("[claim_daily_increases_currency_once] Fetching updated curency amount");
            let user = api.get_user_by_id(&user_id).await.unwrap().unwrap();

            // Assert that the currency amount increased once
            info!("[claim_daily_increases_currency_once] Running assertions");
            assert!(ret.is_ok());
            assert!(ret2.is_err());
            assert!(user.currency > starting_currency);
            assert_eq!(user.currency - starting_currency, constants::DAILY);
        })
        .await
        .unwrap();
    }
}
