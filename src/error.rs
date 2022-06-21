use std::{fmt};


/// Error kind enum for Reindeer-related errors.
#[non_exhaustive]
#[derive(Debug,Clone,Copy)]
pub enum ErrorKind {
    /// Something went wrong at the `sled` level.
    SledError,
    /// Could not serialize or deserialize an entity
    SerializationError,
    /// Any kind of file system error while using the database
    IOError,
    /// An integrity constraint has been violated while trying to remove an entity from the database
    IntegrityError,
    /// An entity was not found
    NotFound,
    /// An entity was used without being registered firts in the database
    UnregisteredEntity,
}

/// Error type for `reindeer`
#[derive(Debug)]
pub struct Error {
    error_kind : ErrorKind,
    message : String,
}

impl Error {
    /// Creates a new error from an error kind and a message
    pub fn new(error_kind : ErrorKind,message : String) -> Error {
        Error {
            error_kind,
            message : message,
        }
    }
    pub fn kind(&self) -> ErrorKind {
        self.error_kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reindeer Error of type {:?} : {}",self.error_kind,&self.message)
    }
}

impl std::error::Error for Error {}

/// Type definition to simplify the use of Result everywhere in the library
pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::new(
            ErrorKind::IOError,
            source.to_string(),
        )
    }
}

impl From<sled::Error> for Error {
    fn from(source: sled::Error) -> Self {
        Error::new(
            ErrorKind::SledError,
            source.to_string(),
        )
    }
}

impl From<bincode::Error> for Error {
    fn from(source: bincode::Error) -> Self {
        Error::new(
            ErrorKind::SerializationError,
            source.to_string(),
        )
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::new(
            ErrorKind::SerializationError,
            source.to_string(),
        )
    }
}