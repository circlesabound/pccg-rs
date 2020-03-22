use crate::models;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AddCardToUserRequest {
    pub user_id: Uuid,
    pub card_id: Uuid,
}

#[derive(Deserialize)]
pub struct PutCardToCompendiumRequest {
    pub card: models::Card,
}
