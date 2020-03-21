use crate::models::Card;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
pub struct Compendium {
    pub current: RwLock<Arc<HashMap<Uuid, Card>>>,
    filename: PathBuf,
}

impl Compendium {
    pub async fn from_file(filename: PathBuf) -> Result<Compendium, Box<dyn Error>> {
        let contents = fs::read_to_string(&filename)?;
        let cards: HashMap<Uuid, Card> = serde_json::from_str(&contents)?;
        Ok(Compendium {
            current: RwLock::new(Arc::new(cards)),
            filename: filename,
        })
    }

    /// Inserts a new card into the compendium, or updates an existing card with the same ID
    /// if it already exists.
    ///
    /// * If an insert operation was performed, returns `None`
    /// * If an update operation was performed, returns `Some` with the value of the replaced card
    pub async fn upsert_card(&self, card: Card) -> Result<Option<Card>, CompendiumWriteError> {
        let id = card.id;
        {
            // Obtain an exclusive write guard
            let mut cards_mut = self.current.write().await;

            // Check whether ID already exists
            match cards_mut.get(&id) {
                None => {
                    // ID does not exist, we are inserting a new card

                    // Clone current card collection
                    let mut new_cards = (**cards_mut).clone();

                    // Add new card to the cloned collection
                    new_cards.insert(id, card);

                    // Persist to storage
                    if let Err(e) = fs::write(
                        &self.filename,
                        serde_json::to_string_pretty(&new_cards).unwrap(),
                    ) {
                        return Err(CompendiumWriteError::Io(e));
                    }

                    // Swap in-memory collection to the cloned hashmap
                    *cards_mut = Arc::new(new_cards);

                    Ok(None)
                }
                Some(old_card) => {
                    // ID already exists, we are updating an existing card

                    // Clone the original card so we can return it later
                    let old_card_clone = old_card.clone();

                    // Clone current card collection
                    let new_cards = (**cards_mut).clone();

                    // Get the card to modify in the cloned collection
                    let mut card_to_modify = new_cards.get(&id).unwrap();

                    // Modify in-place
                    std::mem::replace(&mut card_to_modify, &card);

                    // Persist to storage
                    if let Err(e) = fs::write(
                        &self.filename,
                        serde_json::to_string_pretty(&new_cards).unwrap(),
                    ) {
                        return Err(CompendiumWriteError::Io(e));
                    }

                    // Swap in-memory collection to the cloned hashmap
                    *cards_mut = Arc::new(new_cards);

                    Ok(Some(old_card_clone))
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum CompendiumWriteError {
    Io(std::io::Error),
}

impl std::fmt::Display for CompendiumWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error when performing a write operation on the compendium: {:?}",
            self
        )
    }
}

impl std::error::Error for CompendiumWriteError {}
