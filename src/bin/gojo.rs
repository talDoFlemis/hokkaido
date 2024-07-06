use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};
const INFINITE_KEYWORD: &'static str = "INFINITO";

use anyhow::Result;
use clap::Parser;
use hokkaido::gojo::{
    cli::Cli,
    parser::{self, Parser as _, Statement},
    Gojo,
};

fn read_from_stdin(buf: &mut String) -> Result<()> {
    let mut stdin = io::stdin();
    stdin.read_to_string(buf)?;

    Ok(())
}

fn read_from_file(buf: &mut String, path: PathBuf) -> Result<()> {
    let mut f = File::open(path)?;
    f.read_to_string(buf)?;

    Ok(())
}

fn process_statements(stms: Vec<Statement>) -> Result<String> {
    let mut gojo: Gojo<i32, i32> = Gojo::default();
    let mut s = String::new();

    for stm in stms {
        match stm {
            parser::Statement::Insert(value) => {
                gojo.insert(value, value);
            }
            parser::Statement::Remove(value) => {
                gojo.remove(&value);
            }
            parser::Statement::Successor { value, version } => {
                s.push_str(&format!("SUC {value} {version}\n"));

                let real_version = if version > gojo.latest_version() {
                    gojo.latest_version()
                } else {
                    version
                };

                match gojo.successor(&value, real_version) {
                    Some(succ) => s.push_str(&format!("{succ}\n")),
                    None => s.push_str(&format!("{INFINITE_KEYWORD}\n")),
                }
            }
            parser::Statement::Print(version) => {
                s.push_str(&format!("IMP {version}\n"));

                let real_version = if version > gojo.latest_version() {
                    gojo.latest_version()
                } else {
                    version
                };

                // TODO: use iterators
                let res = String::new();
                s.push_str(&res);
                s.push('\n');
            }
        }
    }

    Ok(s)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut buf = String::new();

    match cli.input {
        Some(path) => read_from_file(&mut buf, path)?,
        None => read_from_stdin(&mut buf)?,
    }

    let mut writer: Box<dyn Write>;

    writer = match cli.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };

    let parser = parser::ParserVagaba::default();
    let stms = parser.parse_lines(&buf)?;

    let output_string = process_statements(stms)?;
    writer.write_all(&mut output_string.as_bytes())?;

    Ok(())
}
