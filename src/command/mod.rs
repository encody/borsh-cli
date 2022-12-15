use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize, BorshSchema, BorshSerialize};
use clap::{Args, Subcommand};
use serde::Serialize;
use thiserror::Error;

use crate::json_borsh::JsonSerializableAsBorsh;

use self::{encode::Encode, pack::Pack, unpack::Unpack};

mod decode;
mod encode;
mod extract;
mod pack;
mod strip;
mod unpack;

pub(self) trait Execute {
    fn execute(&mut self) -> Result<(), IOError>;
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Pack(pack::PackArgs),
    Unpack(unpack::UnpackArgs),
    Encode(encode::EncodeArgs),
    Decode(decode::DecodeArgs),
    Extract(extract::ExtractArgs),
    Strip(strip::StripArgs),
}

impl Command {
    pub fn run(&self) {
        match self {
            Command::Pack(args) => Pack::execute(&mut args.try_into().unwrap()).unwrap(),
            Command::Unpack(args) => Unpack::execute(&mut args.try_into().unwrap()).unwrap(),
            Command::Encode(args) => Encode::execute(&mut args.try_into().unwrap()).unwrap(),
            Command::Decode(decode::DecodeArgs { input, output }) => {
                let input_bytes = get_input_bytes(input.as_ref()).unwrap();

                let mut buf = &input_bytes as &[u8];

                let schema =
                    <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

                let value = crate::dynamic_schema::deserialize_from_schema(&mut buf, &schema)
                    .expect("Unable to deserialize according to embedded schema");

                output_json(output.as_ref(), &value).unwrap();
            }
            Command::Extract(extract::ExtractArgs { input, output }) => {
                let input_bytes = get_input_bytes(input.as_ref()).unwrap();

                let mut buf = &input_bytes as &[u8];

                let schema =
                    <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

                output_borsh(output.as_ref(), &schema, None);
            }
            Command::Strip(strip::StripArgs { input, output }) => {
                let input_bytes = get_input_bytes(input.as_ref()).unwrap();

                let mut buf = &input_bytes as &[u8];

                let _ = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

                output_writer(output.as_ref())
                    .unwrap()
                    .write_all(buf)
                    .expect("Unable to write output");
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum IOError {
    #[error("Failed to read input file {0}")]
    ReadInputFile(String),
    #[error("Failed to read from STDIN")]
    ReadStdin,
    #[error("Failed to create output file {0}")]
    CreateOutputFile(String),
    #[error("Failed to write Borsh")]
    WriteBorsh,
    #[error("Failed to write JSON")]
    WriteJson,
    #[error("Failed to write raw bytes")]
    WriteBytes,
    #[error("Failed to deserialize input as Borsh {0}")]
    DeserializeBorsh(&'static str),
    #[error("Failed to deserialize input as Json")]
    DeserializeJson,
    #[error("Unexpected schema header: {0}")]
    IncorrectBorshSchemaHeader(String),
}

fn get_input_bytes(input_path: Option<&PathBuf>) -> Result<Vec<u8>, IOError> {
    input_path.map_or_else(read_stdin, |path| {
        fs::read(path).map_err(|_| IOError::ReadInputFile(path.display().to_string()))
    })
}

fn read_stdin() -> Result<Vec<u8>, IOError> {
    let mut v = Vec::new();
    io::stdin()
        .read_to_end(&mut v)
        .map_err(|_e| IOError::ReadStdin)?;
    Ok(v)
}

fn output_writer(output: Option<&PathBuf>) -> Result<Box<dyn Write>, IOError> {
    if let Some(o) = output {
        let f =
            fs::File::create(o).map_err(|_e| IOError::CreateOutputFile(o.display().to_string()))?;
        Ok(Box::new(f) as Box<dyn Write>)
    } else {
        Ok(Box::new(io::stdout()) as Box<dyn Write>)
    }
}

fn output_bytes(writer: &mut impl Write, value: &[u8]) -> Result<(), IOError> {
    writer.write_all(value).map_err(|_| IOError::WriteBytes)
}

fn output_borsh2(writer: impl Write, value: impl BorshSerialize) -> Result<(), IOError> {
    borsh::to_writer(writer, &value).map_err(|_| IOError::WriteBorsh)
}

fn output_borsh(
    output: Option<&PathBuf>,
    value: &impl BorshSerialize,
    schema: Option<&BorshSchemaContainer>,
) {
    let writer = output_writer(output).unwrap();

    if let Some(schema) = schema {
        borsh::to_writer(writer, &(schema, value))
    } else {
        borsh::to_writer(writer, value)
    }
    .expect("Failed to write Borsh");
}

fn output_json(output: Option<&PathBuf>, value: &impl Serialize) -> Result<(), IOError> {
    let writer = output_writer(output)?;
    serde_json::to_writer(writer, value).map_err(|_| IOError::WriteJson)
}

#[cfg(test)]
#[allow(unused, dead_code)]
mod tests {
    use borsh::{BorshSchema, BorshSerialize};
    use serde::Serialize;

    use super::{output_borsh, output_json};

    #[test]
    fn test_schema() {
        #[derive(BorshSerialize, BorshSchema, Serialize)]
        struct First {
            a: (u32, u64),
            b: String,
            c: Second,
            // d: HashMap<String, bool>,
            e: Vec<String>,
        }

        #[derive(BorshSerialize, BorshSchema, Serialize)]
        struct Second {
            a: Third,
            b: Third,
            c: Third,
            d: u32,
            e: u32,
        }

        #[derive(BorshSerialize, BorshSchema, Serialize)]
        enum Third {
            Alpha { field: u32 },
            Beta(u32),
            Gamma,
        }

        dbg!("{:?}", First::schema_container());
        // return;
        let v = First {
            a: (32, 64),
            b: "String".to_string(),
            c: Second {
                a: Third::Alpha { field: 1 },
                b: Third::Beta(1),
                c: Third::Gamma,
                d: 2,
                e: 3,
            },
            // d: vec![("true".to_string(), true), ("false".to_string(), false)]
            //     .into_iter()
            //     .collect(),
            e: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        borsh::try_to_vec_with_schema(&v);
        output_json(
            Some(&"./dataonly.json".into()),
            &v,
            // Some(&First::schema_container()),
        );
        output_borsh(
            Some(&"./dataandschema.borsh".into()),
            &v,
            Some(&First::schema_container()),
        );
    }

    #[test]
    fn ensure_key_order_is_preserved() {
        let s1 = r#"{
            "a": 1,
            "b": 1,
            "c": 1,
            "d": 1,
            "e": 1,
            "f": 1
        }"#;
        let s2 = r#"{
            "f": 1,
            "e": 1,
            "d": 1,
            "c": 1,
            "b": 1,
            "a": 1
        }"#;

        let v1 = serde_json::from_str::<serde_json::Value>(s1).unwrap();
        let v2 = serde_json::from_str::<serde_json::Value>(s2).unwrap();

        assert_eq!(v1, v2);

        let (o1, o2) = match (v1, v2) {
            (serde_json::Value::Object(o1), serde_json::Value::Object(o2)) => (o1, o2),
            _ => unreachable!(),
        };

        let k1 = o1.keys().collect::<Vec<_>>();
        let k2 = o2.keys().collect::<Vec<_>>();

        assert_ne!(k1, k2);
    }
}
