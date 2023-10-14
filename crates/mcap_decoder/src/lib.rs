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
        input: &[u8],
        visitor: &mut V,
    ) -> Result<(), Self::Error>
    where
        V: Visitor,
        V::Error: std::error::Error + Send + Sync + 'static;
}

macro_rules! gen_visitor_method {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self, path: &str, value: $ty) -> Result<(), Self::Error>;
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
    gen_visitor_method!(visit_bytes, &[u8]);
}
