use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

use borsh::BorshSerialize;
use clap::Subcommand;
use serde::Serialize;
use thiserror::Error;

use self::{
    decode::Decode, encode::Encode, extract::Extract, pack::Pack, strip::Strip, unpack::Unpack,
};

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
        #[inline]
        fn run_args<E: Execute>(args: impl TryInto<E, Error = IOError>) -> Result<(), IOError> {
            E::execute(&mut args.try_into()?)
        }

        if let Err(e) = match self {
            Command::Pack(args) => run_args::<Pack>(args),
            Command::Unpack(args) => run_args::<Unpack>(args),
            Command::Encode(args) => run_args::<Encode>(args),
            Command::Decode(args) => run_args::<Decode>(args),
            Command::Extract(args) => run_args::<Extract>(args),
            Command::Strip(args) => run_args::<Strip>(args),
        } {
            eprintln!("Error: {e}");
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
    #[error("Failed to deserialize input as JSON")]
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

fn output_bytes(mut writer: impl Write, value: &[u8]) -> Result<(), IOError> {
    writer.write_all(value).map_err(|_| IOError::WriteBytes)
}

fn output_borsh(writer: impl Write, value: impl BorshSerialize) -> Result<(), IOError> {
    borsh::to_writer(writer, &value).map_err(|_| IOError::WriteBorsh)
}

fn output_json(writer: impl Write, value: &impl Serialize, pretty: bool) -> Result<(), IOError> {
    if pretty {
        serde_json::to_writer_pretty(writer, value).map_err(|_| IOError::WriteJson)
    } else {
        serde_json::to_writer(writer, value).map_err(|_| IOError::WriteJson)
    }
}

#[cfg(test)]
#[allow(unused, dead_code)]
mod tests {
    use borsh::{BorshSchema, BorshSerialize};
    use serde::Serialize;

    use crate::command::{output_borsh, output_json, output_writer};

    #[test]
    #[ignore = "pollution"]
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
            &mut output_writer(Some(&"./dataonly.json".into())).unwrap(),
            &v,
            false,
            // Some(&First::schema_container()),
        );
        output_borsh(
            &mut output_writer(Some(&"./dataandschema.borsh".into())).unwrap(),
            (&v, &First::schema_container()),
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
