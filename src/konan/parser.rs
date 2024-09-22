use anyhow::{Ok, Result};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Statement {
    Insert(i32),
    Print,
    Remove(i32),
    Successor(i32),
}

pub trait Parser {
    fn parse_lines(&self, s: &str) -> Result<Vec<Statement>>;
    fn parse_line(&self, s: &str) -> Result<Statement>;
}

pub struct ParserVagaba {}

impl ParserVagaba {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ParserVagaba {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for ParserVagaba {
    fn parse_lines(&self, s: &str) -> Result<Vec<Statement>> {
        let mut vec: Vec<Statement> = Vec::new();

        for line in s.lines() {
            let stm = self.parse_line(line)?;
            vec.push(stm);
        }

        Ok(vec)
    }

    fn parse_line(&self, s: &str) -> Result<Statement> {
        let tokens: Vec<&str> = s.split_whitespace().collect();
        if tokens.is_empty() || tokens.len() > 2 {
            anyhow::bail!("Passando parametro de menos ou mais doto");
        }

        let stm = tokens[0];

        if tokens.len() == 1 {
            if stm.to_lowercase() != "imp" {
                anyhow::bail!("Esperado imprimir");
            }

            return Ok(Statement::Print);
        }

        let value: i32 = tokens[1].parse()?;
        match stm.to_lowercase().as_str() {
            "inc" => Ok(Statement::Insert(value)),
            "rem" => Ok(Statement::Remove(value)),
            "suc" => Ok(Statement::Successor(value)),
            e => anyhow::bail!("Não esperado esse caba {}", e),
        }
    }
}

#[cfg(test)]
mod parser_vagaba_tests {
    use pretty_assertions::assert_eq;

    use crate::konan::parser::{Parser, ParserVagaba, Statement};
    use anyhow::Result;

    #[test]
    fn test_parse_insert_statement() -> Result<()> {
        // Arrange
        let s = "INC 14";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Insert(14);

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_remove_statement() -> Result<()> {
        // Arrange
        let s = "REM 14";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Remove(14);

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_print_statement() -> Result<()> {
        // Arrange
        let s = "IMP";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Print;

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_successor_statement() -> Result<()> {
        // Arrange
        let s = "SUC 14";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Successor(14);

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_lines() -> Result<()> {
        // Arrange
        let s = "SUC 420\nINC 69\nIMP\nREM 777";
        let p = ParserVagaba::new();
        let expected_stms = Vec::from([
            Statement::Successor(420) ,
            Statement::Insert(69),
            Statement::Print,
            Statement::Remove(777),
        ]);

        // Act
        let actual_stms = p.parse_lines(s)?;

        //Assert
        assert_eq!(expected_stms, actual_stms);

        Ok(())
    }

    #[test]
    fn test_cant_parse_unknown_tree_tokens() {
        // Arrange
        let s = "TUBIAS 24";
        let p = ParserVagaba::new();

        // Act
        let err = p.parse_line(s);

        //Assert
        assert!(err.is_err());
    }

    #[test]
    fn test_cant_parse_unknown_one_tokens() {
        // Arrange
        let s = "GARGAMEL";
        let p = ParserVagaba::new();

        // Act
        let err = p.parse_line(s);

        //Assert
        assert!(err.is_err());
    }
}
