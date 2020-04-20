use crate::models::{CompendiumError, UserRegistryError};
use crate::storage;
use std::error;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    source: Option<ErrorSource>,
}

#[derive(Debug)]
pub enum ErrorCode {
    CardNotFound,
    CompendiumEmpty,
    DailyAlreadyClaimed,
    Other,
    Storage,
    UserNotFound,
}

impl Error {
    pub fn new(code: ErrorCode, source: Option<ErrorSource>) -> Error {
        Error { code, source }
    }

    pub fn classify(&self) -> ErrorCategory {
        match self.code {
            ErrorCode::CardNotFound | ErrorCode::UserNotFound => ErrorCategory::BadRequest,
            ErrorCode::CompendiumEmpty | ErrorCode::Other | ErrorCode::Storage => {
                ErrorCategory::Internal
            }
            ErrorCode::DailyAlreadyClaimed => ErrorCategory::FailedPrecondition,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("code={:?}, source={:?}", self.code, self.source))
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.source {
            Some(ref e) => Some(e),
            None => None,
        }
    }
}

impl From<CompendiumError> for Error {
    fn from(e: CompendiumError) -> Self {
        match e {
            CompendiumError::NotFound => Error::new(ErrorCode::CardNotFound, Some(e.into())),
            CompendiumError::Storage(_) => Error::new(ErrorCode::Storage, Some(e.into())),
            _ => Error::new(ErrorCode::Other, Some(e.into())),
        }
    }
}

impl From<UserRegistryError> for Error {
    fn from(e: UserRegistryError) -> Self {
        match e {
            UserRegistryError::NotFound => Error::new(ErrorCode::UserNotFound, Some(e.into())),
            UserRegistryError::Storage(_) => Error::new(ErrorCode::Storage, Some(e.into())),
            _ => Error::new(ErrorCode::Other, Some(e.into())),
        }
    }
}

impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Self {
        Error::new(ErrorCode::Storage, Some(e.into()))
    }
}

#[derive(Debug)]
pub enum ErrorSource {
    Compendium(CompendiumError),
    Storage(storage::Error),
    UserRegistry(UserRegistryError),
}

impl Display for ErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorSource::Compendium(ref e) => Display::fmt(e, f),
            ErrorSource::UserRegistry(ref e) => Display::fmt(e, f),
            ErrorSource::Storage(ref e) => Display::fmt(e, f),
        }
    }
}

impl error::Error for ErrorSource {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ErrorSource::Compendium(e) => Some(e),
            ErrorSource::UserRegistry(e) => Some(e),
            ErrorSource::Storage(e) => Some(e),
        }
    }
}

impl From<CompendiumError> for ErrorSource {
    fn from(e: CompendiumError) -> Self {
        ErrorSource::Compendium(e)
    }
}

impl From<UserRegistryError> for ErrorSource {
    fn from(e: UserRegistryError) -> Self {
        ErrorSource::UserRegistry(e)
    }
}

impl From<storage::Error> for ErrorSource {
    fn from(e: storage::Error) -> Self {
        ErrorSource::Storage(e)
    }
}

pub enum ErrorCategory {
    BadRequest,
    FailedPrecondition,
    Internal,
}
