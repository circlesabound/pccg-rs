use crate::storage::{self, StorageDriver};
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

/// In-memory StorageDriver.
pub(crate) struct InMemoryStore<T> {
    items: DashMap<Uuid, String>,
    _item_type: std::marker::PhantomData<T>,
}

impl<T> InMemoryStore<T> {
    #[allow(dead_code)]
    pub fn new() -> storage::Result<InMemoryStore<T>> {
        Ok(InMemoryStore {
            items: DashMap::new(),
            _item_type: std::marker::PhantomData,
        })
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync> StorageDriver for InMemoryStore<T> {
    type Item = T;

    fn list_ids(&self) -> storage::Result<Vec<Uuid>> {
        Ok(self.items.iter().map(|kvp| kvp.key().clone()).collect())
    }

    fn read(&self, id: &Uuid) -> storage::Result<Option<T>> {
        match self.items.get(id) {
            Some(json_ref) => {
                let item: T = serde_json::from_str(&*json_ref)?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    fn read_all(&self) -> storage::Result<Vec<T>> {
        match self
            .items
            .iter()
            .map(|kvp| serde_json::from_str(kvp.value()))
            .collect::<Result<Vec<T>, _>>()
        {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into()),
        }
    }

    fn write(&self, id: &Uuid, value: &T) -> storage::Result<()> {
        match serde_json::to_string(value) {
            Ok(json) => {
                self.items.insert(*id, json);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
