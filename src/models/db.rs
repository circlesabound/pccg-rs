use std::sync::Arc;

use rand::Rng;
use tokio::sync::Mutex;

use super::card::Card;

pub type Db = Arc<Mutex<DbRep>>;

pub fn new_db() -> Db {
    DbRep::new()
}

pub struct DbRep {
    cards: Vec<Card>
}

impl DbRep {
    fn new() -> Db {
        let mut pre_populated: Vec<Card> = Vec::with_capacity(4);
        pre_populated.push(Card::new(
            uuid::Uuid::parse_str("c95e4135-4503-4572-a8f0-37c40af11d5c").unwrap(),
            String::from("Iolo Kirby"),
            String::from("Sample card 0"),
            String::from("https://via.placeholder.com/300"),
        ));
        pre_populated.push(Card::new(
            uuid::Uuid::parse_str("456ed2bc-b32f-49ad-ba1f-f0de01334e81").unwrap(),
            String::from("Terrence Costa"),
            String::from("Sample card 1"),
            String::from("https://via.placeholder.com/300"),
        ));
        pre_populated.push(Card::new(
            uuid::Uuid::parse_str("b708561e-81e2-4b54-872b-e8c025d2fd22").unwrap(),
            String::from("Caitlyn Cote"),
            String::from("Sample card 2"),
            String::from("https://via.placeholder.com/300"),
        ));
        pre_populated.push(Card::new(
            uuid::Uuid::parse_str("6b87f972-7aa9-4ff9-b0dd-0d4fb819a25d").unwrap(),
            String::from("Jameson Berg"),
            String::from("Sample card 3"),
            String::from("https://via.placeholder.com/300"),
        ));

        Arc::new(Mutex::new(DbRep {
            cards: pre_populated
        }))
    }

    pub fn get_random(&self) -> &Card {
        let mut rng = rand::thread_rng();
        &self.cards[rng.gen_range(0, self.cards.len() - 1)]
    }
}