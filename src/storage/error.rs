use std::error;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Conflict(String),
    Io(std::io::Error),
    Hyper(hyper::Error),
    Jwt(jsonwebtoken::errors::Error),
    OAuth(String),
    Other(String),
    Serialization(serde_json::error::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Conflict(ref e) => Display::fmt(e, f),
            Error::Hyper(ref e) => Display::fmt(e, f),
            Error::Io(ref e) => Display::fmt(e, f),
            Error::Jwt(ref e) => Display::fmt(e, f),
            Error::OAuth(ref e) => Display::fmt(e, f),
            Error::Other(ref e) => Display::fmt(e, f),
            Error::Serialization(ref e) => Display::fmt(e, f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Conflict(_) => None,
            Error::Hyper(ref e) => Some(e),
            Error::Io(ref e) => Some(e),
            Error::Jwt(ref e) => Some(e),
            Error::OAuth(_) => None,
            Error::Other(_) => None,
            Error::Serialization(ref e) => Some(e),
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(value: hyper::Error) -> Self {
        Error::Hyper(value)
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        Error::Jwt(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Serialization(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}
