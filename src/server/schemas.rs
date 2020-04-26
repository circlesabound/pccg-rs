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

// ####################
// # Response schemas #
// ####################

#[derive(Serialize)]
pub struct ClaimDailyForUserResponse {
    pub user_id: Uuid,
    pub currency: u32,
}

#[derive(Serialize)]
pub struct DrawCardToStageForUserResponse {
    pub user_id: Uuid,
    pub currency: u32,
}

pub type ListCardsForUserResponse = Vec<Uuid>;

pub type ListCardsFromCompendiumResponse = Vec<Uuid>;

pub type ListUsersFromRegistryResponse = Vec<Uuid>;

#[derive(Serialize)]
pub struct ScrapCardResponse {
    pub user_id: Uuid,
    pub currency: u32,
}
