#![warn(missing_docs)]
//! This crate deals with having to store potentially large amounts of
//! information in a human readable and editable format (short json files).
//! Databases are excluded because of the human readable part, and so are
//! excruciatingly long files.
//!
//! In order to manage that information inside a program we propose the `Table<T>`
//! structure. It can manage information of type `<T>` that can be serialized and
//! deserialized by [serde](https://serde.rs/). For that purpose the traits and
//! derive macros are reexported. (So that there is no need to explicitly depend
//! on serde to use this crate)

use serde::de::DeserializeOwned;
pub use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::{HashMap, Iter, Keys, Values, ValuesMut},
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File},
    io::{prelude::*, SeekFrom},
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
};

mod table_error;
pub use table_error::{TableBuilderError, TableError};

mod aux;
pub use aux::{ContentPolicy, ExtensionPolicy, RWPolicy, TableBuilder, TableMetadata, WriteType};

/// The structure that's stored in the internal `hash_map`. It contains a file and
/// the content of the file. You can only access the information and not the file
#[derive(Debug)]
pub struct TableElement<T> {
    /// The file in which the element is read
    file: File,
    /// The element that you actually want stored/read
    pub info: T,
}

/// Main structure of this crate. Holds the information from the table. It
/// reads all at once, so huge tables will be slow and memory intensive
#[derive(Debug)]
pub struct Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// A string instead of a ReadDir, because it's easier to modify and write
    /// (new files from). ReadDir doesn't implement clone or copy so it's just
    /// annoying to deal with)
    dir: PathBuf,
    content: HashMap<String, TableElement<T>>,
    metadata: TableMetadata,
    is_modified: bool,
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new table
    ///
    /// # Errors
    /// 1. There was already a table in that directory
    /// 2. Couldn't create a path to the table
    pub fn new<Q: AsRef<Path>>(dir: Q, metadata: TableMetadata) -> Result<Self, TableBuilderError> {
        if metadata.rw_policy == RWPolicy::ReadOnly {
            return Err(TableBuilderError::CreateWithoutWriteError);
        }
        match fs::metadata(&dir) {
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {}
                _ => return Err(e.into()),
            },
            Ok(_) => return Err(TableBuilderError::TableAlreadyExistsError),
        };
        fs::create_dir_all(&dir)?;
        Ok(Table {
            dir: dir.as_ref().to_path_buf(),
            content: HashMap::new(),
            metadata,
            is_modified: false,
        })
    }

    /// Generate a `TableBuilder` to open or load a table
    pub fn builder<Q: AsRef<Path>>(dir: Q) -> TableBuilder<T> {
        TableBuilder::new(dir)
    }

    /// Load an exiting table, it can also be loaded through a builder
    ///
    /// # Errors
    /// 1. Whenever there's a file in the directory which you don't have
    /// permission to read, or is not a file or directory
    /// 2. Couldn't open a file with the required permissions
    /// 3. There is a deserialization error and the policy was `PromoteSerdeErrors`
    /// 4. There was a non .json file in a table with the `OnlyJsonFiles` extension policy
    ///
    /// # Panics
    /// If somehow you have a file without a name, or with an name that is not utf-8
    /// compatible
    pub fn load<Q: AsRef<Path>>(
        dir: Q,
        metadata: Option<TableMetadata>,
    ) -> Result<Self, TableError> {
        let metadata = metadata.unwrap_or_default();
        let mut content = HashMap::<String, TableElement<T>>::new();
        fs::read_dir(&dir)?.try_for_each(|dir_entry| {
            let path = dir_entry?.path();
            let jstr = OsStr::new("json");
            if path.is_file() && Some(jstr) == path.extension() {
                // we know it has a name, because it's a file therefore the unwraps
                let name = path.file_name().unwrap().to_str().unwrap();
                let (name, _) = name.rsplit_once('.').unwrap();
                let file = match metadata.rw_policy {
                    RWPolicy::ReadOnly => File::open(&path),
                    RWPolicy::Write(_) => File::options().read(true).write(true).open(&path),
                };
                match file {
                    Ok(fi) => match serde_json::from_reader(&fi) {
                        Ok(info) => {
                            content.insert(name.to_string(), TableElement { file: fi, info });
                            Ok(())
                        }
                        Err(serde_error) => match metadata.content_policy {
                            ContentPolicy::IgnoreSerdeErrors => Ok(()),
                            ContentPolicy::PromoteSerdeErrors => Err(serde_error.into()),
                        },
                    },
                    Err(e) => Err(TableError::FileOpError(e)),
                }
            } else {
                match metadata.extension_policy {
                    ExtensionPolicy::OnlyJsonFiles => Err(TableError::JsonError),
                    ExtensionPolicy::IgnoreNonJson => Ok(()),
                }
            }
        })?;
        Ok(Table {
            metadata,
            dir: dir.as_ref().to_path_buf(),
            content,
            is_modified: false,
        })
    }

    /// It appends an element to the table and opens a file `{dir}/{fname}.json`
    /// when the table has been created with write policy.
    /// It doesn't write back the file, it only opens it, creating it.
    ///
    /// # Errors
    /// 1. If you don't have permission to write
    /// 2. If you cant create a new file
    /// 3. If an element without a file already exists with the same name
    /// can only happen if while executing your aplication you deleted a file
    pub fn push(&mut self, fname: &str, info_elem: T) -> Result<(), TableError> {
        self.mod_permissions()?;
        let mut f_elem_name = self.dir.clone();
        f_elem_name.push(format!("{}.json", fname));
        let f_elem = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&f_elem_name)?;
        let element = TableElement {
            file: f_elem,
            info: info_elem,
        };
        if let Some(e) = self.content.insert(fname.into(), element) {
            drop(e.file);
            fs::remove_file(f_elem_name)?;
            return Err(TableError::PushError(fname.into()));
        }
        self.is_modified = true;
        Ok(())
    }

    /// It removes an element to the table and deletes the file `{dir}/{fname}.json`
    /// If you dont have permission to write
    ///
    /// # Errors
    /// 1. If you don't have permission to write
    /// 2. You try to delete a non existing element
    /// 2. If you cant delete the file
    pub fn pop(&mut self, fname: &str) -> Result<(), TableError> {
        self.mod_permissions()?;
        self.is_modified = true;
        match self.content.remove(fname) {
            Some(_) => {
                let mut f_elem = self.dir.clone();
                f_elem.push(format!("{}.json", fname));
                fs::remove_file(f_elem).map_err(|err| err.into())
            }
            None => Err(TableError::PopError(fname.to_string())),
        }
    }

    /// Do not delete completely, but eliminate from current Table content and
    /// make associated file non json `{dir}/{fname}.json_soft_delete` or
    /// `{dir}/{alt_name}.json_soft_delete`
    ///
    /// # Errors
    /// 1. If you don't have permission to write
    /// 2. The element doesn't exist
    /// 2. If you can't create the `.json_soft_delete` file
    /// 3. If you have serialization problems
    /// 4, If you cant `pop` the element
    pub fn soft_pop(&mut self, fname: &str, alt_name: Option<&str>) -> Result<(), TableError> {
        self.mod_permissions()?;
        match self.content.get(fname) {
            Some(content) => {
                let mut f_elem = self.dir.clone();
                f_elem.push(format!("{}.json_soft_delete", alt_name.unwrap_or(fname)));
                let file = File::options().write(true).create_new(true).open(f_elem)?;
                serde_json::to_writer_pretty(file, &content.info)?;
                self.pop(fname)?;
                Ok(())
            }
            None => {
                Err(TableError::PopError(fname.to_string()))
            }
        }
    }

    /// Returns true when a mutable reference has been taken in the past or when
    /// some item(s) has been pushed popped or appended. If after an operation
    /// there is a `write_back` it will return false again.
    ///
    /// Thanks to the borrow checker you can't try check if is something is modified
    /// while a there is a mutable reference around. So keep that in mind
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Get the names of the files aka the table's primary keys
    pub fn get_table_keys(&self) -> Keys<String, TableElement<T>> {
        self.content.keys()
    }

    /// An iterator over names and elements
    pub fn iter(&self) -> Iter<String, TableElement<T>> {
        self.content.iter()
    }

    /// Get the values stored in the table
    pub fn get_table_content(&self) -> Values<String, TableElement<T>> {
        self.content.values()
    }

    /// Get the values stored in the table in a convenient mutable reference
    pub fn get_mut_table_content(&mut self) -> ValuesMut<String, TableElement<T>> {
        self.is_modified = true;
        self.content.values_mut()
    }

    /// Get an individual element of the table by key
    pub fn get_element(&self, entry_name: &str) -> Option<&TableElement<T>> {
        self.content.get(entry_name)
    }

    /// Get an individual mutable element of the table by key
    pub fn get_mut_element(&mut self, entry_name: &str) -> Option<&mut TableElement<T>> {
        self.is_modified = true;
        self.content.get_mut(entry_name)
    }

    /// Write the changes in the corresponding files,
    ///
    /// # Errors
    /// 1. If you don't have permission to write
    /// 2. There are problems with serialization
    pub fn write_back(&mut self) -> Result<(), TableError> {
        self.mod_permissions()?;
        if self.is_modified() {
            self.is_modified = false;
            for table_element in self.content.values_mut() {
                let file = &mut table_element.file;
                file.set_len(0)?;
                file.seek(SeekFrom::Start(0))?;
                serde_json::to_writer_pretty(file, &table_element.info)?;
            }
        }
        Ok(())
    }

    /// the number of elements in the table
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Whether the table is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Table has been declared with the ability to modify the file system
    fn mod_permissions(&self) -> Result<(), TableError> {
        match self.metadata.rw_policy {
            RWPolicy::Write(_) => Ok(()),
            RWPolicy::ReadOnly => Err(TableError::NoWritePolicyError),
        }
    }

    /// Table has been declared with the ability to modify the file system
    pub fn has_mod_permissions(&self) -> bool {
        self.mod_permissions().is_ok()
    }
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// Append an array of items when they are Clone but not Copy
    ///
    /// # Errors
    /// 1. Whenever the length of names and elements is not the same
    /// 2. Whenever there is an error with an individual `push`
    pub fn append_clone<Q: AsRef<str>>(
        &mut self,
        fnames: &[Q],
        elements: &[T],
    ) -> Result<(), TableError> {
        if elements.len() != fnames.len() {
            return Err(TableError::AppendLengthError);
        }

        for (element, fname) in elements.iter().zip(fnames) {
            self.push(fname.as_ref(), element.clone())?;
        }

        Ok(())
    }

    /// Rename a element
    ///
    /// # Errors
    /// 1. If you don't have permission to write
    /// 2. If you try to rename a non existing element
    /// 3. If you have trouble pushing the element with the new name
    pub fn rename(&mut self, old_name: &str, new_name: &str) -> Result<(), TableError> {
        self.mod_permissions()?;
        self.is_modified = true;
        let name_string = old_name.to_string();
        let info = self
            .get_element(old_name)
            .ok_or(TableError::PopError(name_string))?
            .info
            .clone();
        self.pop(old_name)?;
        self.push(new_name, info)?;
        Ok(())
    }
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Copy,
{
    /// Append an array of items when they are Copy
    ///
    /// # Errors
    /// 1. Whenever the length of names and elements is not the same
    /// 2. Whenever there is an error with an individual `push`
    pub fn append<Q: AsRef<str>>(
        &mut self,
        fnames: &[Q],
        elements: &[T],
    ) -> Result<(), TableError> {
        if elements.len() != fnames.len() {
            return Err(TableError::AppendLengthError);
        }

        for (&element, fname) in elements.iter().zip(fnames) {
            self.push(fname.as_ref(), element)?;
        }

        Ok(())
    }
}

impl<T> Index<&str> for Table<T>
where
    T: Serialize + DeserializeOwned,
{
    type Output = TableElement<T>;
    fn index(&self, index: &str) -> &Self::Output {
        &self.content[index]
    }
}

impl<T> IndexMut<&str> for Table<T>
where
    T: Serialize + DeserializeOwned,
{
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        self.is_modified = true;
        self.content.get_mut(index).unwrap()
    }
}
impl<T> Drop for Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Writes back in case the write back is set to automatic
    ///
    /// # Panics
    /// - When there are problems with the write back mainly when
    ///     - There are problems with file handles
    ///     - There are problems with serialization
    fn drop(&mut self) {
        if RWPolicy::Write(WriteType::Automatic) == self.metadata.rw_policy {
            self.write_back().unwrap();
        }
    }
}
