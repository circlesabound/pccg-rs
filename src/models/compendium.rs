use super::card::Card;

#[derive(Default)]
pub struct Compendium {
     pub cards: Vec<Card>
}

impl Compendium {
    pub fn new() -> Compendium {
        Compendium {
            cards: Vec::new()
        }
    }
}