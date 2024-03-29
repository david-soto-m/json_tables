use std::fmt;

/// Errors during the management of a table
#[derive(Debug)]
pub enum TableError {
    /// Trying to write without setting a policy
    NoWritePolicyError,
    /// A file doesn't end with .json and you have an OnlyJson policy for that
    /// table
    JsonError,
    /// Something went wrong with an operation
    FileOpError(std::io::Error),
    /// There was an error while trying to serialize/deserialize
    SerdeError(serde_json::Error),
    /// There was an error trying to append
    AppendLengthError,
    /// Trying to push to existing key
    PushError(String),
    /// Tried to pop a non existant key,
    PopError(String),
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileOpError(e) => write!(f, "{e}"),
            Self::JsonError => write!(f, "Non Json file in Table"),
            Self::SerdeError(e) => write!(f, "{e}"),
            Self::NoWritePolicyError => {
                write!(
                    f,
                    "You are trying to modify a Table without permission to do so"
                )
            }
            Self::AppendLengthError => {
                write!(f, "Not equal lengths of file names and elements")
            }
            Self::PushError(s) => {
                write!(
                    f,
                    "File {s}.json already exists in table and can't be pushed into the table"
                )
            }
            Self::PopError(s) => {
                write!(f, "File {s}.json doesn't exist in the table")
            } // _ => write!(f, "Weird error with a Table"),
        }
    }
}

impl std::error::Error for TableError {}

impl From<std::io::Error> for TableError {
    fn from(e: std::io::Error) -> Self {
        Self::FileOpError(e)
    }
}

impl From<serde_json::Error> for TableError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

/// Error trying to create a new table
#[derive(Debug)]
pub enum TableBuilderError {
    /// Couldn't create the directory for the table
    DirCreateError(std::io::Error),
    /// Trying to create without a write policy
    CreateWithoutWriteError,
    /// Trying to create a table that already exists
    TableAlreadyExistsError,
}

impl fmt::Display for TableBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirCreateError(e) => write!(f, "{e}"),
            Self::CreateWithoutWriteError => {
                write!(f, "Tried to create a table without write policy")
            }
            Self::TableAlreadyExistsError => {
                write!(f, "The table already exists, try loading it instead")
            }
        }
    }
}

impl std::error::Error for TableBuilderError {}

impl From<std::io::Error> for TableBuilderError {
    fn from(e: std::io::Error) -> Self {
        TableBuilderError::DirCreateError(e)
    }
}
