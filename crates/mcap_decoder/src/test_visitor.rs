use crate::FieldId;

use super::Visitor;

pub struct StdoutVisitor;

impl Visitor for StdoutVisitor {
    type Error = std::convert::Infallible;

    fn visit_bool(&mut self, field: &FieldId, value: bool) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_i8(&mut self, field: &FieldId, value: i8) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_i16(&mut self, field: &FieldId, value: i16) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_i32(&mut self, field: &FieldId, value: i32) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_i64(&mut self, field: &FieldId, value: i64) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_u8(&mut self, field: &FieldId, value: u8) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_u16(&mut self, field: &FieldId, value: u16) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_u32(&mut self, field: &FieldId, value: u32) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_u64(&mut self, field: &FieldId, value: u64) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_f32(&mut self, field: &FieldId, value: f32) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_f64(&mut self, field: &FieldId, value: f64) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }

    fn visit_str(&mut self, field: &FieldId, value: &str) -> Result<(), Self::Error> {
        println!("{field}: {value}");
        Ok(())
    }
}

pub struct NoopVisitor;

impl Visitor for NoopVisitor {
    type Error = std::convert::Infallible;

    fn visit_bool(&mut self, field: &FieldId, value: bool) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_i8(&mut self, field: &FieldId, value: i8) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_i16(&mut self, field: &FieldId, value: i16) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_i32(&mut self, field: &FieldId, value: i32) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_i64(&mut self, field: &FieldId, value: i64) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_u8(&mut self, field: &FieldId, value: u8) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_u16(&mut self, field: &FieldId, value: u16) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_u32(&mut self, field: &FieldId, value: u32) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_u64(&mut self, field: &FieldId, value: u64) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_f32(&mut self, field: &FieldId, value: f32) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_f64(&mut self, field: &FieldId, value: f64) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }

    fn visit_str(&mut self, field: &FieldId, value: &str) -> Result<(), Self::Error> {
        std::hint::black_box((field, value));
        Ok(())
    }
}
