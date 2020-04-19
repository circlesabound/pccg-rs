use crate::models::Card;
use crate::storage::{self, StorageDriver};
use dashmap::mapref::entry::Entry::*;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;

pub struct Compendium {
    pub current: Arc<DashMap<Uuid, Card>>,
    storage: Arc<dyn StorageDriver<Item = Card>>,
}

impl Compendium {
    pub async fn from_storage<T: 'static>(storage: Arc<T>) -> Result<Compendium, CompendiumError>
    where
        T: StorageDriver<Item = Card>,
    {
        let cards: DashMap<Uuid, Card> = DashMap::new();
        for card in task::block_in_place(|| storage.read_all())? {
            match cards.entry(card.id) {
                Occupied(_) => {
                    error!(
                        "Detected duplicate card with id '{}' when loading Compendium",
                        card.id
                    );
                    return Err(CompendiumError::DuplicateId(card.id));
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
                let ret = match task::block_in_place(|| self.storage.write(&card.id, &card)) {
                    Ok(_) => Ok(Some(old_card)),
                    Err(e) => Err(e.into()),
                };
                o.insert(card);
                ret
            }
            Vacant(v) => {
                let ret = match task::block_in_place(|| self.storage.write(&card.id, &card)) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e.into()),
                };
                v.insert(card);
                ret
            }
        }
    }
}

#[derive(Debug)]
pub enum CompendiumError {
    DuplicateId(Uuid),
    NotFound,
    Storage(storage::Error),
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

impl From<storage::Error> for CompendiumError {
    fn from(e: storage::Error) -> Self {
        CompendiumError::Storage(e)
    }
}
