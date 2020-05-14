pub mod read;
mod schema;
pub mod write;

use std::convert::From;

#[derive(Debug)]
pub enum Error {
    DBNotFound,
    InvalidIdentifier(String),
    EmptyColumns,
    InvalidTimeRange,
    SQLite(rusqlite::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::CannotOpen,
                    extended_code: _,
                },
                _,
            ) => Error::DBNotFound,
            _ => Error::SQLite(err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
