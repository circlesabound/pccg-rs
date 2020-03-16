use crate::models::Card;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Compendium {
    pub current: RwLock<Arc<Vec<Card>>>,
}

impl Compendium {
    pub async fn from_file(filename: impl AsRef<Path>) -> Result<Compendium, Box<dyn Error>> {
        let contents = fs::read_to_string(filename)?;
        let cards: Vec<Card> = serde_json::from_str(&contents)?;
        Ok(Compendium {
            current: RwLock::new(Arc::new(cards)),
        })
    }
}
