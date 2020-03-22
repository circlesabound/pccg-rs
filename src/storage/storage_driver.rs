use crate::storage;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

pub trait StorageDriver<Item>: Send + Sync {
    type Item: DeserializeOwned + Serialize;

    fn read(&self, id: &Uuid) -> storage::Result<Option<Item>>;
    fn read_all(&self) -> storage::Result<Vec<Item>>;
    fn write(&self, id: &Uuid, value: &Item) -> storage::Result<()>;
}
