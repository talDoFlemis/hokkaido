use clap::Parser;
use std::path::PathBuf;

/// A Program to parse a Gojo Tree AKA Partial Persistence Red Black Tree
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Name of input file to read and process statements
    #[arg(short, long, value_name = "INPUT_FILE")]
    pub input: Option<PathBuf>,

    /// Name of output file to write
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,
}
