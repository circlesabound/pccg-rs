use super::StorageDriver;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct FsStore<T> {
    dirname: PathBuf,
    _item_type: std::marker::PhantomData<T>,
}

impl<T> FsStore<T> {
    pub fn new<P: Into<PathBuf>>(dirname: P) -> Result<FsStore<T>, Box<dyn Error>> {
        Ok(FsStore {
            dirname: PathBuf::from(dirname.into()),
            _item_type: std::marker::PhantomData,
        })
    }

    fn get_filename_from_id(&self, id: &Uuid) -> PathBuf {
        self.dirname.join(format!("{}.json", id))
    }
}

impl<'de, T: DeserializeOwned + Serialize> StorageDriver<'de, T> for FsStore<T> {
    type Item = T;

    fn read(&self, id: &Uuid) -> Result<Option<T>, Box<dyn Error>> {
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

    fn write(&self, id: &Uuid, value: &T) -> Result<(), Box<dyn Error>> {
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