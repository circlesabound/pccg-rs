use crate::storage;
use serde::Serialize;
use std::error;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    source: Option<ErrorSource>,
}

#[derive(Debug, Serialize)]
pub enum ErrorCode {
    CardNotFound,
    CompendiumEmpty,
    DailyAlreadyClaimed,
    DrawStageEmpty,
    DrawStagePopulated,
    IdMismatch,
    InsufficientFunds,
    JobNotFound,
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
            ErrorCode::CardNotFound
            | ErrorCode::IdMismatch
            | ErrorCode::JobNotFound
            | ErrorCode::UserNotFound => ErrorCategory::BadRequest,
            ErrorCode::CompendiumEmpty | ErrorCode::Other | ErrorCode::Storage => {
                ErrorCategory::Internal
            }
            ErrorCode::DailyAlreadyClaimed
            | ErrorCode::DrawStageEmpty
            | ErrorCode::DrawStagePopulated
            | ErrorCode::InsufficientFunds => ErrorCategory::FailedPrecondition,
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
        Error::new(ErrorCode::Storage, Some(e.into()))
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
}
