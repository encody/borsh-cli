use clap::Parser;

mod command;
mod dynamic_schema;
mod json_borsh;

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for manipulating Borsh-serialized data
///
/// Note: Does not play particularly nicely with `HashMap<_, _>` types in
/// schema.
struct Args {
    #[command(subcommand)]
    command: command::Command,
}

fn main() {
    Args::parse().command.run();
}
