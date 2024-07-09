use hokkaido::gojo::{
    self,
    parser::{Parser, ParserVagaba},
    Color, Gojo, NodeInfo,
};

use anyhow::{bail, Result};

#[test]
fn only_insert_and_print() -> Result<()> {
    // Arrange
    let str = include_str!("./inputs/01.txt");
    let p = ParserVagaba::default();
    let mut gojo: Gojo<i32, i32> = Gojo::default();
    let expecteds = [
        NodeInfo::new(3, 1419, 1419, Color::Black),
        NodeInfo::new(2, 1537, 1537, Color::Black),
        NodeInfo::new(3, 1934, 1934, Color::Black),
        NodeInfo::new(1, 2493, 2493, Color::Red),
        NodeInfo::new(3, 2764, 2764, Color::Black),
        NodeInfo::new(2, 3158, 3158, Color::Black),
        NodeInfo::new(3, 3485, 3485, Color::Black),
        NodeInfo::new(0, 3850, 3850, Color::Black),
        NodeInfo::new(3, 4809, 4809, Color::Black),
        NodeInfo::new(2, 4872, 4872, Color::Black),
    ];

    // Act
    let stms = p.parse_lines(str)?;

    for stm in stms {
        match stm {
            gojo::parser::Statement::Insert(value) => gojo.insert(value, value),
            _ => bail!("Should not come here"),
        }
    }

    let mut iterator = gojo.node_info_iter(gojo.latest_version())?;

    Gojo::print_in_order(gojo.root, gojo.latest_version());

    // Assert
    for expected in expecteds {
        let item = iterator.next();
        println!("item {item:?}");
        assert!(item.is_some());
        let actual = item.unwrap();
        assert_eq!(expected.key, actual.key, "for item {}", actual.key);
        assert_eq!(expected.value, actual.value, "for item {}", actual.key);
        assert_eq!(expected.color, actual.color, "for item {}", actual.key);
        assert_eq!(expected.depth, actual.depth, "for item {}", actual.key);
    }

    Ok(())
}
