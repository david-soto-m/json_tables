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
//! users of the crate don't have to depend on rayon in order to use it.

use rayon::{collections::hash_map::Iter, iter::Map as ry_Map, prelude::*};
use serde::de::DeserializeOwned;
pub use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::{HashMap, Keys, Values, ValuesMut},
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File},
    io::{prelude::*, SeekFrom},
    ops::Index,
    path::Path,
};

mod table_error;
pub use table_error::{TableBuilderError, TableError};

mod aux;
pub use aux::{ContentPolicy, ExtensionPolicy, RWPolicy, TableBuilder, TableMetadata, WriteType};

/// The structure that's stored in the internal hash_map. It contains a file and
/// the content of the file. You can only access the information and not the
/// file
#[derive(Debug)]
pub struct TableElement<T> {
    /// The file in which the element is read
    file: Option<File>,
    /// The element that you actually want stored/read
    pub info: T,
}

#[derive(Debug)]
pub struct Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// A string instead of a ReadDir, because it's easier to modify and write
    /// (new files from). ReadDir doesn't implement clone or copy so it's just
    /// annoying to deal with)
    dir: String,
    content: HashMap<String, TableElement<T>>,
    metadata: TableMetadata,
    is_modified: bool,
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// create a new table
    pub fn new(dir: &str, metadata: TableMetadata) -> Result<Self, TableBuilderError> {
        if metadata.rw_policy == RWPolicy::ReadOnly {
            return Err(TableBuilderError::CreateWithoutWriteError);
        }
        fs::create_dir_all(dir)?;

        Ok(Table {
            dir: dir.into(),
            content: HashMap::new(),
            metadata,
            is_modified: false,
        })
    }
    pub fn builder() -> TableBuilder {
        TableBuilder::default()
    }
    pub fn load(dir: &str, metadata: Option<TableMetadata>) -> Result<Self, TableError> {
        let metadata = metadata.unwrap_or_default();
        let files: Vec<Result<(String, File), TableError>> = fs::read_dir(dir)?
            .par_bridge()
            .map(|dir_entry| {
                let path = dir_entry.unwrap().path();
                let jstr = OsStr::new("json");
                let check = {
                    path.is_file()
                        && match metadata.extension_policy {
                            ExtensionPolicy::IgnoreExtensions => true,
                            _ => Some(jstr) == path.extension(),
                        }
                };
                if check {
                    // we know it has a name, because it's a file therefore the unwraps
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();
                    let name = name.split('.').next().unwrap().to_string();
                    let file = match metadata.rw_policy {
                        RWPolicy::ReadOnly => File::open(&path),
                        RWPolicy::Write(_) => File::options().read(true).write(true).open(&path),
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
                Ok((name, file)) => match serde_json::from_reader(&file) {
                    Ok(info) => {
                        let file = match metadata.rw_policy {
                            RWPolicy::ReadOnly => None,
                            RWPolicy::Write(_) => Some(file),
                        };
                        content.insert(name, TableElement { file, info });
                    }
                    Err(serde_error) => match metadata.content_policy {
                        ContentPolicy::IgnoreSerdeErrors => {}
                        ContentPolicy::PromoteSerdeErrors => return Err(serde_error.into()),
                    },
                },
                Err(TableError::JsonError) =>{
                    if metadata.extension_policy == ExtensionPolicy::OnlyJsonFiles{
                        return Err(TableError::JsonError)
                    }
                },
                Err(e) => return Err(e),
            };
        }
        Ok(Table {
            metadata,
            dir: dir.into(),
            content,
            is_modified: false,
        })
    }

    pub fn reload(self) -> Result<Self, TableError> {
        Table::load(&self.dir, Some(self.metadata))
    }

    /// It appends an element to the table and opens a file "{dir}/{fname}.json"
    /// when the table has been created with write policy.
    /// It doesn't write back the file, it only opens it.
    pub fn push(&mut self, info_elem: T, fname: &str) -> Result<(), TableError> {
        match self.metadata.rw_policy {
            RWPolicy::Write(_) => {}
            RWPolicy::ReadOnly => return Err(TableError::NoWritePermError),
        };
        let f_elem = format!("{}/{fname}.json", self.dir);
        let f_elem = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(Path::new(&f_elem))?;
        let element = TableElement {
            file: Some(f_elem),
            info: info_elem,
        };
        self.content.insert(fname.into(), element);
        self.is_modified = true;
        Ok(())
    }

    /// Returns true when a mutable reference has been taken in the past or when
    /// some item(s) has been pushed/appended. If after an operation there is a writeback
    /// it will return false again.
    ///
    /// Thanks to the borrow checker you can't try check if is something is modified
    /// while a there is a mutable reference around. So keep that in mind
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
        match self.metadata.rw_policy {
            RWPolicy::Write(_) => {}
            RWPolicy::ReadOnly => return Err(TableError::NoWritePermError),
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


impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub fn append_clone(&mut self, elements: &[T], fnames: &[&str])-> Result<(), TableError>{
        if elements.len()!= fnames.len(){
            return Err(TableError::AppendLengthError);
        }

        for (element, &fname) in elements.iter().zip(fnames){
            self.push(element.clone(), fname)?
        }

        Ok(())
    }
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Copy,
{
    pub fn append(&mut self, elements: &[T], fnames: &[&str])-> Result<(), TableError>{
        if elements.len()!= fnames.len(){
            return Err(TableError::AppendLengthError);
        }

        for (&element, &fname) in elements.iter().zip(fnames){
            self.push(element, fname)?
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

impl<T> Drop for Table<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Writes back in case the write back is set to automatic
    /// ## Panics
    /// - When there are problems with the write back
    ///     - There are problems with file handles
    ///     - There are problems with serialization
    fn drop(&mut self) {
        if RWPolicy::Write(WriteType::Automatic) == self.metadata.rw_policy {
            self.write_back().unwrap();
        }
    }
}



#[cfg(test)]
mod tests {
    use crate::{Deserialize, Serialize, Table, TableError};

    #[derive(Serialize, Deserialize, Default)]
    struct ExampleStruct {
        int: i32,
        float: f64,
        array: [u32; 4],
        tuple: (i32, f64),
        string: String,
        vector: Vec<f64>,
    }
    #[test]
    fn err_load_table_doesnt_exist() {
        match Table::<ExampleStruct>::load("tests/doesnt_exist", None) {
            Err(TableError::FileOpError(_)) => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn table_has_no_json_files() {
        let table = Table::<ExampleStruct>::load("tests/", None).unwrap();
        assert_eq!(table.len(), 0)
    }
}
