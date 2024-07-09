use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};
const INFINITE_KEYWORD: &str = "INFINITO";

use anyhow::Result;
use clap::Parser;
use hokkaido::gojo::{
    cli::Cli,
    parser::{self, Parser as _, Statement},
    Color, Gojo,
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

fn convert_color_to_str(c: Color) -> String {
    match c {
        Color::Red => String::from("R"),
        Color::Black => String::from("N"),
    }
}

fn process_statements(stms: Vec<Statement>) -> Result<String> {
    let mut gojo: Gojo<i32, i32> = Gojo::default();
    let mut str_list: Vec<String> = Vec::new();

    for stm in stms {
        match stm {
            parser::Statement::Insert(value) => {
                gojo.insert(value, value);
            }
            parser::Statement::Remove(value) => {
                gojo.college_remove(&value);
            }
            parser::Statement::Successor { value, version } => {
                str_list.push(format!("SUC {value} {version}"));

                let real_version = if version > gojo.latest_version() {
                    gojo.latest_version()
                } else {
                    version
                };

                match gojo.successor(&value, real_version) {
                    Some(succ) => str_list.push(format!("{succ}")),
                    None => str_list.push(INFINITE_KEYWORD.to_string()),
                }
            }
            parser::Statement::Print(version) => {
                str_list.push(format!("IMP {version}"));

                let real_version = if version > gojo.latest_version() {
                    gojo.latest_version()
                } else {
                    version
                };

                let mut list: Vec<String> =
                    Vec::with_capacity(gojo.len(real_version).expect("Should not come here"));

                for info in gojo.node_info_iter(real_version)? {
                    list.push(format!(
                        "{},{},{}",
                        info.value,
                        info.depth,
                        convert_color_to_str(info.color)
                    ));
                }

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
