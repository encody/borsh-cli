use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
/// Convert JSON to Borsh.
///
/// Note: If a schema is not specified, values that can be null (e.g. a
/// Rust Option<T>), etc. WILL NOT be serialized correctly.
pub struct EncodeArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,

    /// Schema to follow when serializing.
    #[arg(short, long)]
    pub schema: Option<PathBuf>,
}
