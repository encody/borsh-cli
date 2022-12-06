use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize, BorshSchema, BorshSerialize};
use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::json_borsh::JsonSerializableAsBorsh;

mod dynamic_schema;
mod json_borsh;

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for manipulating Borsh-serialized data
/// 
/// Note: Does not play particularly nicely with `HashMap<_, _>` types.
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Serialize the input as a simple binary blob with Borsh headers.
    Pack {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// By default, the Borsh schema is included in the header. Enable this flag to remove it.
        #[arg(short, long)]
        no_schema: bool,
    },
    /// Deserialize the input as a simple binary blob with Borsh headers.
    Unpack {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// By default, we assume the Borsh schema is included in the header. Enable this flag to prevent this.
        #[arg(short, long)]
        no_schema: bool,
    },
    /// Convert JSON to Borsh.
    ///
    /// Note: Schemas are not yet supported, so values that can be null (e.g. a
    /// Rust Option<T>), etc. WILL NOT be serialized correctly.
    Encode {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Schema to follow when serializing.
        #[arg(short, long)]
        schema: Option<PathBuf>,
    },
    /// Decode Borsh input to JSON.
    ///
    /// Requires the input to contain the embedded schema.
    Decode {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extracts the Borsh schema header
    Extract {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Removes the Borsh schema header
    Strip {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn get_input_bytes(input_path: Option<&PathBuf>) -> Vec<u8> {
    input_path
        .map(|path| {
            fs::read(path)
                .unwrap_or_else(|_| panic!("Could not read input file {}", path.display()))
        })
        .unwrap_or_else(|| {
            let mut v = Vec::new();
            io::stdin()
                .read_to_end(&mut v)
                .expect("Could not read from STDIN");
            v
        })
}

fn output_writer(output: Option<&PathBuf>) -> Box<dyn Write> {
    if let Some(o) = output {
        let f = fs::File::create(o)
            .unwrap_or_else(|_| panic!("Could not create output file {}", o.display()));
        Box::new(f) as Box<dyn Write>
    } else {
        Box::new(io::stdout()) as Box<dyn Write>
    }
}

fn output_borsh(
    output: Option<&PathBuf>,
    value: &impl BorshSerialize,
    schema: Option<&BorshSchemaContainer>,
) {
    let writer = output_writer(output);

    if let Some(schema) = schema {
        borsh::to_writer(writer, &(schema, value))
    } else {
        borsh::to_writer(writer, value)
    }
    .expect("Failed to write Borsh to output");
}

fn output_json(output: Option<&PathBuf>, value: &impl Serialize) {
    if let Some(o) = output {
        let path = o.display();
        let f =
            fs::File::create(o).unwrap_or_else(|_| panic!("Could not create output file {path}"));
        serde_json::to_writer(f, &value)
            .unwrap_or_else(|_| panic!("Could not write JSON to output file {path}"));
    } else {
        serde_json::to_writer(io::stdout(), &value).expect("Could not write JSON to STDOUT");
    }
}

fn main() {
    let args = Args::parse();

    match &args.command {
        Command::Pack {
            input,
            output,
            no_schema,
        } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let schema = Vec::<u8>::schema_container();

            output_borsh(
                output.as_ref(),
                &input_bytes,
                if *no_schema { None } else { Some(&schema) },
            );
        }
        Command::Unpack {
            input,
            output,
            no_schema,
        } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let value = if *no_schema {
                Vec::<u8>::try_from_slice(&input_bytes).expect("Could not read input as byte array")
            } else {
                let (schema, v) = <(BorshSchemaContainer, Vec<u8>)>::try_from_slice(&input_bytes)
                    .expect("Could not read input as byte array with schema headers");
                assert_eq!(
                    schema,
                    Vec::<u8>::schema_container(),
                    "Incorrect schema header",
                );
                v
            };

            if let Some(o) = output {
                let mut f = fs::File::create(o)
                    .unwrap_or_else(|_| panic!("Could not create output file {}", o.display()));

                f.write_all(&value)
                    .unwrap_or_else(|_| panic!("Could not write to output file {}", o.display()));
            } else {
                io::stdout()
                    .write_all(&value)
                    .expect("Could not write to STDOUT");
            }
        }
        Command::Encode {
            input,
            output,
            schema,
        } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let v = serde_json::from_slice::<serde_json::Value>(&input_bytes)
                .expect("Could not parse input as JSON");

            if let Some(schema_path) = schema {
                let schema_bytes = get_input_bytes(Some(schema_path));
                let mut writer = output_writer(output.as_ref());
                let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(
                    &mut (&schema_bytes as &[u8]),
                )
                .expect("Could not parse schema");
                BorshSerialize::serialize(&schema, &mut writer)
                    .expect("could not serialize schema to output");
                dynamic_schema::serialize_with_schema(&mut writer, &v, &schema)
                    .expect("Could not write output");
            } else {
                let v = JsonSerializableAsBorsh(&v);

                output_borsh(output.as_ref(), &v, None);
            }
        }
        Command::Decode { input, output } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let mut buf = &input_bytes as &[u8];

            let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

            let value = dynamic_schema::deserialize_from_schema(&mut buf, &schema)
                .expect("Unable to deserialize according to embedded schema");

            output_json(output.as_ref(), &value);
        }
        Command::Extract { input, output } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let mut buf = &input_bytes as &[u8];

            let schema = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

            output_borsh(output.as_ref(), &schema, None);
        }
        Command::Strip { input, output } => {
            let input_bytes = get_input_bytes(input.as_ref());

            let mut buf = &input_bytes as &[u8];

            let _ = <BorshSchemaContainer as BorshDeserialize>::deserialize(&mut buf).unwrap();

            output_writer(output.as_ref()).write_all(buf).expect("Unable to write output");
        }
    }
}

#[cfg(test)]
#[allow(unused, dead_code)]
mod tests {
    use borsh::{BorshSchema, BorshSerialize};
    use serde::Serialize;

    use crate::{output_borsh, output_json};

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
