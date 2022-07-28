// #![warn(missing_docs)]

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
//!
//! Some parts of the crate rely heavily on parallel iterators, provided by
//! [rayon](https://docs.rs/rayon/latest/rayon/). In order to use those the type
//! `<T>` must be marked with Sync. The type `IterOut` is exported so that
//! users of the crate dont have to depend on rayon in order to use it.
//!
//! This crate uses

use rayon::{collections::hash_map::Iter, iter::Map as ry_Map, prelude::*};
use serde::de::DeserializeOwned;
pub use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::hash_map::{HashMap, Keys, Values, ValuesMut},
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File},
    io::{prelude::*, SeekFrom},
    ops::Index,
};

mod table_error;
pub use table_error::{TableBuilderError, TableError};

mod aux;
pub use aux::{Permissions, WriteType};

#[derive(Debug)]
pub struct TableElement<T> {
    file: Option<File>,
    pub info: T,
}

#[derive(Debug)]
pub struct TableBuilder {
    dir: String,
    permissions: Permissions,
}

impl TableBuilder {
    pub fn dir(mut self, dir: &str) -> TableBuilder {
        self.dir = dir.into();
        self
    }
    pub fn set_manual_write(mut self) -> TableBuilder {
        self.permissions = Permissions::Write(WriteType::Manual);
        self
    }
    pub fn set_auto_write(mut self) -> TableBuilder {
        self.permissions = Permissions::Write(WriteType::Automatic);
        self
    }

    pub fn build<T>(self) -> Result<Table<T>, TableBuilderError>
    where
        T: Serialize + DeserializeOwned + Sync,
    {
        fs::create_dir_all(&self.dir)?;

        Ok(Table {
            dir: self.dir,
            content: HashMap::new(),
            permissions: self.permissions,
            is_modified: false,
        })
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self {
            dir: "".into(),
            permissions: Permissions::default(),
        }
    }
}

#[derive(Debug)]
pub struct Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// A string instead of a ReadDir, because it's easier to modify and write new files from.
    /// (ReadDir doesnt implement clone or copy so it's just annoying to deal with)
    dir: String,
    content: HashMap<String, TableElement<T>>,
    permissions: Permissions,
    is_modified: bool,
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn builder() -> TableBuilder {
        TableBuilder::default()
    }
    pub fn load(dir: &str, permissions: Permissions) -> Result<Table<T>, TableError> {
        let files: Vec<Result<(String, File), TableError>> = fs::read_dir(dir)?
            .par_bridge()
            .map(|dir_entry| {
                let path = dir_entry.unwrap().path();
                let jstr = OsStr::new("json");
                if Some(jstr) == path.extension() {
                    // we know it has a name, because it ends in .json
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();
                    let name = name.split('.').next().unwrap().to_string();
                    let file = match permissions {
                        Permissions::ReadOnly => File::open(&path),
                        Permissions::Write(_) => File::options().read(true).write(true).open(&path),
                    };
                    match file {
                        Ok(fi) => Ok((name, fi)),
                        Err(e) => Err(TableError::FileOpError(e)),
                    }
                } else {
                    Err(TableError::JsonError)
                }
            })
            .collect();
        let mut content = HashMap::<String, TableElement<T>>::new();
        for element in files.into_iter() {
            match element {
                Ok((name, file)) => {
                    let info = serde_json::from_reader(&file)?;
                    let file = match permissions {
                        Permissions::ReadOnly => None,
                        Permissions::Write(_) => Some(file),
                    };
                    content.insert(name, TableElement { file, info });
                }
                Err(TableError::JsonError) => {}
                Err(e) => return Err(e),
            };
        }
        Ok(Table {
            permissions,
            dir: dir.into(),
            content,
            is_modified: false,
        })
    }

    pub fn reload(self) -> Result<Table<T>, TableError> {
        Table::load(&self.dir, self.permissions)
    }

    /// It appends an element to the table and opens a file "fname.json" with
    /// read write permissions.
    /// It doesn't write back the file, it only opens it.
    /// The open file occurs sync
    pub fn push(&mut self, info_elem: T, fname: &str) -> Result<(), TableError> {
        match self.permissions {
            Permissions::Write(_) => {}
            Permissions::ReadOnly => return Err(TableError::NoWritePermError),
        };
        let mut f_elem = self.dir.clone();
        f_elem.push('/');
        f_elem.push_str(fname);
        f_elem.push_str(".json");
        let f_elem = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&f_elem)?;
        let element = TableElement {
            file: Some(f_elem),
            info: info_elem,
        };
        self.content.insert(fname.into(), element);
        self.is_modified = true;
        Ok(())
    }

    /// Returns true when a mutable reference has been taken or when some item
    /// has been appended. If after an operation there is a writeback it will
    /// return false.
    /// Thanks to the borrow checker you can't try check if is something is modified
    /// while a there is a mutable reference around.
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn get_table_keys(&self) -> Keys<String, TableElement<T>> {
        self.content.keys()
    }

    pub fn get_table_content(&self) -> Values<String, TableElement<T>> {
        self.content.values()
    }

    pub fn get_mut_table_content(&mut self) -> ValuesMut<String, TableElement<T>> {
        self.is_modified = true;
        self.content.values_mut()
    }

    pub fn get_element(&self, entry_name: &str) -> &TableElement<T> {
        &self.content[entry_name]
    }

    pub fn get_mut_element(&mut self, entry_name: &str) -> Option<&mut TableElement<T>> {
        self.is_modified = true;
        self.content.get_mut(entry_name)
    }

    pub fn write_back(&mut self) -> Result<(), TableError> {
        match self.permissions {
            Permissions::Write(_) => {}
            Permissions::ReadOnly => return Err(TableError::NoWritePermError),
        };
        if self.is_modified() {
            self.is_modified = false;
            for table_element in self.content.values_mut() {
                // all will be some
                let file = &mut table_element.file.as_ref().unwrap();
                file.set_len(0)?;
                file.seek(SeekFrom::Start(0))?;
                serde_json::to_writer_pretty(*file, &table_element.info)?
            }
        }
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}


/// The output type of the `get_info_iter` function that maps the different
/// elements of a HashMap onto a reference of your type <T>
pub type IterOut<'r, T> =
    ry_Map<Iter<'r, String, TableElement<T>>, fn((&'r String, &'r TableElement<T>)) -> &'r T>;

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Sync,
{
    pub fn get_info_iter(&self) -> IterOut<T> {
        self.content.par_iter().map(|(_, element)| &element.info)
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

impl<T> Drop for Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Writes back in case the write back is set to automatic
    /// ## Panics
    /// - When there is problems in the write back
    fn drop(&mut self) {
        if Permissions::default() == self.permissions {
            self.write_back().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Permissions, Table, TableError};
    #[test]
    fn err_table_doesnt_exist() {
        match Table::<BuildAux>::load("tests/db/two_elems", Permissions::ReadOnly) {
            Err(TableError::FileOpError(_)) => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn table_has_no_json_files() {
        let table = Table::<BuildAux>::load("tests/db", Permissions::ReadOnly).unwrap();
        assert_eq!(table.len(), 0)
    }
    #[test]
    fn table_loads_correctly() {
        let table: Table<BuildAux> =
            Table::load("tests/db/three_elems", Permissions::ReadOnly).unwrap();
        assert_eq!(table.len(), 3)
    }
    #[test]
    fn table_processes_with_mix_of_files() {
        let table = Table::<BuildAux>::load("tests/db/mixed_json", Permissions::ReadOnly).unwrap();
        assert_eq!(table.len(), 2)
    }
    #[test]
    fn table_processes_with_mix_of_files_wb() {
        let table = Table::<BuildAux>::load("tests/db/mixed_json", Permissions::default()).unwrap();
        assert_eq!(table.len(), 2)
    }

    #[test]
    fn doc() {
        let mut table = Table::<BuildAux>::load("tests/db/append", Permissions::default()).unwrap();
        assert_eq!(table.is_modified(), false);
        assert_eq!(table.len(), 3);
        table.push(BuildAux::default(), "deff").unwrap();
        assert_eq!(table.is_modified(), true);
        assert_eq!(table.len(), 4);
        table.write_back().unwrap();
        assert_eq!(table.is_modified(), false);
        assert_eq!(table.len(), 4);
        let element = table.get_mut_table_content().next().unwrap();
        element.info.file_types.push("add element".into());
        //Drops element
        assert_eq!(table.is_modified(), true);
        assert_eq!(table.len(), 4);
        table.write_back().unwrap();
        assert_eq!(table.is_modified(), false);
        assert_eq!(table.len(), 4);
        std::fs::remove_file("tests/db/append/deff.json").unwrap();
    }
}
