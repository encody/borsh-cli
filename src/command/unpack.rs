use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
/// Deserialize the input as a simple binary blob with Borsh headers.
pub struct UnpackArgs {
    /// Read input from this file, otherwise from stdin.
    pub input: Option<PathBuf>,

    /// Write output to this file, otherwise to stdout.
    pub output: Option<PathBuf>,

    /// By default, we assume the Borsh schema is included in the header. Enable this flag to prevent this.
    #[arg(short, long)]
    pub no_schema: bool,
}
