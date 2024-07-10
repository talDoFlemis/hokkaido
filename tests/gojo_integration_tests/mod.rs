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
        NodeInfo::new(2, 1419, 1419, Color::Black),
        NodeInfo::new(3, 1537, 1537, Color::Red),
        NodeInfo::new(1, 1934, 1934, Color::Black),
        NodeInfo::new(4, 2493, 2493, Color::Red),
        NodeInfo::new(3, 2764, 2764, Color::Black),
        NodeInfo::new(4, 3158, 3158, Color::Red),
        NodeInfo::new(2, 3485, 3485, Color::Red),
        NodeInfo::new(3, 3850, 3850, Color::Black),
        NodeInfo::new(4, 4809, 4809, Color::Red),
        NodeInfo::new(0, 4872, 4872, Color::Black),
        NodeInfo::new(3, 4971, 4971, Color::Black),
        NodeInfo::new(2, 5398, 5398, Color::Red),
        NodeInfo::new(3, 6712, 6712, Color::Black),
        NodeInfo::new(4, 7382, 7382, Color::Red),
        NodeInfo::new(1, 7532, 7532, Color::Black),
        NodeInfo::new(3, 7610, 7610, Color::Black),
        NodeInfo::new(2, 8264, 8264, Color::Red),
        NodeInfo::new(4, 8420, 8420, Color::Red),
        NodeInfo::new(3, 8906, 8906, Color::Black),
        NodeInfo::new(4, 9627, 9627, Color::Red),
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

    // Assert
    for expected in expecteds {
        let item = iterator.next();
        assert!(item.is_some());
        let actual = item.unwrap();
        assert_eq!(expected.key, actual.key, "for item {}", expected.key);
        assert_eq!(expected.value, actual.value, "for item {}", expected.key);
        assert_eq!(expected.color, actual.color, "for item {}", expected.key);
        assert_eq!(expected.depth, actual.depth, "for item {}", expected.key);
    }

    Ok(())
}
