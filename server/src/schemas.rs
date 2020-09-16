use crate::models;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ###################
// # Request schemas #
// ###################

#[derive(Deserialize)]
pub struct AddCardToUserRequest {
    pub user_id: Uuid,
    pub card_id: Uuid,
}

#[derive(Deserialize)]
pub struct ConfirmStagedCardRequest {
    pub card_id: Uuid,
    pub action: StagedCardAction,
}

#[derive(Deserialize)]
pub enum StagedCardAction {
    Promote,
    Scrap,
}

#[derive(Deserialize)]
pub struct PutCardToCompendiumRequest {
    pub card: models::Card,
}

#[derive(Deserialize)]
pub struct RecallJobRequest {
    pub action: RecallJobAction,
}

#[derive(Deserialize)]
pub enum RecallJobAction {
    Cancel,
    Complete,
}

#[derive(Deserialize)]
pub struct TakeJobRequest {
    pub job_prototype_id: Uuid,
    pub character_ids: Vec<Uuid>,
}

// ####################
// # Response schemas #
// ####################

#[derive(Serialize)]
pub struct ClaimDailyForUserResponse {
    pub user_id: Uuid,
    pub currency: u32,
}

#[derive(Serialize)]
pub struct CompleteJobResponse {
    // TODO
}

#[derive(Serialize)]
pub struct DrawCardToStageForUserResponse {
    pub user_id: Uuid,
    pub currency: u32,
}

#[derive(Serialize)]
pub struct ListCharactersForUserResponse {
    pub characters: Vec<models::CharacterEx>,
}

pub type ListCardsFromCompendiumResponse = Vec<Uuid>;

pub type ListUsersFromRegistryResponse = Vec<Uuid>;

#[derive(Serialize)]
pub struct ScrapCardResponse {
    pub user_id: Uuid,
    pub currency: u32,
}
