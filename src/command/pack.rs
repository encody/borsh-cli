use std::{io::Write, path::PathBuf};

use borsh::BorshSchema;
use clap::Args;

#[derive(Args, Debug)]
/// Serialize the input as a simple binary blob with Borsh headers.
pub struct PackArgs {
    /// Read input from this file, otherwise from stdin.
    pub input_path: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output_path: Option<PathBuf>,

    /// By default, the Borsh schema is included in the header. Enable this flag to remove it.
    #[arg(short, long)]
    pub no_schema: bool,
}

pub struct Pack {
    pub input: Vec<u8>,
    pub output: Box<dyn Write>,
    pub no_schema: bool,
}

impl TryFrom<&'_ PackArgs> for Pack {
    type Error = super::IOError;

    fn try_from(args: &'_ PackArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            input: super::get_input_bytes(args.input_path.as_ref())?,
            output: super::output_writer(args.output_path.as_ref())?,
            no_schema: args.no_schema,
        })
    }
}

impl super::Execute for Pack {
    fn execute(&mut self) -> Result<(), super::IOError> {
        if self.no_schema {
            super::output_borsh(&mut self.output, &self.input)
        } else {
            let schema = Vec::<u8>::schema_container();
            super::output_borsh(&mut self.output, &(schema, &self.input))
        }
    }
}
