use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize, BorshSerialize};
use clap::Args;

use crate::{dynamic_schema::serialize_with_schema, json_borsh::JsonSerializableAsBorsh};

use super::{get_input_bytes, output_borsh, output_writer, Execute, IOError};

#[derive(Args, Debug)]
/// Convert JSON to Borsh.
///
/// Note: If a schema is not specified, values that can be null (e.g. a
/// Rust Option<T>), etc. WILL NOT be serialized correctly.
pub struct EncodeArgs {
    /// Read input from this file, otherwise from stdin.
    pub input_path: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output_path: Option<PathBuf>,

    /// Schema to follow when serializing.
    #[arg(short, long)]
    pub schema: Option<PathBuf>,
}

pub struct Encode<'a> {
    pub input: serde_json::Value,
    pub output: Box<dyn Write + 'a>,
    pub schema: Option<BorshSchemaContainer>,
}

impl TryFrom<&'_ EncodeArgs> for Encode<'_> {
    type Error = IOError;

    fn try_from(args: &'_ EncodeArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            input: serde_json::from_slice(&get_input_bytes(args.input_path.as_ref())?)
                .map_err(|_e| IOError::DeserializeJson)?,
            output: output_writer(args.output_path.as_ref())?,
            schema: if let Some(ref path) = args.schema {
                let schema_bytes = get_input_bytes(Some(path))?;
                let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(
                    &mut (&schema_bytes as &[u8]),
                )
                .map_err(|_| IOError::DeserializeBorsh("schema header"))?;

                Some(schema)
            } else {
                None
            },
        })
    }
}

impl Execute for Encode<'_> {
    fn execute(&mut self) -> Result<(), IOError> {
        let writer = &mut self.output;
        if let Some(schema) = &self.schema {
            BorshSerialize::serialize(&schema, writer).map_err(|_| IOError::WriteBorsh)?;
            serialize_with_schema(writer, &self.input, schema).map_err(|_| IOError::WriteBorsh)?;
            Ok(())
        } else {
            output_borsh(writer, &JsonSerializableAsBorsh(&self.input))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
    use serde::{Deserialize, Serialize};

    use crate::command::Execute;

    use super::Encode;

    #[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, BorshSchema, Debug)]
    struct Parent {
        integer: u32,
        vector: [u8; 8],
        child: Child,
    }

    #[derive(
        Serialize, Deserialize, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq, Debug,
    )]
    struct JsonParent {
        integer: f64,
        vector: Vec<f64>,
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
    fn with_schema() {
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

        let mut p = Encode {
            input: serde_json::to_value(&value).unwrap(),
            output: Box::new(writer),
            schema: Some(Parent::schema_container()),
        };

        p.execute().unwrap();
        drop(p);

        let expected = borsh::try_to_vec_with_schema(&value).unwrap();

        assert_eq!(expected, output_vector);
    }

    #[test]
    fn without_schema() {
        let value = Parent {
            integer: 24,
            vector: [8, 7, 6, 5, 4, 3, 2, 1],
            child: Child {
                string: "()".to_string(),
                boolean: false,
            },
        };
        let expected = JsonParent {
            integer: 24.0,
            vector: vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0],
            child: Child {
                string: "()".to_string(),
                boolean: false,
            },
        };

        let mut output_vector: Vec<u8> = vec![];
        let writer = BufWriter::new(&mut output_vector);

        let mut p = Encode {
            input: serde_json::to_value(value).unwrap(),
            output: Box::new(writer),
            schema: None,
        };

        p.execute().unwrap();
        drop(p);

        assert_eq!(
            <JsonParent as BorshDeserialize>::try_from_slice(&output_vector).unwrap(),
            expected,
        );
    }
}
