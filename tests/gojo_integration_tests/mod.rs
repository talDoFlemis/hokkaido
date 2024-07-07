use hokkaido::gojo::{
    self,
    parser::{Parser, ParserVagaba},
    Gojo,
};

use anyhow::{bail, Result};

#[test]
fn only_insert_and_print() -> Result<()> {
    // Arrange
    let str = include_str!("./inputs/01.txt");
    let p = ParserVagaba::default();
    let mut gojo: Gojo<i32, i32> = Gojo::default();

    let stms = p.parse_lines(str)?;

    for stm in stms {
        match stm {
            gojo::parser::Statement::Insert(value) => gojo.insert(value, value),
            _ => bail!("Should not come here"),
        }
    }

    for info in gojo.node_info_iter(42)? {
        println!("{info:?}");
    }

    Ok(())
}
