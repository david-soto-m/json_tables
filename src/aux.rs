use std::fmt::Debug;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum WriteType {
    Manual,
    #[default]
    Automatic,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Permissions {
    ReadOnly,
    Write(WriteType),
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions::Write(WriteType::default())
    }
}
