use crate::models::User;
use crate::storage::{self, StorageDriver};
use dashmap::mapref::entry::Entry::*;
use dashmap::DashMap;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserRegistry {
    pub current: Arc<DashMap<Uuid, User>>,
    storage: Arc<dyn StorageDriver<User, Item = User>>,
}

impl UserRegistry {
    pub async fn from_storage<T: 'static>(storage: Arc<T>) -> Result<UserRegistry, Box<dyn Error>>
    where
        T: StorageDriver<User, Item = User>,
    {
        let users: DashMap<Uuid, User> = DashMap::new();
        for user in storage.read_all()? {
            match users.entry(user.id) {
                Occupied(_) => {
                    error!(
                        "Detected duplicate user with id '{}' when loading UserRegistry",
                        user.id
                    );
                    return Err(Box::new(UserRegistryError::DataIntegrity(
                        UserRegistryDataIntegrityError::DuplicateId(user.id),
                    )));
                }
                Vacant(v) => v.insert(user),
            };
        }

        info!("Loaded {} users from storage", users.len());
        Ok(UserRegistry {
            current: Arc::new(users),
            storage,
        })
    }

    pub async fn add_user(&self, user: User) -> Result<(), UserRegistryError> {
        match self.current.entry(user.id) {
            Occupied(_) => Err(UserRegistryWriteError::Conflict.into()),
            Vacant(v) => {
                let user_ref = v.insert(user);
                let user = user_ref.value();
                match self.storage.write(&user.id, &user) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(UserRegistryWriteError::Storage(e).into()),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum UserRegistryError {
    DataIntegrity(UserRegistryDataIntegrityError),
    Write(UserRegistryWriteError),
}

impl std::error::Error for UserRegistryError {}

impl std::fmt::Display for UserRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error when performing an operation on UserRegistry: {:?}",
            self
        )
    }
}

impl From<UserRegistryWriteError> for UserRegistryError {
    fn from(e: UserRegistryWriteError) -> Self {
        UserRegistryError::Write(e)
    }
}

impl From<UserRegistryDataIntegrityError> for UserRegistryError {
    fn from(e: UserRegistryDataIntegrityError) -> Self {
        UserRegistryError::DataIntegrity(e)
    }
}

#[derive(Debug)]
pub enum UserRegistryDataIntegrityError {
    DuplicateId(Uuid),
}

#[derive(Debug)]
pub enum UserRegistryWriteError {
    Conflict,
    Storage(storage::Error),
}
