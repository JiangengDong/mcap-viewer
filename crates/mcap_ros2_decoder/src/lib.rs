#![warn(clippy::pedantic)]
#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use mcap_decoder::Visitor;

pub mod schema;

pub mod decode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Schema error: {0}")]
    Schema(schema::Error),
    #[error("Decode error: {0}")]
    Decode(decode::Error),
}

pub struct Decoder<S> {
    message_table: Arc<RwLock<HashMap<schema::MsgId, Arc<schema::Msg>, S>>>,
}

impl<S> mcap_decoder::Decoder for Decoder<S> {
    type Error = Error;

    type Schema = Arc<schema::Msg>;

    fn parse_schema(
        &self,
        schema_name: &str,
        schema_text: &[u8],
    ) -> Result<Self::Schema, Self::Error> {
        todo!()
    }

    fn get_schema(&self, schema_name: &str) -> Result<Option<Self::Schema>, Self::Error> {
        todo!()
    }

    fn decode<V>(&self, schema_name: &str, input: &[u8], visitor: &mut V) -> Result<(), Self::Error>
    where
        V: Visitor,
        V::Error: std::error::Error,
    {
        todo!()
    }
}
