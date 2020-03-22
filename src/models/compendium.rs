use crate::models::Card;
use dashmap::mapref::entry::Entry::*;
use dashmap::DashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Default)]
pub struct Compendium {
    pub current: Arc<DashMap<Uuid, Card>>,
    dirname: PathBuf,
}

impl Compendium {
    pub async fn from_fs(dirname: PathBuf) -> Result<Compendium, Box<dyn Error>> {
        let cards: DashMap<Uuid, Card> = DashMap::new();
        for entry in fs::read_dir(&dirname)? {
            let filename = entry?.path();
            let contents = fs::read_to_string(filename)?;
            let card: Card = serde_json::from_str(&contents)?;

            match cards.entry(card.id) {
                Occupied(_) => {
                    error!(
                        "Detected duplicate card with id '{}' when loading Compendium",
                        card.id
                    );
                    return Err(Box::new(CompendiumDataIntegrityError::DuplicateId(card.id)));
                }
                Vacant(v) => v.insert(card),
            };
        }

        info!("Loaded {} cards from filesystem", cards.len());
        Ok(Compendium {
            current: Arc::new(cards),
            dirname,
        })
    }

    /// Inserts a new card into the compendium, or updates an existing card with the same ID
    /// if it already exists.
    ///
    /// * If an insert operation was performed, returns `None`
    /// * If an update operation was performed, returns `Some` with the value of the replaced card
    pub async fn upsert_card(&self, card: Card) -> Result<Option<Card>, CompendiumWriteError> {
        let json = serde_json::to_string_pretty(&card).unwrap();
        match self.current.entry(card.id) {
            Occupied(mut o) => {
                let old_card = o.get().clone();
                let ret = match fs::write(&self.dirname.join(format!("{}.json", card.id)), json) {
                    Ok(_) => Ok(Some(old_card)),
                    Err(e) => Err(CompendiumWriteError::Io(e)),
                };
                o.insert(card);
                ret
            }
            Vacant(v) => {
                let ret = match fs::write(&self.dirname.join(format!("{}.json", card.id)), json) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(CompendiumWriteError::Io(e)),
                };
                v.insert(card);
                ret
            }
        }
    }
}

#[derive(Debug)]
pub enum CompendiumDataIntegrityError {
    DuplicateId(uuid::Uuid),
}

impl std::fmt::Display for CompendiumDataIntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Data integrity issue with the compendium: {:?}", self)
    }
}

impl std::error::Error for CompendiumDataIntegrityError {}

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
