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

pub struct Extract<'a> {
    pub input: Vec<u8>,
    pub output: Box<dyn Write + 'a>,
}

impl TryFrom<&'_ ExtractArgs> for Extract<'_> {
    type Error = IOError;

    fn try_from(ExtractArgs { input, output }: &'_ ExtractArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            input: get_input_bytes(input.as_ref())?,
            output: output_writer(output.as_ref())?,
        })
    }
}

impl Execute for Extract<'_> {
    fn execute(&mut self) -> Result<(), IOError> {
        let mut buf = &self.input as &[u8];

        let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf)
            .map_err(|_| IOError::DeserializeBorsh("schema"))?;

        output_borsh(&mut self.output, &schema)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize, schema::BorshSchemaContainer};
    use serde::{Deserialize, Serialize};

    use crate::command::Execute;

    use super::Extract;

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

        let mut p = Extract {
            input: borsh::try_to_vec_with_schema(&value).unwrap(),
            output: Box::new(writer),
        };

        p.execute().unwrap();
        drop(p);

        let expected = Parent::schema_container();

        assert_eq!(
            expected,
            BorshSchemaContainer::try_from_slice(&output_vector).unwrap(),
        );
    }
}
