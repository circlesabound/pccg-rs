use crate::models::Card;
use crate::storage::{self, StorageDriver};
use dashmap::mapref::entry::Entry::*;
use dashmap::DashMap;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

pub struct Compendium {
    pub current: Arc<DashMap<Uuid, Card>>,
    storage: Arc<dyn StorageDriver<Card, Item = Card>>,
}

impl Compendium {
    pub async fn from_storage<T: 'static>(storage: Arc<T>) -> Result<Compendium, Box<dyn Error>>
    where
        T: StorageDriver<Card, Item = Card>,
    {
        let cards: DashMap<Uuid, Card> = DashMap::new();
        for card in storage.read_all()? {
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

        info!("Loaded {} cards from storage", cards.len());
        Ok(Compendium {
            current: Arc::new(cards),
            storage,
        })
    }

    /// Inserts a new card into the compendium, or updates an existing card with the same ID
    /// if it already exists.
    ///
    /// * If an insert operation was performed, returns `None`
    /// * If an update operation was performed, returns `Some` with the value of the replaced card
    pub async fn upsert_card(&self, card: Card) -> Result<Option<Card>, CompendiumWriteError> {
        match self.current.entry(card.id) {
            Occupied(mut o) => {
                let old_card = o.get().clone();
                let ret = match self.storage.write(&card.id, &card) {
                    Ok(_) => Ok(Some(old_card)),
                    Err(e) => Err(CompendiumWriteError::Storage(e)),
                };
                o.insert(card);
                ret
            }
            Vacant(v) => {
                let ret = match self.storage.write(&card.id, &card) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(CompendiumWriteError::Storage(e)),
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
    Storage(storage::Error),
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
