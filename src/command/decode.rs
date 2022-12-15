use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize};
use clap::Args;

use super::{get_input_bytes, output_json, output_writer, Execute, IOError};

#[derive(Args, Debug)]
/// Decode Borsh input to JSON.
///
/// Requires the input to contain the embedded schema.
pub struct DecodeArgs {
    /// Read input from this file, otherwise from stdin.
    pub input_path: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output_path: Option<PathBuf>,
}

pub struct Decode {
    pub input: serde_json::Value,
    pub output: Box<dyn Write>,
}

impl TryFrom<&'_ DecodeArgs> for Decode {
    type Error = IOError;

    fn try_from(
        DecodeArgs {
            input_path,
            output_path,
        }: &'_ DecodeArgs,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            input: {
                let input_bytes = get_input_bytes(input_path.as_ref())?;

                let mut buf = &input_bytes as &[u8];

                let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf)
                    .map_err(|_| IOError::DeserializeBorsh("schema"))?;

                let value = crate::dynamic_schema::deserialize_from_schema(&mut buf, &schema)
                    .map_err(|_| IOError::DeserializeBorsh("data according to embedded schema"))?;
                value
            },
            output: output_writer(output_path.as_ref())?,
        })
    }
}

impl Execute for Decode {
    fn execute(&mut self) -> Result<(), IOError> {
        output_json(&mut self.output, &self.input)
    }
}
