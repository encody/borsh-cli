use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize};
use clap::Args;

use super::{get_input_bytes, output_bytes, output_writer, Execute, IOError};

#[derive(Args, Debug)]
/// Remove the Borsh schema header.
pub struct StripArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,
}

pub struct Strip {
    pub input_bytes: Vec<u8>,
    pub schema_length: usize,
    pub output: Box<dyn Write>,
}

impl TryFrom<&'_ StripArgs> for Strip {
    type Error = IOError;

    fn try_from(StripArgs { input, output }: &'_ StripArgs) -> Result<Self, Self::Error> {
        let input_bytes = get_input_bytes(input.as_ref())?;

        let mut buf = &input_bytes as &[u8];
        let buf_length = buf.len();

        let _ = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf)
            .map_err(|_| IOError::DeserializeBorsh("schema"));

        let schema_length = buf_length - buf.len();

        Ok(Self {
            input_bytes,
            schema_length,
            output: output_writer(output.as_ref())?,
        })
    }
}

impl Execute for Strip {
    fn execute(&mut self) -> Result<(), IOError> {
        output_bytes(&mut self.output, &self.input_bytes[self.schema_length..])
    }
}
