use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
/// Remove the Borsh schema header.
pub struct StripArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,
}
