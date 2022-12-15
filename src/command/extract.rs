use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize};
use clap::Args;

use super::{get_input_bytes, output_borsh, output_writer, Execute, IOError};

#[derive(Args, Debug)]
/// Extract the Borsh schema header.
pub struct ExtractArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,
}

pub struct Extract {
    pub schema: BorshSchemaContainer,
    pub output: Box<dyn Write>,
}

impl TryFrom<&'_ ExtractArgs> for Extract {
    type Error = IOError;

    fn try_from(ExtractArgs { input, output }: &'_ ExtractArgs) -> Result<Self, Self::Error> {
        let input_bytes = get_input_bytes(input.as_ref())?;

        let mut buf = &input_bytes as &[u8];

        let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf)
            .map_err(|_| IOError::DeserializeBorsh("schema"))?;

        Ok(Self {
            schema,
            output: output_writer(output.as_ref())?,
        })
    }
}

impl Execute for Extract {
    fn execute(&mut self) -> Result<(), IOError> {
        output_borsh(&mut self.output, &self.schema)
    }
}
