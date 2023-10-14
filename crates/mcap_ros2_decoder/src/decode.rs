use std::str::Utf8Error;

use mcap_decoder::{FieldId, Visitor};

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
    let mut field_id = FieldId::new();
    decode_inner(schema, &mut reader, visitor, &mut field_id)
}

fn decode_inner<'a, V>(
    schema: &'a Msg,
    reader: &mut CdrReader,
    visitor: &mut V,
    field_id: &mut FieldId<'a>,
) -> Result<(), Error>
where
    V: Visitor,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    for field in &schema.fields {
        field_id.push_member(&field.name);
        let size = match field.repitition {
            crate::schema::Repitition::Single => 1,
            crate::schema::Repitition::Fixed(v) => v,
            crate::schema::Repitition::Unbounded => reader.u32() as usize,
            crate::schema::Repitition::Bounded(max_size) => {
                let size = reader.u32() as usize;
                if size > max_size {
                    return Err(Error::ArraySizeTooLarge {
                        field: field_id.to_string(),
                        size,
                        max_size,
                    });
                }
                size
            }
        };
        #[allow(clippy::needless_range_loop)]
        for i in 0..size {
            field_id.push_index(i);
            match &field.ty {
                crate::schema::Type::Bool => visitor
                    .visit_bool(field_id, reader.bool())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I8 => visitor
                    .visit_i8(field_id, reader.i8())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U8 => visitor
                    .visit_u8(field_id, reader.u8())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I16 => visitor
                    .visit_i16(field_id, reader.i16())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U16 => visitor
                    .visit_u16(field_id, reader.u16())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I32 => visitor
                    .visit_i32(field_id, reader.i32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U32 => visitor
                    .visit_u32(field_id, reader.u32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::I64 => visitor
                    .visit_i64(field_id, reader.i64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::U64 => visitor
                    .visit_u64(field_id, reader.u64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::F32 => visitor
                    .visit_f32(field_id, reader.f32())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::F64 => visitor
                    .visit_f64(field_id, reader.f64())
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::Str => visitor
                    .visit_str(field_id, reader.str()?)
                    .map_err(Error::from_visitor)?,
                crate::schema::Type::Msg(msg_schema) => {
                    decode_inner(msg_schema, reader, visitor, field_id)?;
                }
            }
            field_id.pop();
        }
        field_id.pop();
    }
    Ok(())
}
