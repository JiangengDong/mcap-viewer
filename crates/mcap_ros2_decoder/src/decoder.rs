use std::str::Utf8Error;

use mcap_decoder::Visitor;

use crate::schema::Msg;

use self::cdr_reader::CdrReader;

mod cdr_reader;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError<E> {
    #[error("The array size for field {field} has maximum size {max_size}, but got {size}")]
    ArraySizeTooLarge {
        field: String,
        size: usize,
        max_size: usize,
    },
    #[error("Got error from the visitor: {0}")]
    Visitor(#[from] E),
    #[error("Got utf8 error {error} when decoding data (hex representation): {data:02?}")]
    Utf8Error {
        #[source]
        error: Utf8Error,
        data: Vec<u8>,
    },
}

pub fn decode<V, E>(schema: &Msg, input: &[u8], visitor: &mut V) -> Result<(), DecodeError<E>>
where
    V: Visitor<Error = E>,
{
    let mut reader = CdrReader::new(input);
    let mut field_path = String::with_capacity(256);
    decode_inner(schema, &mut reader, visitor, &mut field_path)
}

const INT_STR_TABLE: [&str; 16] = [
    "[0]", "[1]", "[2]", "[3]", "[4]", "[5]", "[6]", "[7]", "[8]", "[9]", "[10]", "[11]", "[12]",
    "[13]", "[14]", "[15]",
];

fn decode_inner<V, E>(
    schema: &Msg,
    reader: &mut CdrReader,
    visitor: &mut V,
    path: &mut String,
) -> Result<(), DecodeError<E>>
where
    V: Visitor<Error = E>,
{
    let old_path_len = path.len();
    for field in &schema.fields {
        path.push_str(&field.name);
        let size = match field.repitition {
            crate::schema::Repitition::Single => 1,
            crate::schema::Repitition::Fixed(v) => v,
            crate::schema::Repitition::Unbounded => reader.u32() as usize,
            crate::schema::Repitition::Bounded(max_size) => {
                let size = reader.u32() as usize;
                if size > max_size {
                    return Err(DecodeError::ArraySizeTooLarge {
                        field: format!("{}.{}", &path, field.name),
                        size,
                        max_size,
                    });
                }
                size
            }
        };
        let new_path_len = path.len();
        #[allow(clippy::needless_range_loop)]
        for i in 0..size {
            if field.repitition != crate::schema::Repitition::Single {
                if i < 16 {
                    path.push_str(INT_STR_TABLE[i]);
                } else {
                    path.push('[');
                    path.push_str(&i.to_string());
                    path.push(']');
                }
            }
            match &field.ty {
                crate::schema::Type::Bool => visitor.visit_bool(path, reader.bool())?,
                crate::schema::Type::I8 => visitor.visit_i8(path, reader.i8())?,
                crate::schema::Type::U8 => visitor.visit_u8(path, reader.u8())?,
                crate::schema::Type::I16 => visitor.visit_i16(path, reader.i16())?,
                crate::schema::Type::U16 => visitor.visit_u16(path, reader.u16())?,
                crate::schema::Type::I32 => visitor.visit_i32(path, reader.i32())?,
                crate::schema::Type::U32 => visitor.visit_u32(path, reader.u32())?,
                crate::schema::Type::I64 => visitor.visit_i64(path, reader.i64())?,
                crate::schema::Type::U64 => visitor.visit_u64(path, reader.u64())?,
                crate::schema::Type::F32 => visitor.visit_f32(path, reader.f32())?,
                crate::schema::Type::F64 => visitor.visit_f64(path, reader.f64())?,
                crate::schema::Type::Str => visitor.visit_str(path, reader.str()?)?,
                crate::schema::Type::Msg(msg_schema) => {
                    decode_inner(msg_schema, reader, visitor, path)?;
                }
            }
            path.truncate(new_path_len);
        }
        path.truncate(old_path_len);
    }
    Ok(())
}
