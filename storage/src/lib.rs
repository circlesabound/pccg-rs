#[macro_use]
extern crate log;

pub mod firestore;

mod error;
pub use error::Error;
pub use error::Result;
