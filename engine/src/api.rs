use crate as engine;
use chrono::Utc;
use engine::{
    constants, experience, job_board::JobBoard, job_board::JobTier, ErrorCategory, ErrorCode,
};
use futures::future;
use pccg_rs_models::{
    Card, Character, CharacterEx, ExperienceGain, Job, JobCompletionReport, JobPrototype, User,
};
use pccg_rs_storage::firestore::{FirestoreClient, Transaction, TransactionType};
use rand::Rng;
use std::{convert::TryInto, sync::Arc, time::Duration};
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

    // ######################
    // # Account management #
    // ######################

    pub async fn add_user(&self, user_id: &Uuid) -> engine::Result<()> {
        let mut user = User::new(*user_id);
        user.currency = constants::USER_STARTING_CURRENCY;
        match self.users.insert(user_id, user).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn delete_user(&self, user_id: &Uuid) -> engine::Result<()> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
                // Check user exists
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadWrite)
                    .await?;
                if let Some(_) = self.users.get::<User>(user_id, Some(&t)).await? {
                    self.users.delete::<User>(user_id, Some(&t)).await?;
                    t.commit().await?;
                    Ok(())
                } else {
                    t.commit().await?;
                    Err(engine::Error::new(ErrorCode::UserNotFound, None))
                }
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    pub async fn get_user(&self, user_id: &Uuid) -> engine::Result<Option<User>> {
        Ok(self.users.get::<User>(user_id, None).await?)
    }

    pub async fn list_user_ids(&self) -> engine::Result<Vec<Uuid>> {
        // TODO find a way to do this without a full db enumeration
        Ok(self
            .users
            .list::<User>()
            .await?
            .into_iter()
            .map(|u| u.id)
            .collect())
    }

    // ##############
    // # Compendium #
    // ##############

    pub async fn add_or_update_card_in_compendium(
        &self,
        card: Card,
    ) -> engine::Result<AddOrUpdateOperation> {
        let mut retries: usize = 2;
        loop {
            let ret: engine::Result<AddOrUpdateOperation> = async {
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadWrite)
                    .await?;
                // TODO figure out how to do this without 2 firestore calls
                let ret = match self.cards.get::<Card>(&card.id, Some(&t)).await? {
                    Some(_) => AddOrUpdateOperation::Update,
                    None => AddOrUpdateOperation::Add,
                };
                self.cards.upsert(&card.id, card.clone(), Some(&t)).await?;
                t.commit().await?;
                Ok(ret)
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
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

    pub async fn get_card(&self, card_id: &Uuid) -> engine::Result<Option<Card>> {
        Ok(self.cards.get::<Card>(card_id, None).await?)
    }

    pub async fn list_card_ids(&self) -> engine::Result<Vec<Uuid>> {
        // TODO find a way to do this without a full db enumeration
        Ok(self
            .cards
            .list::<Card>()
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect())
    }

    // #############
    // # Job board #
    // #############

    pub async fn list_available_jobs(&self, tier: &JobTier) -> engine::Result<Vec<JobPrototype>> {
        Ok(self.job_board.list_available_jobs(tier).await)
    }

    // ###################
    // # User characters #
    // ###################

    pub async fn get_character(
        &self,
        user_id: &Uuid,
        character_id: &Uuid,
    ) -> engine::Result<Option<CharacterEx>> {
        let fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "characters".to_owned(),
        );

        let mut retries: usize = 2;
        loop {
            let ret = async {
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
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
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

    pub async fn list_characters(&self, user_id: &Uuid) -> engine::Result<Vec<CharacterEx>> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
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
                        let prototypes = self
                            .cards
                            .batch_get::<Card>(
                                &characters.iter().map(|ch| ch.prototype_id).collect(),
                                Some(&t),
                            )
                            .await?;

                        for ch in characters.iter() {
                            let prototype = prototypes[&ch.prototype_id].clone().unwrap();
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
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    // ################
    // # User economy #
    // ################

    pub async fn draw_card(&self, user_id: &Uuid) -> engine::Result<u32> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
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
                    t.commit().await?;

                    Ok(new_currency_amount)
                }
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    pub async fn get_staged_card(&self, user_id: &Uuid) -> engine::Result<Option<Card>> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadOnly)
                    .await?;
                if let Some(user) = self.users.get::<User>(user_id, Some(&t)).await? {
                    if let Some(staged_card_id) = user.staged_card {
                        if let Some(card) =
                            self.cards.get::<Card>(&staged_card_id, Some(&t)).await?
                        {
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
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    pub async fn promote_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<Card> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadWrite)
                    .await?;
                if let Some(mut user) = self.users.get::<User>(user_id, Some(&t)).await? {
                    if let Some(staged_card_id) = user.staged_card {
                        if staged_card_id == *requested_card_id {
                            if let Some(card) =
                                self.cards.get::<Card>(&staged_card_id, Some(&t)).await?
                            {
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
                                t.commit().await?;
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
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    pub async fn scrap_staged_card(
        &self,
        user_id: &Uuid,
        requested_card_id: &Uuid,
    ) -> engine::Result<u32> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
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
                            t.commit().await?;
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
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    // #############
    // # User jobs #
    // #############

    pub async fn cancel_job(&self, user_id: &Uuid, job_id: &Uuid) -> engine::Result<()> {
        let fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        );

        Ok(fs.delete::<Job>(job_id, None).await?)
    }

    pub async fn complete_job(
        &self,
        user_id: &Uuid,
        job_id: &Uuid,
    ) -> engine::Result<JobCompletionReport> {
        let char_fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "characters".to_owned(),
        );
        let job_fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        );

        let mut retries: usize = 2;
        loop {
            let ret = async {
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadWrite)
                    .await?;
                if let Some(job) = job_fs.get::<Job>(job_id, Some(&t)).await? {
                    if job.can_complete() {
                        // Generate completion report
                        let report = self.generate_job_completion_report(job, &t).await?;

                        // Apply currency rewards
                        let mut user = self.users.get::<User>(user_id, Some(&t)).await?.expect("User assumed to exist");
                        user.currency += report.currency_gain;
                        self.users.upsert(user_id, user, Some(&t)).await?;

                        // Apply experience changes
                        let char_ids: Vec<Uuid> = report.experience_gain.iter().map(|eg| eg.character_id).collect();
                        let mut chars = char_fs.batch_get::<Character>(&char_ids, Some(&t)).await?;
                        for eg in report.experience_gain.iter() {
                            let mut ch = chars.remove(&eg.character_id).expect("Character assumed to exist").expect("Character assumed to exist");
                            ch.level = eg.level_after;
                            ch.experience = eg.exp_after;
                            char_fs.upsert(&eg.character_id, ch, Some(&t)).await?;
                        }

                        // Delete job
                        job_fs.delete::<Job>(job_id, Some(&t)).await?;

                        // Commit transaction
                        t.commit().await?;

                        Ok(report)
                    } else {
                        Err(engine::Error::new(ErrorCode::JobNotComplete, None))
                    }
                } else {
                    Err(engine::Error::new(ErrorCode::JobNotFound, None))
                }
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    pub async fn get_job(&self, user_id: &Uuid, job_id: &Uuid) -> engine::Result<Option<Job>> {
        let fs = FirestoreClient::new_for_subcollection(
            &self.users,
            user_id.to_string(),
            "jobs".to_owned(),
        );

        Ok(fs.get::<Job>(job_id, None).await?)
    }

    pub async fn list_jobs(&self, user_id: &Uuid) -> engine::Result<Vec<Job>> {
        match self.get_user(user_id).await? {
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

    pub async fn take_job(
        &self,
        user_id: Uuid,
        job_prototype_id: &Uuid,
        character_ids: Vec<Uuid>,
    ) -> engine::Result<Job> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
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
                let char_map = char_fs
                    .batch_get::<Character>(&character_ids, Some(&t))
                    .await?;
                if !char_map.values().all(|o| o.is_some()) {
                    return Err(engine::Error::new(ErrorCode::CharacterNotFound, None));
                }

                // Check characters are not preoccupied with other jobs
                // TODO this sucks. query would be nice (or at least a batch version)
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
                    .create_job(job_prototype_id, user_id, character_ids.clone())
                    .await?;

                let job_fs = Arc::new(FirestoreClient::new_for_subcollection(
                    &self.users,
                    user_id.to_string(),
                    "jobs".to_owned(),
                ));

                job_fs.upsert(&job.id, job.clone(), Some(&t)).await?;
                t.commit().await?;

                Ok(job)
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }

    async fn generate_job_completion_report(
        &self,
        job: Job,
        transaction: &Transaction,
    ) -> engine::Result<JobCompletionReport> {
        let char_fs = FirestoreClient::new_for_subcollection(
            &self.users,
            job.user_id.to_string(),
            "characters".to_owned(),
        );

        let char_map = char_fs
            .batch_get::<Character>(&job.character_ids, Some(transaction))
            .await?;
        if !char_map.values().all(|c| c.is_some()) {
            // TODO handle this properly
            todo!();
        }
        let chars: Vec<Character> = char_map.into_iter().map(|(_, o)| o.unwrap()).collect();

        // TODO un-hardcode this
        let mut exp_gains = vec![];
        for ch in chars.into_iter() {
            let exp_gain = 70;
            let (level_after, exp_after) =
                experience::experience_add(ch.level, ch.experience, exp_gain);
            exp_gains.push(ExperienceGain {
                character_id: ch.id,
                level_before: ch.level,
                exp_before: ch.experience,
                level_after,
                exp_after,
                exp_gain,
            });
        }

        Ok(JobCompletionReport {
            job,
            currency_gain: 70,
            experience_gain: exp_gains,
        })
    }

    // #############
    // # User misc #
    // #############

    pub async fn claim_user_daily_reward(&self, user_id: &Uuid) -> engine::Result<u32> {
        let mut retries: usize = 2;
        loop {
            let ret = async {
                let t = self
                    .users
                    .begin_transaction(TransactionType::ReadWrite)
                    .await?;
                match self.users.get::<User>(user_id, Some(&t)).await? {
                    Some(mut user) => {
                        if user.daily_last_claimed.date() < Utc::now().date() {
                            let new_currency_amount =
                                user.currency + constants::DAILY_CURRENCY_REWARD;
                            user.currency = new_currency_amount;
                            user.daily_last_claimed = Utc::now();

                            self.users.upsert(user_id, user, Some(&t)).await?;
                            t.commit().await?;
                            Ok(new_currency_amount)
                        } else {
                            Err(engine::Error::new(ErrorCode::DailyAlreadyClaimed, None))
                        }
                    }
                    None => Err(engine::Error::new(ErrorCode::UserNotFound, None)),
                }
            }
            .await;

            match ret {
                Err(ref e) if retries > 0 => {
                    if let ErrorCategory::InternalRetryable = e.classify() {
                        info!("Caught retryable error, {} retries remaining", retries);
                        retries -= 1;
                        tokio::time::delay_for(Duration::from_millis(300)).await;
                    } else {
                        break ret;
                    }
                }
                _ => break ret,
            }
        }
    }
}

pub enum AddOrUpdateOperation {
    Add,
    Update,
}
