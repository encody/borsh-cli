use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

use borsh::{BorshDeserialize, BorshSerialize};
use clap::{Parser, Subcommand};

use crate::json_borsh::JsonSerializableAsBorsh;

mod json_borsh;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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
    },
    /// Deserialize the input as a simple binary blob with Borsh headers.
    Unpack {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,
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
        schema: Option<String>,
    },
    /// NOT IMPLEMENTED -- Decode Borsh input to JSON.
    Decode {
        /// Read input from this file if STDIN is empty.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Write output this file, otherwise to STDOUT.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Schema to follow when deserializing.
        #[arg(short, long)]
        schema: String,
    },
}

fn input_bytes(input_path: Option<&PathBuf>) -> Vec<u8> {
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

fn output_borsh(output: Option<&PathBuf>, value: impl BorshSerialize) {
    if let Some(o) = output {
        let path = o.display();
        let f =
            fs::File::create(o).unwrap_or_else(|_| panic!("Could not create output file {path}"));
        borsh::to_writer(f, &value)
            .unwrap_or_else(|_| panic!("Could not write to output file {path}"));
    } else {
        borsh::to_writer(io::stdout(), &value).expect("Could not write to STDOUT");
    }
}

fn main() {
    let args = Args::parse();

    match &args.command {
        Command::Pack { input, output } => {
            let input_bytes = input_bytes(input.as_ref());

            output_borsh(output.as_ref(), input_bytes);
        }
        Command::Unpack { input, output } => {
            let input_bytes = input_bytes(input.as_ref());

            let value = Vec::<u8>::try_from_slice(&input_bytes)
                .expect("Could not read input as byte array");

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
            if schema.is_some() {
                todo!("schema is not supported");
            }

            let input_bytes = input_bytes(input.as_ref());

            let v = serde_json::from_slice::<serde_json::Value>(&input_bytes)
                .expect("Could not parse input as JSON");

            let v = JsonSerializableAsBorsh(&v);

            output_borsh(output.as_ref(), v);
        }
        Command::Decode { .. } => todo!(),
    }
}

#[cfg(test)]
mod tests {

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
