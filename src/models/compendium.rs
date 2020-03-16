use crate::models::Card;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Compendium {
    pub current: RwLock<Arc<Vec<Card>>>,
    filename: PathBuf,
}

impl Compendium {
    pub async fn from_file(filename: PathBuf) -> Result<Compendium, Box<dyn Error>> {
        let contents = fs::read_to_string(&filename)?;
        let cards: Vec<Card> = serde_json::from_str(&contents)?;
        Ok(Compendium {
            current: RwLock::new(Arc::new(cards)),
            filename: filename.clone(),
        })
    }

    pub async fn add_card(&self, card: Card) -> Result<uuid::Uuid, CompendiumWriteError> {
        let id = card.id;
        {
            // Obtain an exclusive write guard
            let mut cards_mut = self.current.write().await;

            // Check for ID conflict
            if cards_mut.iter().any(|c| c.id == id) {
                return Err(CompendiumWriteError::Conflict);
            }

            // Clone current card list
            let mut new_cards = (**cards_mut).clone();
            new_cards.push(card);

            // Persist to storage
            if let Err(e) = fs::write(
                &self.filename,
                serde_json::to_string_pretty(&new_cards).unwrap(),
            ) {
                return Err(CompendiumWriteError::Io(e));
            }

            // Update in memory
            *cards_mut = Arc::new(new_cards);
        }
        Ok(id)
    }
}

#[derive(Debug)]
pub enum CompendiumWriteError {
    Conflict,
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
