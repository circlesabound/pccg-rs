use pccg_rs_storage as storage;
use serde::Serialize;
use std::error;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub source: Option<ErrorSource>,
}

#[derive(Debug, Serialize)]
pub enum ErrorCode {
    CardNotFound,
    CharacterNotFound,
    CharacterPreoccupied,
    CompendiumEmpty,
    DailyAlreadyClaimed,
    DrawStageEmpty,
    DrawStagePopulated,
    IdMismatch,
    InsufficientFunds,
    JobNotFound,
    Other,
    StorageGeneric,
    StorageTransaction,
    UserNotFound,
}

impl Error {
    pub fn new(code: ErrorCode, source: Option<ErrorSource>) -> Error {
        Error { code, source }
    }

    pub fn classify(&self) -> ErrorCategory {
        match self.code {
            ErrorCode::CardNotFound
            | ErrorCode::CharacterNotFound
            | ErrorCode::IdMismatch
            | ErrorCode::JobNotFound
            | ErrorCode::UserNotFound => ErrorCategory::BadRequest,
            ErrorCode::CompendiumEmpty | ErrorCode::Other | ErrorCode::StorageGeneric => {
                ErrorCategory::Internal
            }
            ErrorCode::CharacterPreoccupied
            | ErrorCode::DailyAlreadyClaimed
            | ErrorCode::DrawStageEmpty
            | ErrorCode::DrawStagePopulated
            | ErrorCode::InsufficientFunds => ErrorCategory::FailedPrecondition,
            ErrorCode::StorageTransaction => ErrorCategory::InternalRetryable,
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

impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Self {
        let code = match e {
            storage::Error::Transaction(_) => ErrorCode::StorageTransaction,
            _ => ErrorCode::StorageGeneric,
        };
        Error::new(code, Some(e.into()))
    }
}

#[derive(Debug, Serialize)]
pub enum ErrorSource {
    Storage(storage::Error),
}

impl Display for ErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorSource::Storage(ref e) => Display::fmt(e, f),
        }
    }
}

impl error::Error for ErrorSource {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ErrorSource::Storage(e) => Some(e),
        }
    }
}

impl From<storage::Error> for ErrorSource {
    fn from(e: storage::Error) -> Self {
        ErrorSource::Storage(e)
    }
}

#[derive(Debug, Serialize)]
pub enum ErrorCategory {
    BadRequest,
    FailedPrecondition,
    Internal,
    InternalRetryable,
}
