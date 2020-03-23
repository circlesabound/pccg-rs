use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub cards: Vec<Uuid>,
    pub currency: u32,
    pub daily_last_claimed: DateTime<Utc>,
}

impl User {
    pub fn new(id: Uuid) -> User {
        User {
            id,
            cards: vec![],
            currency: 0,
            daily_last_claimed: Utc.timestamp(0, 0),
        }
    }
}
