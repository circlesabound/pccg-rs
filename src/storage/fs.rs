use super::StorageDriver;
use crate::storage;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub struct FsStore<T> {
    dirname: PathBuf,
    _item_type: std::marker::PhantomData<T>,
}

impl<T> FsStore<T> {
    pub fn new<P: Into<PathBuf>>(dirname: P) -> storage::Result<FsStore<T>> {
        Ok(FsStore {
            dirname: PathBuf::from(dirname.into()),
            _item_type: std::marker::PhantomData,
        })
    }

    fn get_filename_from_id(&self, id: &Uuid) -> PathBuf {
        self.dirname.join(format!("{}.json", id))
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync> StorageDriver<T> for FsStore<T> {
    type Item = T;

    fn read(&self, id: &Uuid) -> storage::Result<Option<T>> {
        let path = self.get_filename_from_id(id);
        match path.exists() {
            false => Ok(None),
            true => {
                let contents = fs::read_to_string(path)?;
                let item: T = serde_json::from_str(&contents)?;
                Ok(Some(item))
            }
        }
    }

    fn read_all(&self) -> storage::Result<Vec<T>> {
        let mut ret: Vec<T> = vec![];
        for entry in fs::read_dir(&self.dirname)? {
            let filename = entry?.path();
            let contents = fs::read_to_string(filename)?;
            let item: T = serde_json::from_str(&contents)?;
            ret.push(item);
        }
        Ok(ret)
    }

    fn write(&self, id: &Uuid, value: &T) -> storage::Result<()> {
        let json = serde_json::to_string_pretty(&value)?;
        Ok(fs::write(self.get_filename_from_id(id), json)?)
    }
}

mod test {
    use super::*;

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct MockItem {
        id: Uuid,
        number: i32,
    }

    #[test]
    fn can_write() {
        let fs: FsStore<MockItem> = FsStore::new(std::env::temp_dir()).unwrap();
        let item = MockItem {
            id: Uuid::new_v4(),
            number: 345,
        };

        assert!(fs.write(&item.id, &item).is_ok());
    }

    #[test]
    fn can_read_after_write() {
        let fs: FsStore<MockItem> = FsStore::new(std::env::temp_dir()).unwrap();
        let item = MockItem {
            id: Uuid::new_v4(),
            number: 543,
        };

        assert!(fs.write(&item.id, &item).is_ok());
        assert_eq!(fs.read(&item.id).unwrap().unwrap(), item);
    }
}