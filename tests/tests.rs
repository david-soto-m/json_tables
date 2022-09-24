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

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
struct SimplifiedStruct {
    int: i32,
    float: f64,
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
        table.push("hola", ExampleStruct::default()).unwrap();
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
        table.push("hola", ExampleStruct::default()).unwrap();
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
        table.push("hola", ExampleStruct::default()).unwrap();
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
fn load_with_non_json() {
    match Table::<ExampleStruct>::builder("tests/extension")
        .set_read_non_json_is_error()
        .load()
    {
        Err(TableError::JsonError) => assert!(true),
        _ => assert!(false),
    };
    let table = Table::<ExampleStruct>::builder("tests/extension")
        .load()
        .unwrap();
    assert_eq!(table.len(), 0);
}

#[test]
fn load_dotted() {
    assert!(Table::<ExampleStruct>::builder("tests/dotted")
        .load()
        .is_ok())
}

#[test]
fn load_mixed_tables_json() {
    match Table::<ExampleStruct>::builder("tests/mixed").load() {
        Err(TableError::SerdeError(_)) => assert!(true),
        _ => assert!(false),
    };
    let table = Table::<ExampleStruct>::builder("tests/mixed")
        .set_ignore_de_errors()
        .load()
        .unwrap();
    assert_eq!(table.len(), 0);
    let table = Table::<SimplifiedStruct>::builder("tests/mixed")
        .load()
        .unwrap();
    assert_eq!(table.len(), 2);
}

#[test]
fn error_on_dir_in_json_only_table() {
    match Table::<ExampleStruct>::builder("tests/only_json")
        .set_read_non_json_is_error()
        .load()
    {
        Err(TableError::JsonError) => assert!(true),
        _ => assert!(false),
    };
}

#[test]
fn keys() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .load()
        .unwrap();
    let mut a: Vec<i32> = (0..5).collect();
    assert!(table.get_table_keys().all(|x| {
        let l = a.len();
        a = a
            .clone()
            .into_iter()
            .filter(|&y| y.to_string() != *x)
            .collect();
        a.len() == l - 1
    }));
}

#[test]
fn values() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .load()
        .unwrap();
    let mut a: Vec<i32> = (0..5).collect();
    assert!(table.get_table_content().all(|x| {
        let l = a.len();
        a = a.clone().into_iter().filter(|&y| y != x.info.int).collect();
        a.len() == l - 1
    }));
}

#[test]
fn iter() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .load()
        .unwrap();
    assert!(table
        .iter()
        .all(|(string, element)| { *string == element.info.int.to_string() }));
}

#[test]
fn element() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .load()
        .unwrap();
    assert_eq!(table["1"].info.int, 1);
    assert_eq!(table.get_element("1").unwrap().info.int, 1);
    assert!(table.get_element("100").is_none());
}

#[test]
fn is_empty() {
    let table = Table::<ExampleStruct>::builder("tests/normal")
        .load()
        .unwrap();
    assert!(!table.is_empty());
    let table = Table::<ExampleStruct>::builder("tests/mixed")
        .set_ignore_de_errors()
        .load()
        .unwrap();
    assert!(table.is_empty());
}

#[test]
fn values_mut() {
    let constant = 0.005;
    assert!(Table::<ExampleStruct>::builder("tests/normal_mut_1")
        .load()
        .unwrap()
        .get_table_content()
        .all(|x| x.info.float == 0.0));
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_1")
        .set_manual_write()
        .load()
        .unwrap();
    table.get_mut_table_content().for_each(|ele| {
        ele.info.float = ele.info.int as f64 * constant;
    });
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    assert!(Table::<ExampleStruct>::builder("tests/normal_mut_1")
        .load()
        .unwrap()
        .get_table_content()
        .all(|x| x.info.float == x.info.int as f64 * constant));
    Table::<ExampleStruct>::builder("tests/normal_mut_1")
        .load()
        .unwrap()
        .get_mut_table_content()
        .for_each(|ele| {
            ele.info.float = 0.0;
        });
}

#[test]
fn element_mut() {
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_2")
        .load()
        .unwrap();
    assert!(table.get_table_content().all(|x| x.info.float == 0.0));
    assert!(!table.is_modified());
    table["0"].info.float = 1.0;
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    table.get_mut_element("1").unwrap().info.float = 0.05;
    assert!(table.is_modified());
    table.write_back().unwrap();
    let table = Table::<ExampleStruct>::builder("tests/normal_mut_2")
        .load()
        .unwrap();
    assert_eq!(table["0"].info.float, 1.0);
    assert_eq!(table["1"].info.float, 0.05);
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_2")
        .load()
        .unwrap();
    table["0"].info.float = 0.0;
    table.get_mut_element("1").unwrap().info.float = 0.0;
}

#[test]
fn push_pop() {
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_3")
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
    table.push("100", ExampleStruct::default()).unwrap();
    assert_eq!(table.len(), 6);
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    table.pop("100").unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    assert_eq!(table.len(), 5);
}

#[test]
fn rename() {
    {
        let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_5")
            .load()
            .unwrap();
        assert_eq!(table.len(), 5);
        assert_eq!(table["4"].info.int, 4);
        table.rename("4", "renamed").unwrap();
        assert_eq!(table["renamed"].info.int, 4);
        assert!(table.get_element("4").is_none())
    }
    {
        let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_5")
            .load()
            .unwrap();
        assert_eq!(table.len(), 5);
        assert_eq!(table["renamed"].info.int, 4);
        table.rename("renamed", "4").unwrap();
        assert_eq!(table["4"].info.int, 4);
        assert!(table.get_element("renamed").is_none())
    }
}

#[test]
fn push_pop_error() {
    let mut table = Table::<ExampleStruct>::builder("tests/mixed_2")
        .load()
        .unwrap();
    match table.push("0", ExampleStruct::default()) {
        Err(TableError::FileOpError(_)) => assert!(true),
        _ => assert!(false),
    };
    match table.pop("100") {
        Err(TableError::PopError(string)) => assert_eq!(string, "100".to_string()),
        _ => assert!(false),
    }
}

#[test]
fn append_remove() {
    let (names, elements): (Vec<String>, Vec<SimplifiedStruct>) = (5..10)
        .map(|el| {
            (
                el.to_string(),
                SimplifiedStruct {
                    int: el,
                    float: el as f64 * 10e-1,
                },
            )
        })
        .unzip();
    let mut table = Table::<SimplifiedStruct>::builder("tests/simplified")
        .load()
        .unwrap();

    table.append(names.as_slice(), &elements).unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    let mut table = Table::<SimplifiedStruct>::builder("tests/simplified")
        .load()
        .unwrap();
    assert_eq!(table.len(), 10);
    table.remove(names.as_slice()).unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
}

#[test]
fn append_clone() {
    let (names, elements): (Vec<String>, Vec<ExampleStruct>) = (5..10)
        .map(|el| {
            (
                el.to_string(),
                ExampleStruct {
                    int: el,
                    float: el as f64 * 10e-1,
                    ..Default::default()
                },
            )
        })
        .unzip();
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_4")
        .load()
        .unwrap();

    table.append_clone(names.as_slice(), &elements).unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    let mut table = Table::<ExampleStruct>::builder("tests/normal_mut_4")
        .load()
        .unwrap();
    assert_eq!(table.len(), 10);
    table.remove(names.as_slice()).unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
}

#[test]
fn append_error() {
    let (mut names, elements): (Vec<String>, Vec<SimplifiedStruct>) = (5..10)
        .map(|el| {
            (
                el.to_string(),
                SimplifiedStruct {
                    int: el,
                    float: el as f64 * 10e-1,
                },
            )
        })
        .unzip();
    names.push("hola".into());
    let mut table = Table::<SimplifiedStruct>::builder("tests/simplified_1")
        .load()
        .unwrap();
    match table.append(names.as_slice(), &elements) {
        Err(TableError::AppendLengthError) => assert!(true),
        _ => assert!(false),
    };
}

#[test]
fn no_write_perm_error() {
    let mut table = Table::<ExampleStruct>::builder("tests/dotted")
        .set_read_only()
        .load()
        .unwrap();
    table["this.file.has.dots"].info.int = 42;
    match table.write_back() {
        Err(TableError::NoWritePolicyError) => assert!(true),
        e => {
            println!("{e:?}");
            assert!(false)
        }
    }
}

#[test]
fn soft_del() {
    {
        let mut table = Table::<SimplifiedStruct>::builder("tests/delete")
            .load()
            .unwrap();
        assert_eq!(table.len(), 5);
        table.soft_pop("0", "0").unwrap();
        assert!(table.is_modified());
        table.write_back().unwrap();
        assert!(!table.is_modified());
    }
    let table = Table::<SimplifiedStruct>::builder("tests/delete")
        .load()
        .unwrap();
    assert_eq!(table.len(), 4);
    std::fs::rename("tests/delete/0.json_soft_delete", "tests/delete/0.json").unwrap();
    let table = Table::<SimplifiedStruct>::builder("tests/delete")
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
}

#[test]
fn soft_del_err() {
    let mut table = Table::<SimplifiedStruct>::builder("tests/delete_2")
        .load()
        .unwrap();
    assert_eq!(table.len(), 5);
    table.soft_pop("0", "0").unwrap();
    assert!(table.is_modified());
    table.write_back().unwrap();
    assert!(!table.is_modified());
    match table.soft_pop("0", "0") {
        Err(TableError::PopError(e)) => assert_eq!(e, "0"),
        _ => assert!(false),
    };
    match table.soft_pop("1", "0") {
        Err(TableError::FileOpError(_)) => assert!(true),
        e => {
            println!("{e:?}");
            assert!(false)
        }
    };
    std::fs::rename("tests/delete_2/0.json_soft_delete", "tests/delete_2/0.json").unwrap();
}
