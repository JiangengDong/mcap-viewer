use std::collections::HashMap;

use mcap_decoder::FieldId;

#[derive(Debug, Default)]
pub struct DataStorage(pub HashMap<String, TopicStorage>);

impl DataStorage {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, topic: &str, timestamp: u64) -> Visitor<'_> {
        let topic_storage = self.0.entry(topic.to_owned()).or_default();
        Visitor {
            storage: topic_storage,
            timestamp,
        }
    }

    pub fn merge(&mut self, rhs: DataStorage) {
        for (topic, rhs_topic_storage) in rhs.0 {
            let topic_storage = self.0.entry(topic).or_default();
            for (field, rhs_time_series) in rhs_topic_storage.0 {
                let time_series = topic_storage.0.entry(field).or_default();
                match rhs_time_series {
                    TimeSeries::Uninit => {}
                    TimeSeries::Bool(v) => time_series.as_mut_bool().unwrap().extend(v),
                    TimeSeries::I8(v) => time_series.as_mut_i8().unwrap().extend(v),
                    TimeSeries::I16(v) => time_series.as_mut_i16().unwrap().extend(v),
                    TimeSeries::I32(v) => time_series.as_mut_i32().unwrap().extend(v),
                    TimeSeries::I64(v) => time_series.as_mut_i64().unwrap().extend(v),
                    TimeSeries::U8(v) => time_series.as_mut_u8().unwrap().extend(v),
                    TimeSeries::U16(v) => time_series.as_mut_u16().unwrap().extend(v),
                    TimeSeries::U32(v) => time_series.as_mut_u32().unwrap().extend(v),
                    TimeSeries::U64(v) => time_series.as_mut_u64().unwrap().extend(v),
                    TimeSeries::F32(v) => time_series.as_mut_f32().unwrap().extend(v),
                    TimeSeries::F64(v) => time_series.as_mut_f64().unwrap().extend(v),
                }
            }
        }
    }

    pub fn sort_unstable(&mut self) {
        for topic_storage in self.0.values_mut() {
            topic_storage.sort_unstable();
        }
    }
}

#[derive(Debug, Default)]
pub struct TopicStorage(pub HashMap<String, TimeSeries>);

impl TopicStorage {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn sort_unstable(&mut self) {
        for time_series in self.0.values_mut() {
            time_series.sort_unstable();
        }
    }
}

pub struct Visitor<'a> {
    storage: &'a mut TopicStorage,
    timestamp: u64,
}

macro_rules! gen_visitor_method {
    ($name:ident, $forward_name:ident, $ty:ty) => {
        fn $name(&mut self, field: &FieldId, value: $ty) -> Result<(), Self::Error> {
            let time_series = self.storage.0.entry(field.to_index_string()).or_default();
            time_series.$forward_name(self.timestamp, value);
            Ok(())
        }
    };
}

impl mcap_decoder::Visitor for Visitor<'_> {
    type Error = ();

    gen_visitor_method!(visit_bool, push_bool, bool);
    gen_visitor_method!(visit_i8, push_i8, i8);
    gen_visitor_method!(visit_i16, push_i16, i16);
    gen_visitor_method!(visit_i32, push_i32, i32);
    gen_visitor_method!(visit_i64, push_i64, i64);
    gen_visitor_method!(visit_u8, push_u8, u8);
    gen_visitor_method!(visit_u16, push_u16, u16);
    gen_visitor_method!(visit_u32, push_u32, u32);
    gen_visitor_method!(visit_u64, push_u64, u64);
    gen_visitor_method!(visit_f32, push_f32, f32);
    gen_visitor_method!(visit_f64, push_f64, f64);

    fn visit_str(&mut self, _field: &FieldId, _value: &str) -> Result<(), Self::Error> {
        // TODO: implement this when I really need to access string
        Ok(())
    }
}

#[derive(Debug, Default)]
pub enum TimeSeries {
    #[default]
    Uninit,
    Bool(Vec<(u64, bool)>),
    I8(Vec<(u64, i8)>),
    I16(Vec<(u64, i16)>),
    I32(Vec<(u64, i32)>),
    I64(Vec<(u64, i64)>),
    U8(Vec<(u64, u8)>),
    U16(Vec<(u64, u16)>),
    U32(Vec<(u64, u32)>),
    U64(Vec<(u64, u64)>),
    F32(Vec<(u64, f32)>),
    F64(Vec<(u64, f64)>),
}

macro_rules! gen_time_series_method {
    ($push_method_name:ident, $as_method_name:ident, $as_mut_method_name:ident, $ty:ty, $variant:ident) => {
        pub fn $push_method_name(&mut self, time: u64, value: $ty) {
            match self {
                TimeSeries::Uninit => {
                    *self = TimeSeries::$variant(vec![(time, value)]);
                }
                TimeSeries::$variant(v) => v.push((time, value)),
                _ => panic!(
                    "{} is called on a mismatched type",
                    stringify!($push_method_name)
                ),
            }
        }

        pub fn $as_method_name(&self) -> Option<&Vec<(u64, $ty)>> {
            match self {
                TimeSeries::$variant(v) => Some(v),
                _ => None,
            }
        }

        pub fn $as_mut_method_name(&mut self) -> Option<&mut Vec<(u64, $ty)>> {
            match self {
                TimeSeries::$variant(v) => Some(v),
                _ => None,
            }
        }
    };
}

impl TimeSeries {
    pub fn is_empty(&self) -> bool {
        match self {
            TimeSeries::Uninit => true,
            TimeSeries::Bool(v) => v.is_empty(),
            TimeSeries::I8(v) => v.is_empty(),
            TimeSeries::I16(v) => v.is_empty(),
            TimeSeries::I32(v) => v.is_empty(),
            TimeSeries::I64(v) => v.is_empty(),
            TimeSeries::U8(v) => v.is_empty(),
            TimeSeries::U16(v) => v.is_empty(),
            TimeSeries::U32(v) => v.is_empty(),
            TimeSeries::U64(v) => v.is_empty(),
            TimeSeries::F32(v) => v.is_empty(),
            TimeSeries::F64(v) => v.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TimeSeries::Uninit => 0,
            TimeSeries::Bool(v) => v.len(),
            TimeSeries::I8(v) => v.len(),
            TimeSeries::I16(v) => v.len(),
            TimeSeries::I32(v) => v.len(),
            TimeSeries::I64(v) => v.len(),
            TimeSeries::U8(v) => v.len(),
            TimeSeries::U16(v) => v.len(),
            TimeSeries::U32(v) => v.len(),
            TimeSeries::U64(v) => v.len(),
            TimeSeries::F32(v) => v.len(),
            TimeSeries::F64(v) => v.len(),
        }
    }

    pub fn sort_unstable(&mut self) {
        match self {
            TimeSeries::Uninit => {}
            TimeSeries::Bool(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::I8(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::I16(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::I32(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::I64(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::U8(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::U16(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::U32(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::U64(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::F32(v) => v.sort_unstable_by_key(|(time, _)| *time),
            TimeSeries::F64(v) => v.sort_unstable_by_key(|(time, _)| *time),
        }
    }

    gen_time_series_method!(push_bool, as_bool, as_mut_bool, bool, Bool);
    gen_time_series_method!(push_i8, as_i8, as_mut_i8, i8, I8);
    gen_time_series_method!(push_i16, as_i16, as_mut_i16, i16, I16);
    gen_time_series_method!(push_i32, as_i32, as_mut_i32, i32, I32);
    gen_time_series_method!(push_i64, as_i64, as_mut_i64, i64, I64);
    gen_time_series_method!(push_u8, as_u8, as_mut_u8, u8, U8);
    gen_time_series_method!(push_u16, as_u16, as_mut_u16, u16, U16);
    gen_time_series_method!(push_u32, as_u32, as_mut_u32, u32, U32);
    gen_time_series_method!(push_u64, as_u64, as_mut_u64, u64, U64);
    gen_time_series_method!(push_f32, as_f32, as_mut_f32, f32, F32);
    gen_time_series_method!(push_f64, as_f64, as_mut_f64, f64, F64);
}
