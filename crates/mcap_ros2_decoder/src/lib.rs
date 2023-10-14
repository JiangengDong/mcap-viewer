#![warn(clippy::pedantic)]
#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{
    collections::{hash_map::RandomState, HashMap},
    sync::{Arc, RwLock},
};

use mcap_decoder::Visitor;

pub mod schema;

pub mod decode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Schema error: {0}")]
    Schema(#[from] schema::Error),
    #[error("Decode error: {0}")]
    Decode(#[from] decode::Error),
}

#[derive(Clone, Default)]
pub struct Decoder<S = RandomState> {
    message_table: Arc<RwLock<HashMap<schema::MsgId, Arc<schema::Msg>, S>>>,
}

impl Decoder<RandomState> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S> mcap_decoder::Decoder for Decoder<S>
where
    S: std::hash::BuildHasher,
{
    type Error = Error;

    type Schema = Arc<schema::Msg>;

    fn parse_schema(
        &self,
        schema_name: &str,
        schema_text: &[u8],
    ) -> Result<Self::Schema, Self::Error> {
        let mut message_table = self.message_table.write().unwrap();
        let schema = schema::parse(schema_name, schema_text, &mut message_table)?;
        Ok(schema)
    }

    fn get_schema(&self, schema_name: &str) -> Result<Option<Self::Schema>, Self::Error> {
        let message_table = self.message_table.read().unwrap();
        let schema = schema::get(schema_name, &message_table)?;
        Ok(schema)
    }

    fn decode<V>(
        &self,
        schema_name: &str,
        schema_text: &[u8],
        data: &[u8],
        visitor: &mut V,
    ) -> Result<(), Self::Error>
    where
        V: Visitor,
        V::Error: std::error::Error + Send + Sync + 'static,
    {
        let schema = if let Some(schema) = self.get_schema(schema_name)? {
            schema
        } else {
            self.parse_schema(schema_name, schema_text)?
        };
        decode::decode(&schema, data, visitor)?;
        Ok(())
    }
}
