use clap::Parser;
use std::path::PathBuf;

/// A Program to parse a Konan AKA Search-Optimized Packed Memory Array
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Name of input file to read and process statements
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: Option<PathBuf>,

    /// Name of output file to write
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,

    /// Trailing newline
    #[arg(short, long)]
    pub new_line: bool,
}
