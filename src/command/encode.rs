use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize, BorshSerialize};
use clap::Args;

use crate::{dynamic_schema::serialize_with_schema, json_borsh::JsonSerializableAsBorsh};

use super::{get_input_bytes, output_borsh2, output_writer, Execute, IOError};

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

pub struct Encode {
    pub input: serde_json::Value,
    pub output: Box<dyn Write>,
    pub schema: Option<BorshSchemaContainer>,
}

impl TryFrom<&'_ EncodeArgs> for Encode {
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

impl Execute for Encode {
    fn execute(&mut self) -> Result<(), IOError> {
        let writer = &mut self.output;
        if let Some(schema) = &self.schema {
            BorshSerialize::serialize(&schema, writer).map_err(|_| IOError::WriteBorsh)?;
            serialize_with_schema(writer, &self.input, &schema).map_err(|_| IOError::WriteBorsh)?;
            Ok(())
        } else {
            output_borsh2(writer, &JsonSerializableAsBorsh(&self.input))
        }
    }
}
