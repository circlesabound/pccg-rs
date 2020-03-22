use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

pub trait StorageDriver<'de, Item> {
    type Item: Deserialize<'de> + Serialize;

    fn read(&self, id: &Uuid) -> Result<Option<Item>, Box<dyn Error>>;
    fn write(&self, id: &Uuid, value: &Item) -> Result<(), Box<dyn Error>>;
}
