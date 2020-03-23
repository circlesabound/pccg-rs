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
    pub async fn from_storage<T: 'static>(
        storage: Arc<T>,
    ) -> Result<UserRegistry, UserRegistryError>
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
                    return Err(UserRegistryError::DuplicateId(user.id));
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
            Occupied(_) => Err(UserRegistryError::Conflict),
            Vacant(v) => {
                let user_ref = v.insert(user);
                let user = user_ref.value();
                match self.storage.write(&user.id, &user) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(UserRegistryError::Storage(e)),
                }
            }
        }
    }

    pub async fn mutate_user_if<T, P, F>(
        &self,
        user_id: Uuid,
        precondition: P,
        f: F,
    ) -> Result<T, UserRegistryError>
    where
        P: Fn(&User) -> bool,
        F: Fn(&mut User) -> Result<T, Box<dyn Error>>,
    {
        match self.current.entry(user_id) {
            Vacant(_) => Err(UserRegistryError::NotFound),
            Occupied(mut o) => {
                // Check precondition
                let entry_ref = o.get();
                if !precondition(entry_ref) {
                    // Failed precondition, return early
                    return Err(UserRegistryError::FailedPrecondition);
                }

                // Clone the object to work on
                let mut clone = entry_ref.clone();

                // Try mutate the clone
                match f(&mut clone) {
                    Ok(ret) => {
                        // Persist to storage
                        if let Err(e) = self.storage.write(&clone.id, &clone) {
                            return Err(UserRegistryError::Storage(e));
                        }

                        // Reflect in-memory
                        o.insert(clone);
                        Ok(ret)
                    }
                    Err(e) => Err(UserRegistryError::ExternalOperation(e)),
                }
            }
        }
    }

    pub async fn mutate_user_with<T, F>(&self, user_id: Uuid, f: F) -> Result<T, UserRegistryError>
    where
        F: Fn(&mut User) -> Result<T, Box<dyn Error>>,
    {
        self.mutate_user_if(user_id, |_| true, f).await
    }
}

#[derive(Debug)]
pub enum UserRegistryError {
    Conflict,
    DuplicateId(Uuid),
    FailedPrecondition,
    ExternalOperation(Box<dyn Error>),
    NotFound,
    Storage(storage::Error),
}

// TODO? fn source
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

impl From<storage::Error> for UserRegistryError {
    fn from(e: storage::Error) -> Self {
        UserRegistryError::Storage(e)
    }
}
