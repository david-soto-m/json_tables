//! All tests are integration tests, because this way I get a feel for the
//! ergonomics of the crate

#[cfg(test)]
use json_tables::{Deserialize, Serialize, Table, TableBuilderError, TableError};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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
    match Table::<ExampleStruct>::builder("tests/doesnt_exist").load() {
        Err(TableError::FileOpError(_)) => assert!(true),
        _ => assert!(false),
    }
}
#[test]
fn table_has_no_json_files() {
    let table = Table::<ExampleStruct>::builder("tests/").load().unwrap();
    assert_eq!(table.len(), 0)
}

#[test]
fn create_automatic_table() {
    {
        let mut table = Table::<ExampleStruct>::builder("tests/create_table")
            .build()
            .unwrap();
        table.push(ExampleStruct::default(), "hola").unwrap();
    }
    // table is forcibly dropped here, and auto-written
    assert_eq!(
        1,
        Table::<ExampleStruct>::builder("tests/create_table")
            .set_read_only()
            .load()
            .unwrap()
            .len()
    );
    std::fs::remove_file("tests/create_table/hola.json").unwrap();
    std::fs::remove_dir("tests/create_table").unwrap();
}

#[test]
fn create_manual_table() {
    {
        let mut table = Table::<ExampleStruct>::builder("tests/create_table_2")
            .set_manual_write()
            .build()
            .unwrap();
        table.push(ExampleStruct::default(), "hola").unwrap();
    }
    // Table is forcibly dropped here, not written, but the directory is created
    assert_eq!(
        0,
        Table::<ExampleStruct>::builder("tests/create_table_2")
            .set_read_only()
            .set_ignore_de_errors()
            .load()
            .unwrap()
            .len()
    );
    std::fs::remove_file("tests/create_table_2/hola.json").unwrap();
    std::fs::remove_dir("tests/create_table_2").unwrap();
    {
        let mut table = Table::<ExampleStruct>::builder("tests/create_table_2")
            .set_manual_write()
            .build()
            .unwrap();
        table.push(ExampleStruct::default(), "hola").unwrap();
        table.write_back().unwrap();
    }
    assert_eq!(
        1,
        Table::<ExampleStruct>::builder("tests/create_table_2")
            .set_read_only()
            .set_ignore_de_errors()
            .load()
            .unwrap()
            .len()
    );
    std::fs::remove_file("tests/create_table_2/hola.json").unwrap();
    std::fs::remove_dir("tests/create_table_2").unwrap();
}
#[test]
fn creation_errors() {
    match Table::<ExampleStruct>::builder("tests/create_table_3")
        .set_read_only()
        .build()
    {
        Err(TableBuilderError::CreateWithoutWriteError) => assert!(true),
        _ => assert!(false),
    }
    std::fs::create_dir("tests/nowrite").unwrap();
    let mut perm = std::fs::metadata("tests/nowrite").unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions("tests/nowrite", perm).unwrap();
    match Table::<ExampleStruct>::builder("tests/nowrite/table").build() {
        Err(TableBuilderError::DirCreateError(_)) => assert!(true),
        _ => assert!(false),
    }
    std::fs::remove_dir_all("tests/nowrite").unwrap();
    match Table::<ExampleStruct>::builder("tests/normal").build() {
        Err(TableBuilderError::TableAlreadyExistsError) => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn load_with_all_perms() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .set_auto_write()
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .set_manual_write()
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .set_read_only()
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
}

#[test]
fn load_with_non_json(){
    match Table::<ExampleStruct>::builder("tests/extension")
        .set_read_non_json_is_error()
        .load(){
        Err(TableError::JsonError) => assert!(true),
        _ => assert!(false),
    };
    let table = Table::<ExampleStruct>::builder("tests/extension")
        .set_read_only_json()
        .load()
        .unwrap();
    assert_eq!(table.len(), 0);
    let table = Table::<ExampleStruct>::builder("tests/extension")
        .set_read_all_files()
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
}

#[test]
fn load_mixed_tables_json(){
    match Table::<ExampleStruct>::builder("tests/mixed")
        .set_read_all_files()
        .load(){
        Err(TableError::SerdeError(_)) => assert!(true),
        _ =>assert!(false),
    };
    let table = Table::<ExampleStruct>::builder("tests/mixed")
        .set_read_only_json()
        .set_ignore_de_errors()
        .load()
        .unwrap();
    assert_eq!(table.len(), 0);
    let table = Table::<ExampleStruct>::builder("tests/mixed")
        .set_read_all_files()
        .set_ignore_de_errors()
        .load()
        .unwrap();
    assert_eq!(table.len(), 3);
}

#[test]
fn error_on_dir_in_json_only_table(){
    match Table::<ExampleStruct>::builder("tests/only_json")
        .set_read_non_json_is_error()
        .load(){
        Err(TableError::JsonError) => assert!(true),
        _ => assert!(false),
    };
}

/* A function to help initialize the tables
#[test]
fn creation() {
    let mut table = Table::<ExampleStruct>::builder("tests/normal_table").build().unwrap();
    let mut a = vec![];
    let mut b = vec![];
    for i in 0..5{
        a.push(ExampleStruct{int: i as i32,..Default::default()});
        b.push(i.to_string());
    }
    table.append_clone(&a, &b).unwrap();
}
*/
