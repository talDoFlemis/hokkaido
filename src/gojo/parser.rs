use anyhow::{Ok, Result};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Statement {
    Insert(i32),
    Print(usize),
    Remove(i32),
    Successor { value: i32, version: usize },
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
        if tokens.len() < 2 || tokens.len() > 3 {
            anyhow::bail!("Passando parametro de menos ou mais doto");
        }

        let stm = tokens[0];

        if tokens.len() == 3 {
            let value: i32 = tokens[1].parse()?;
            let version: usize = tokens[2].parse()?;
            if stm.to_lowercase() != "suc" {
                anyhow::bail!("Esperado sucessor");
            }

            return Ok(Statement::Successor { value, version });
        }

        match stm.to_lowercase().as_str() {
            "inc" => {
                let value: i32 = tokens[1].parse()?;
                Ok(Statement::Insert(value))
            }
            "rem" => {
                let value: i32 = tokens[1].parse()?;
                Ok(Statement::Remove(value))
            }
            "imp" => {
                let version: usize = tokens[1].parse()?;
                Ok(Statement::Print(version))
            }
            e => anyhow::bail!("Não esperado esse caba {}", e),
        }
    }
}

#[cfg(test)]
mod parser_vagaba_tests {
    use pretty_assertions::assert_eq;

    use crate::gojo::parser::{Parser, ParserVagaba, Statement};
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
        let s = "IMP 14";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Print(14);

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_successor_statement() -> Result<()> {
        // Arrange
        let s = "SUC 14 1";
        let p = ParserVagaba::new();
        let expected_stm = Statement::Successor {
            value: 14,
            version: 1,
        };

        // Act
        let actual_stm = p.parse_line(s)?;

        //Assert
        assert_eq!(expected_stm, actual_stm);

        Ok(())
    }

    #[test]
    fn test_parse_lines() -> Result<()> {
        // Arrange
        let s = "SUC 420 69\nINC 69\nIMP 420\nREM 777";
        let p = ParserVagaba::new();
        let expected_stms = Vec::from([
            Statement::Successor {
                value: 420,
                version: 69,
            },
            Statement::Insert(69),
            Statement::Print(420),
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
        let s = "TUBIAS 14 1";
        let p = ParserVagaba::new();

        // Act
        let err = p.parse_line(s);

        //Assert
        assert!(err.is_err());
    }

    #[test]
    fn test_cant_parse_unknown_two_tokens() {
        // Arrange
        let s = "GARGAMEL 24";
        let p = ParserVagaba::new();

        // Act
        let err = p.parse_line(s);

        //Assert
        assert!(err.is_err());
    }
}
