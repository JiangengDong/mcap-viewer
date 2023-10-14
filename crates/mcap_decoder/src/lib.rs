use std::fmt::Display;

#[cfg(feature = "test-utils")]
pub mod test_visitor;

pub trait Decoder {
    type Error;
    type Schema;

    fn parse_schema(
        &self,
        schema_name: &str,
        schema_text: &[u8],
    ) -> Result<Self::Schema, Self::Error>;

    fn get_schema(&self, schema_name: &str) -> Result<Option<Self::Schema>, Self::Error>;

    fn decode<V>(
        &self,
        schema_name: &str,
        schema_text: &[u8],
        data: &[u8],
        visitor: &mut V,
    ) -> Result<(), Self::Error>
    where
        V: Visitor,
        V::Error: std::error::Error + Send + Sync + 'static;
}

#[derive(Clone, Debug)]
enum FieldPart {
    Member(Box<str>),
    Index,
}

#[derive(Clone, Debug, Default)]
pub struct FieldId {
    parts: Vec<FieldPart>,
    index: Vec<usize>,
}

impl FieldId {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            parts: Vec::new(),
            index: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            parts: Vec::with_capacity(capacity),
            index: Vec::with_capacity(capacity),
        }
    }

    pub fn push_member<S>(&mut self, member: S)
    where
        S: Into<Box<str>>,
    {
        self.parts.push(FieldPart::Member(member.into()));
    }

    pub fn push_index(&mut self, index: usize) {
        self.parts.push(FieldPart::Index);
        self.index.push(index);
    }

    pub fn pop(&mut self) {
        if let Some(FieldPart::Index) = self.parts.pop() {
            self.index.pop();
        }
    }

    pub fn to_no_index_string(&self) -> String {
        let mut total_len = 0;
        for part in &self.parts {
            match part {
                FieldPart::Member(s) => total_len += s.len(),
                FieldPart::Index => total_len += 4,
            }
        }

        let mut total_s = String::with_capacity(total_len);
        for part in &self.parts {
            match part {
                FieldPart::Member(s) => total_s.push_str(s),
                FieldPart::Index => total_s.push_str("[]"),
            }
        }

        total_s
    }

    pub fn indices(&self) -> &[usize] {
        &self.index
    }
}

impl Display for FieldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut idx_iter = self.index.iter().copied();
        for part in &self.parts {
            match part {
                FieldPart::Member(member) => write!(f, ".{}", member)?,
                FieldPart::Index => write!(f, "[{}]", idx_iter.next().unwrap())?,
            }
        }
        Ok(())
    }
}

macro_rules! gen_visitor_method {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self, field: &FieldId, value: $ty) -> Result<(), Self::Error>;
    };
}

pub trait Visitor {
    type Error;

    gen_visitor_method!(visit_bool, bool);
    gen_visitor_method!(visit_i8, i8);
    gen_visitor_method!(visit_i16, i16);
    gen_visitor_method!(visit_i32, i32);
    gen_visitor_method!(visit_i64, i64);
    gen_visitor_method!(visit_u8, u8);
    gen_visitor_method!(visit_u16, u16);
    gen_visitor_method!(visit_u32, u32);
    gen_visitor_method!(visit_u64, u64);
    gen_visitor_method!(visit_f32, f32);
    gen_visitor_method!(visit_f64, f64);
    gen_visitor_method!(visit_str, &str);
}
