use serde_json;
use std::fmt;

#[derive(Debug)]
pub enum TableError {
    NoWritePermError,
    JsonError,
    FileOpError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableError::FileOpError(e) => write!(f, "{e}"),
            TableError::JsonError => write!(f, "Non Json file in Table"),
            TableError::SerdeError(e) => write!(f, "{e}"),
            TableError::NoWritePermError => write!(
                f,
                "You are trying to modify a Table without permission to do so"
            ),
            // _ => write!(f, "Weird error with a Table"),
        }
    }
}

impl std::error::Error for TableError {}

impl From<std::io::Error> for TableError {
    fn from(e: std::io::Error) -> Self {
        TableError::FileOpError(e)
    }
}

impl From<serde_json::Error> for TableError {
    fn from(e: serde_json::Error) -> Self {
        TableError::SerdeError(e)
    }
}

#[derive(Debug)]
pub enum TableBuilderError {
    DirCreateError(std::io::Error),
}

impl fmt::Display for TableBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirCreateError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TableBuilderError {}

impl From<std::io::Error> for TableBuilderError {
    fn from(e: std::io::Error) -> Self {
        TableBuilderError::DirCreateError(e)
    }
}
