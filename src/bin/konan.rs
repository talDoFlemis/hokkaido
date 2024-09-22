use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};
const INFINITE_KEYWORD: &str = "INFINITO";

use anyhow::Result;
use clap::Parser;
use hokkaido::konan::{
    cli::Cli,
    parser::{self, Parser as _, Statement},
    Konan,
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
    let mut konan: Konan<i32> = Konan::default();
    let mut str_list: Vec<String> = Vec::new();

    for stm in stms {
        match stm {
            parser::Statement::Insert(value) => {
                konan.insert(value);
            }
            parser::Statement::Remove(value) => {
                konan.remove(&value);
            }
            parser::Statement::Successor(value) => match konan.successor(&value) {
                Some(succ) => str_list.push(format!("{succ}")),
                None => str_list.push(INFINITE_KEYWORD.to_string()),
            },
            parser::Statement::Print => {
                let list: Vec<String> = konan.iter().map(|x| x.to_string()).collect();
                let res = list.join(" ");
                str_list.push(res);
            }
        }
    }

    let res = str_list.join("\n");

    Ok(res)
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

    let mut output_string = process_statements(stms)?;

    if cli.new_line {
        output_string.push('\n');
    }
    writer.write_all(output_string.as_bytes())?;

    Ok(())
}
