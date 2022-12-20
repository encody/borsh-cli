use std::{io::Write, path::PathBuf};

use borsh::{schema::BorshSchemaContainer, BorshDeserialize, BorshSchema};
use clap::Args;

use super::{output_bytes, IOError};

#[derive(Args, Debug)]
/// Deserialize the input as a simple binary blob with Borsh headers.
pub struct UnpackArgs {
    /// Read input from this file, otherwise from stdin.
    pub input_path: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output_path: Option<PathBuf>,

    /// By default, we assume the Borsh schema is included in the header. Enable this flag to prevent this.
    #[arg(short, long)]
    pub no_schema: bool,
}

pub struct Unpack<'a> {
    pub input: Vec<u8>,
    pub output: Box<dyn Write + 'a>,
    pub no_schema: bool,
}

impl TryFrom<&'_ UnpackArgs> for Unpack<'_> {
    type Error = super::IOError;
    fn try_from(args: &'_ UnpackArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            input: super::get_input_bytes(args.input_path.as_ref())?,
            output: super::output_writer(args.output_path.as_ref())?,
            no_schema: args.no_schema,
        })
    }
}

impl super::Execute for Unpack<'_> {
    fn execute(&mut self) -> Result<(), super::IOError> {
        let value = if self.no_schema {
            Vec::<u8>::try_from_slice(&self.input)
                .map_err(|_| IOError::DeserializeBorsh("byte array"))?
        } else {
            let (schema, v) = <(BorshSchemaContainer, Vec<u8>)>::try_from_slice(&self.input)
                .map_err(|_| IOError::DeserializeBorsh("byte array with schema headers"))?;
            if schema != Vec::<u8>::schema_container() {
                return Err(IOError::IncorrectBorshSchemaHeader(schema.declaration));
            }
            v
        };

        output_bytes(&mut self.output, &value)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use crate::command::Execute;

    use super::Unpack;

    #[test]
    fn with_schema() {
        let test_vector = vec![1u8, 2, 3, 4];
        let mut output_vector: Vec<u8> = vec![];
        let writer = BufWriter::new(&mut output_vector);

        let mut p = Unpack {
            input: borsh::try_to_vec_with_schema(&test_vector).unwrap(),
            output: Box::new(writer),
            no_schema: false,
        };

        p.execute().unwrap();
        drop(p);

        assert_eq!(test_vector, output_vector);
    }

    #[test]
    fn without_schema() {
        let test_vector = vec![1u8, 2, 3, 4];
        let mut output_vector: Vec<u8> = vec![];
        let writer = BufWriter::new(&mut output_vector);

        let mut p = Unpack {
            input: borsh::to_vec(&test_vector).unwrap(),
            output: Box::new(writer),
            no_schema: true,
        };

        p.execute().unwrap();
        drop(p);

        assert_eq!(test_vector, output_vector);
    }
}
