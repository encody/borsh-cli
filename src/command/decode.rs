use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
/// Decode Borsh input to JSON.
///
/// Requires the input to contain the embedded schema.
pub struct DecodeArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,
}
