use crate::models::User;
use dashmap::mapref::entry::Entry::*;
use dashmap::DashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserRegistry {
    pub current: Arc<DashMap<Uuid, User>>,
    dirname: PathBuf,
}

impl UserRegistry {
    pub async fn from_fs(dirname: PathBuf) -> Result<UserRegistry, Box<dyn Error>> {
        let users: DashMap<Uuid, User> = DashMap::new();
        for entry in fs::read_dir(&dirname)? {
            let filename = entry?.path();
            let contents = fs::read_to_string(filename)?;
            let user: User = serde_json::from_str(&contents)?;

            match users.entry(user.id) {
                Occupied(_) => {
                    error!(
                        "Detected duplicate user with id '{}' when loading UserRegistry",
                        user.id
                    );
                    return Err(Box::new(UserRegistryWriteError::Conflict));
                }
                Vacant(v) => v.insert(user),
            };
        }

        info!("Loaded {} users from filesystem", users.len());
        Ok(UserRegistry {
            current: Arc::new(users),
            dirname,
        })
    }

    pub async fn add_user(&self, user: User) -> Result<(), UserRegistryWriteError> {
        let id = user.id;
        match self.current.entry(id) {
            Occupied(_) => Err(UserRegistryWriteError::Conflict),
            Vacant(v) => {
                let json = serde_json::to_string_pretty(&user).unwrap();
                v.insert(user);
                match fs::write(&self.dirname.join(format!("{}.json", id)), json) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(UserRegistryWriteError::Io(e)),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum UserRegistryReadError {
    //
}

impl std::fmt::Display for UserRegistryReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error when performing a read operation on UserRegistry: {:?}",
            self
        )
    }
}

impl std::error::Error for UserRegistryReadError {}

#[derive(Debug)]
pub enum UserRegistryWriteError {
    Conflict,
    Io(std::io::Error),
}

impl std::fmt::Display for UserRegistryWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error when performing a write operation on UserRegistry: {:?}",
            self
        )
    }
}

impl std::error::Error for UserRegistryWriteError {}
