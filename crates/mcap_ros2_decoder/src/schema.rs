use std::sync::Arc;

mod parse;

pub use parse::{get, parse, Error};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Type {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Str,
    Msg(Arc<Msg>),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct MsgId {
    pub package: String,
    pub name: String,
}

impl MsgId {
    pub fn new<S1, S2>(package: S1, name: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            package: package.into(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Msg {
    pub fields: Vec<Field>,
    // TODO: add constants
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub repitition: Repitition,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Repitition {
    Single,
    Bounded(usize),
    Fixed(usize),
    Unbounded,
}
