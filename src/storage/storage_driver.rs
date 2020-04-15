use crate::storage;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

pub trait StorageDriver: Send + Sync {
    type Item: DeserializeOwned + Serialize;

    fn list_ids(&self) -> storage::Result<Vec<Uuid>>;
    fn read(&self, id: &Uuid) -> storage::Result<Option<Self::Item>>;
    fn read_all(&self) -> storage::Result<Vec<Self::Item>>;
    fn write(&self, id: &Uuid, value: &Self::Item) -> storage::Result<()>;
}
