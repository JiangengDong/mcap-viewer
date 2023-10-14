use std::{mem::size_of, str};

use super::Error;

macro_rules! impl_value {
( $($name:ident), +) => {
    $(
        pub fn $name(&mut self) -> $name {
            const SIZE: usize = size_of::<$name>();
            self.align(SIZE);
            let slice = &self.data[self.offset..self.offset + SIZE].try_into().unwrap();
            let value = if self.little_endian {
                $name::from_le_bytes(*slice)
            } else {
                $name::from_be_bytes(*slice)
            };
            self.offset += SIZE;
            value
        }
    )*
};
}

pub struct CdrReader<'a> {
    data: &'a [u8],
    pub offset: usize,
    little_endian: bool,
}

impl CdrReader<'_> {
    pub fn new(data: &[u8]) -> CdrReader {
        let little_endian = data[1] & 1 == 1;
        CdrReader {
            data: &data[4..],
            offset: 0,
            little_endian,
        }
    }
    fn align(&mut self, size: usize) {
        if size > 1 {
            self.offset = self.offset.next_multiple_of(size);
        }
    }

    pub fn bool(&mut self) -> bool {
        self.u8() != 0
    }

    pub fn u8(&mut self) -> u8 {
        let value = self.data[self.offset];
        self.offset += 1;
        value
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn i8(&mut self) -> i8 {
        let value = self.data[self.offset] as i8;
        self.offset += 1;
        value
    }

    impl_value!(i16, u16, i32, u32, i64, u64, f32, f64);

    pub fn str(&mut self) -> Result<&str, Error> {
        let length = self.u32() as usize;
        if length <= 1 {
            self.offset += length;
            Ok("")
        } else {
            let data = &self.data[self.offset..self.offset + length - 1];
            self.offset += length;
            str::from_utf8(data).map_err(|e| Error::Utf8Error {
                error: e,
                data: data.to_vec(),
            })
        }
    }
}
