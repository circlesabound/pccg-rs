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
                    return Err(Box::new(CompendiumError::DataIntegrity(
                        CompendiumDataIntegrityError::DuplicateId(card.id),
                    )));
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
    pub async fn upsert_card(&self, card: Card) -> Result<Option<Card>, CompendiumError> {
        match self.current.entry(card.id) {
            Occupied(mut o) => {
                let old_card = o.get().clone();
                let ret = match self.storage.write(&card.id, &card) {
                    Ok(_) => Ok(Some(old_card)),
                    Err(e) => Err(CompendiumWriteError::Storage(e).into()),
                };
                o.insert(card);
                ret
            }
            Vacant(v) => {
                let ret = match self.storage.write(&card.id, &card) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(CompendiumWriteError::Storage(e).into()),
                };
                v.insert(card);
                ret
            }
        }
    }
}

#[derive(Debug)]
pub enum CompendiumError {
    DataIntegrity(CompendiumDataIntegrityError),
    Read(CompendiumReadError),
    Write(CompendiumWriteError),
}

impl std::error::Error for CompendiumError {}

impl std::fmt::Display for CompendiumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error when performing an operation on Compendium: {:?}",
            self
        )
    }
}

impl From<CompendiumDataIntegrityError> for CompendiumError {
    fn from(e: CompendiumDataIntegrityError) -> Self {
        CompendiumError::DataIntegrity(e)
    }
}

impl From<CompendiumReadError> for CompendiumError {
    fn from(e: CompendiumReadError) -> Self {
        CompendiumError::Read(e)
    }
}

impl From<CompendiumWriteError> for CompendiumError {
    fn from(e: CompendiumWriteError) -> Self {
        CompendiumError::Write(e)
    }
}

#[derive(Debug)]
pub enum CompendiumDataIntegrityError {
    DuplicateId(uuid::Uuid),
}

#[derive(Debug)]
pub enum CompendiumReadError {
    CardNotFound(uuid::Uuid),
}

#[derive(Debug)]
pub enum CompendiumWriteError {
    Storage(storage::Error),
}
