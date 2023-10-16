use std::str::Utf8Error;

use mcap_decoder::Visitor;

use crate::schema::Msg;

use self::cdr_reader::CdrReader;

mod cdr_reader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The array size for field {field} has maximum size {max_size}, but got {size}")]
    ArraySizeTooLarge {
        field: String,
        size: usize,
        max_size: usize,
    },
    #[error("Got utf8 error {error} when decoding data (hex representation): {data:02?}")]
    Utf8Error {
        #[source]
        error: Utf8Error,
        data: Vec<u8>,
    },
    #[error(transparent)]
    Visitor(#[from] anyhow::Error),
}

impl Error {
    fn from_visitor(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Visitor(anyhow::Error::new(e))
    }
}

pub fn decode<V>(schema: &Msg, input: &[u8], visitor: &mut V) -> Result<(), Error>
where
    V: Visitor,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    let mut reader = CdrReader::new(input);
    let mut field_path = String::with_capacity(256);
    decode_inner(schema, &mut reader, visitor, &mut field_path)
}

fn push_index(s: &mut String, i: usize) {
    const INDEX_TABLE: [&str; 16] = [
        "[0]", "[1]", "[2]", "[3]", "[4]", "[5]", "[6]", "[7]", "[8]", "[9]", "[10]", "[11]",
        "[12]", "[13]", "[14]", "[15]",
    ];

    if i < 16 {
        s.push_str(unsafe { INDEX_TABLE.get_unchecked(i) });
    } else {
        s.push('[');
        s.push_str(&i.to_string());
        s.push(']');
    }
}

fn decode_inner<V>(
    schema: &Msg,
    reader: &mut CdrReader,
    visitor: &mut V,
    field_path: &mut String,
) -> Result<(), Error>
where
    V: Visitor,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    let old_field_path_len = field_path.len();
    for field in &schema.fields {
        field_path.push_str(&field.name);
        let size = match field.repitition {
            crate::schema::Repitition::Single => 1,
            crate::schema::Repitition::Fixed(v) => v,
            crate::schema::Repitition::Unbounded => reader.u32() as usize,
            crate::schema::Repitition::Bounded(max_size) => {
                let size = reader.u32() as usize;
                if size > max_size {
                    return Err(Error::ArraySizeTooLarge {
                        field: field_path.clone(),
                        size,
                        max_size,
                    });
                }
                size
            }
        };

        let new_field_path_len = field_path.len();
        #[allow(clippy::needless_range_loop)]
        for i in 0..size {
            if field.repitition != crate::schema::Repitition::Single {
                push_index(field_path, i);
            }
            match &field.ty {
                crate::schema::Type::Bool => visitor
                    .visit_bool(field_path, reader.bool())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I8 => visitor
                    .visit_i8(field_path, reader.i8())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U8 => visitor
                    .visit_u8(field_path, reader.u8())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I16 => visitor
                    .visit_i16(field_path, reader.i16())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U16 => visitor
                    .visit_u16(field_path, reader.u16())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I32 => visitor
                    .visit_i32(field_path, reader.i32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U32 => visitor
                    .visit_u32(field_path, reader.u32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I64 => visitor
                    .visit_i64(field_path, reader.i64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U64 => visitor
                    .visit_u64(field_path, reader.u64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::F32 => visitor
                    .visit_f32(field_path, reader.f32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::F64 => visitor
                    .visit_f64(field_path, reader.f64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::Str => visitor
                    .visit_str(field_path, reader.str()?)
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::Msg(msg_schema) => {
                    decode_inner(msg_schema, reader, visitor, field_path)?;
                }
            }
            if field.repitition != crate::schema::Repitition::Single {
                field_path.truncate(new_field_path_len);
            }
        }
        field_path.truncate(old_field_path_len);
    }
    Ok(())
}
