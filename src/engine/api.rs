use super::super::models;

use rand::Rng;

pub struct Api {
    db: models::Db,
}

impl Api {
    pub fn new(db: models::Db) -> Api {
        Api { db }
    }

    pub fn get_random_card(&self) -> &models::Card {
        let cards = self.db.cards();
        &cards[rand::thread_rng().gen_range(0, cards.len() - 1)]
    }
}
