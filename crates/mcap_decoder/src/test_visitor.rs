use super::Visitor;

pub struct StdoutVisitor;

impl Visitor for StdoutVisitor {
    type Error = std::convert::Infallible;

    fn visit_bool(&mut self, path: &str, value: bool) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_i8(&mut self, path: &str, value: i8) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_i16(&mut self, path: &str, value: i16) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_i32(&mut self, path: &str, value: i32) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_i64(&mut self, path: &str, value: i64) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_u8(&mut self, path: &str, value: u8) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_u16(&mut self, path: &str, value: u16) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_u32(&mut self, path: &str, value: u32) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_u64(&mut self, path: &str, value: u64) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_f32(&mut self, path: &str, value: f32) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_f64(&mut self, path: &str, value: f64) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_str(&mut self, path: &str, value: &str) -> Result<(), Self::Error> {
        println!("{path}: {value}");
        Ok(())
    }

    fn visit_bytes(&mut self, path: &str, value: &[u8]) -> Result<(), Self::Error> {
        println!("{path}: {value:02X?}");
        Ok(())
    }
}

pub struct NoopVisitor;

impl Visitor for NoopVisitor {
    type Error = std::convert::Infallible;

    fn visit_bool(&mut self, path: &str, value: bool) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_i8(&mut self, path: &str, value: i8) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_i16(&mut self, path: &str, value: i16) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_i32(&mut self, path: &str, value: i32) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_i64(&mut self, path: &str, value: i64) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_u8(&mut self, path: &str, value: u8) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_u16(&mut self, path: &str, value: u16) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_u32(&mut self, path: &str, value: u32) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_u64(&mut self, path: &str, value: u64) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_f32(&mut self, path: &str, value: f32) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_f64(&mut self, path: &str, value: f64) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_str(&mut self, path: &str, value: &str) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }

    fn visit_bytes(&mut self, path: &str, value: &[u8]) -> Result<(), Self::Error> {
        std::hint::black_box((path, value));
        Ok(())
    }
}
