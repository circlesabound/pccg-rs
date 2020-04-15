use crate::storage::{self, StorageDriver};
use futures::executor::block_on;
use hyper::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio::task;
use uuid::Uuid;

pub(crate) struct Firestore<T> {
    _item_type: std::marker::PhantomData<T>,
}

impl<T> Firestore<T> {
    pub fn new() -> storage::Result<Firestore<T>> {
        Ok(Firestore {
            _item_type: std::marker::PhantomData,
        })
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync> StorageDriver for Firestore<T> {
    type Item = T;

    fn list_ids(&self) -> storage::Result<Vec<Uuid>> {
        unimplemented!()
    }

    fn read(&self, id: &Uuid) -> storage::Result<Option<T>> {
        unimplemented!()
    }

    fn read_all(&self) -> storage::Result<Vec<T>> {
        unimplemented!()
    }

    fn write(&self, id: &Uuid, value: &T) -> storage::Result<()> {
        unimplemented!()
    }
}
