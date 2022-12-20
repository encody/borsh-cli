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

    /// Format output
    #[arg(short, long)]
    pub pretty: bool,
}

pub struct Decode<'a> {
    pub input: Vec<u8>,
    pub output: Box<dyn Write + 'a>,
    pub pretty: bool,
}

impl TryFrom<&'_ DecodeArgs> for Decode<'_> {
    type Error = IOError;

    fn try_from(
        DecodeArgs {
            input_path,
            output_path,
            pretty,
        }: &'_ DecodeArgs,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            input: get_input_bytes(input_path.as_ref())?,
            output: output_writer(output_path.as_ref())?,
            pretty: *pretty,
        })
    }
}

impl Execute for Decode<'_> {
    fn execute(&mut self) -> Result<(), IOError> {
        let mut buf = &self.input as &[u8];

        let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf)
            .map_err(|_| IOError::DeserializeBorsh("schema"))?;

        let value = crate::dynamic_schema::deserialize_from_schema(&mut buf, &schema)
            .map_err(|_| IOError::DeserializeBorsh("data according to embedded schema"))?;

        output_json(&mut self.output, &value, self.pretty)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
    use serde::{Deserialize, Serialize};

    use crate::command::Execute;

    use super::Decode;

    #[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, BorshSchema, Debug)]
    struct Parent {
        integer: u32,
        vector: [u8; 8],
        child: Child,
    }

    #[derive(
        Serialize, Deserialize, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq, Debug,
    )]
    struct Child {
        string: String,
        boolean: bool,
    }

    #[test]
    fn test() {
        let value = Parent {
            integer: 24,
            vector: [8, 7, 6, 5, 4, 3, 2, 1],
            child: Child {
                string: "()".to_string(),
                boolean: false,
            },
        };

        let mut output_vector: Vec<u8> = vec![];
        let writer = BufWriter::new(&mut output_vector);

        let mut p = Decode {
            input: borsh::try_to_vec_with_schema(&value).unwrap(),
            output: Box::new(writer),
            pretty: false,
        };

        p.execute().unwrap();
        drop(p);

        let expected = serde_json::to_value(&value).unwrap();

        assert_eq!(
            expected,
            serde_json::from_slice::<serde_json::Value>(&output_vector).unwrap()
        );
    }
}
