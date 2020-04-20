use serde::Serialize;
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

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = "storage::Error";
        let variant_index;
        let variant;
        let value: String;
        match *self {
            Error::Conflict(ref e) => {
                variant_index = 0;
                variant = "Conflict";
                value = e.to_string();
            }
            Error::Hyper(ref e) => {
                variant_index = 1;
                variant = "Hyper";
                value = format!("{:?}", e);
            }
            Error::Io(ref e) => {
                variant_index = 2;
                variant = "Io";
                value = format!("{:?}", e);
            }
            Error::Jwt(ref e) => {
                variant_index = 3;
                variant = "Jwt";
                value = format!("{:?}", e);
            }
            Error::OAuth(ref e) => {
                variant_index = 4;
                variant = "OAuth";
                value = e.to_string();
            }
            Error::Other(ref e) => {
                variant_index = 5;
                variant = "Other";
                value = e.to_string();
            }
            Error::Serialization(ref e) => {
                variant_index = 6;
                variant = "Serialization";
                value = format!("{:?}", e);
            }
        };
        serializer.serialize_newtype_variant(name, variant_index, variant, &value)
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
