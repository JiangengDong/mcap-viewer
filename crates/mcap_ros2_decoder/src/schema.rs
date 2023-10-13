use std::{fmt::Debug, sync::Arc};

mod parse;

pub use parse::{get, parse, Error};

#[derive(Hash, PartialEq, Eq, Clone)]
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

impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => f.write_fmt(format_args!("bool")),
            Type::I8 => f.write_fmt(format_args!("i8")),
            Type::I16 => f.write_fmt(format_args!("i16")),
            Type::I32 => f.write_fmt(format_args!("i32")),
            Type::I64 => f.write_fmt(format_args!("i64")),
            Type::U8 => f.write_fmt(format_args!("u8")),
            Type::U16 => f.write_fmt(format_args!("u16")),
            Type::U32 => f.write_fmt(format_args!("u32")),
            Type::U64 => f.write_fmt(format_args!("u64")),
            Type::F32 => f.write_fmt(format_args!("f32")),
            Type::F64 => f.write_fmt(format_args!("f64")),
            Type::Str => f.write_fmt(format_args!("str")),
            Type::Msg(msg) => msg.fmt(f),
        }
    }
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

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Msg {
    pub fields: Vec<Field>,
    // TODO: add constants
}

impl Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("Msg");
        for field in &self.fields {
            builder.field(&field.name, &FieldTypeDebug(&field.ty, &field.repitition));
        }
        builder.finish()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub repitition: Repitition,
}

struct FieldTypeDebug<'a>(&'a Type, &'a Repitition);

impl Debug for FieldTypeDebug<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            Repitition::Single => self.0.fmt(f),
            Repitition::Bounded(n) => {
                f.write_str("[")?;
                self.0.fmt(f)?;
                f.write_fmt(format_args!("; <={n}]"))
            }
            Repitition::Fixed(n) => {
                f.write_str("[")?;
                self.0.fmt(f)?;
                f.write_fmt(format_args!("; {n}]"))
            }
            Repitition::Unbounded => {
                f.write_str("[")?;
                self.0.fmt(f)?;
                f.write_str("]")
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Repitition {
    Single,
    Bounded(usize),
    Fixed(usize),
    Unbounded,
}
