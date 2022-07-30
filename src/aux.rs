use crate::{Table, TableBuilderError, TableError};
pub use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, marker::PhantomData};

/// Whether the write operation is performed on drop or not
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum WriteType {
    /// You have to manually write back into the files. If the table structure
    /// is dropped without writing back no changes will be applied.
    Manual,
    /// The table is written back when the table is dropped. All changes up to
    /// that point will be saved
    #[default]
    Automatic,
}

/// Weather you can write or not with a table.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RWPolicy {
    /// No write can or will occur, it will send back an error when write
    /// operations occur
    ReadOnly,
    /// You have the ability to write back to the files the changes you make
    Write(WriteType),
}

impl Default for RWPolicy {
    fn default() -> Self {
        RWPolicy::Write(WriteType::default())
    }
}

/// How to treat the file extensions
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum ExtensionPolicy {
    /// Give an error if a non json file or a directory is found in the table's
    /// directory
    OnlyJsonFiles,
    #[default]
    /// Ignore non json files or directories
    IgnoreNonJson,
}

/// Whether to give an error when a file can't be deserialized to the intended
/// structure
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum ContentPolicy {
    /// Ignore deserialization fails
    IgnoreSerdeErrors,
    /// Promote the deserialization fails to fails in the loading of the table
    #[default]
    PromoteSerdeErrors,
}

/// A compilation of all the policies of a Table
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct TableMetadata {
    /// The read write policy for the table
    pub rw_policy: RWPolicy,
    /// the extension policy for the table
    pub extension_policy: ExtensionPolicy,
    /// The content policy for the table
    pub content_policy: ContentPolicy,
}

/// A builder that creates new tables and opens existing tables.
/// The default TableBuilder configures the table to ignore write back
/// automatically, ignore non json files, and report errors when
/// deserialization cant be completed
#[derive(Debug)]
pub struct TableBuilder<T> {
    data: PhantomData<T>,
    dir: String,
    metadata: TableMetadata,
}

impl<T> TableBuilder<T> {
    /// Create a new tableBuilder from a directory
    pub fn new(dir: &str) -> Self {
        Self {
            data: PhantomData,
            dir: dir.into(),
            metadata: TableMetadata {
                rw_policy: RWPolicy::Write(WriteType::Automatic),
                extension_policy: ExtensionPolicy::IgnoreNonJson,
                content_policy: ContentPolicy::PromoteSerdeErrors,
            },
        }
    }

    /// Set the writeback to be manual
    pub fn set_manual_write(mut self) -> Self {
        self.metadata.rw_policy = RWPolicy::Write(WriteType::Manual);
        self
    }

    /// Set the writeback to be automatic on drops
    pub fn set_auto_write(mut self) -> Self {
        self.metadata.rw_policy = RWPolicy::Write(WriteType::Automatic);
        self
    }

    /// Set the table so that it won't be written over
    pub fn set_read_only(mut self) -> Self {
        self.metadata.rw_policy = RWPolicy::ReadOnly;
        self
    }

    /// Set the table so that non json files in the table's directory provoke
    /// an error on loading
    pub fn set_read_non_json_is_error(mut self) -> Self {
        self.metadata.extension_policy = ExtensionPolicy::OnlyJsonFiles;
        self
    }

    /// When a read file does **not** contain a valid json for the type T just
    /// ignore it
    pub fn set_ignore_de_errors(mut self) -> Self {
        self.metadata.content_policy = ContentPolicy::IgnoreSerdeErrors;
        self
    }

    /// Load an existing table
    pub fn load(self) -> Result<Table<T>, TableError>
    where
        T: Serialize + DeserializeOwned + Sync,
    {
        Table::load(&self.dir, Some(self.metadata))
    }

    /// Create a new table. In order to do so a write policy must be in place
    pub fn build(self) -> Result<Table<T>, TableBuilderError>
    where
        T: Serialize + DeserializeOwned + Sync,
    {
        Table::new(&self.dir, self.metadata)
    }
}

impl<T> Default for TableBuilder<T> {
    fn default() -> Self {
        Self {
            data: PhantomData,
            dir: "".into(),
            metadata: TableMetadata {
                rw_policy: RWPolicy::Write(WriteType::Automatic),
                extension_policy: ExtensionPolicy::IgnoreNonJson,
                content_policy: ContentPolicy::PromoteSerdeErrors,
            },
        }
    }
}
