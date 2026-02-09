//! STEP Physical File parser.
//!
//! Consumes [`Token`]s from the lexer and produces a structured [`StepFile`].

use crate::step_lexer::Token;
use cst_core::{CstError, Result};

// ---------------------------------------------------------------------------
// AST types
// ---------------------------------------------------------------------------

/// A single attribute value inside a STEP entity.
#[derive(Debug, Clone, PartialEq)]
pub enum StepAttribute {
    Integer(i64),
    Real(f64),
    String(String),
    Bool(bool),
    Enum(String),
    EntityRef(u64),
    List(Vec<StepAttribute>),
    Null,
    Derived,
}

/// A parsed STEP entity, e.g. `#1 = IFCPROJECT(...)`.
#[derive(Debug, Clone)]
pub struct StepEntity {
    pub entity_id: u64,
    pub type_name: String,
    pub attributes: Vec<StepAttribute>,
}

/// Header section info (simplified).
#[derive(Debug, Clone, Default)]
pub struct StepHeader {
    pub description: Vec<String>,
    pub file_name: Vec<String>,
    pub file_schema: Vec<String>,
}

/// A complete parsed STEP file.
#[derive(Debug, Clone)]
pub struct StepFile {
    pub header: StepHeader,
    pub entities: Vec<StepEntity>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Result<&Token> {
        if self.pos >= self.tokens.len() {
            return Err(CstError::Parse("Unexpected end of tokens".into()));
        }
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        Ok(tok)
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<()> {
        match self.advance()? {
            Token::Keyword(k) if k == kw => Ok(()),
            other => Err(CstError::Parse(format!(
                "Expected keyword '{kw}', got {other:?}"
            ))),
        }
    }

    fn expect_semicolon(&mut self) -> Result<()> {
        match self.advance()? {
            Token::Semicolon => Ok(()),
            other => Err(CstError::Parse(format!(
                "Expected ';', got {other:?}"
            ))),
        }
    }

    /// Parse a complete STEP file.
    fn parse_file(&mut self) -> Result<StepFile> {
        // ISO-10303-21;
        self.expect_keyword("ISO-10303-21")?;
        self.expect_semicolon()?;

        // HEADER section
        let header = self.parse_header()?;

        // DATA section
        self.expect_keyword("DATA")?;
        self.expect_semicolon()?;

        let mut entities = Vec::new();
        while let Some(tok) = self.peek() {
            match tok {
                Token::Keyword(k) if k == "ENDSEC" => {
                    self.advance()?;
                    self.expect_semicolon()?;
                    break;
                }
                Token::EntityId(_) => {
                    entities.push(self.parse_entity()?);
                }
                _ => {
                    // Skip unrecognized tokens in data section
                    self.advance()?;
                }
            }
        }

        // END-ISO-10303-21;
        self.expect_keyword("END-ISO-10303-21")?;
        self.expect_semicolon()?;

        Ok(StepFile { header, entities })
    }

    /// Parse the HEADER section.
    fn parse_header(&mut self) -> Result<StepHeader> {
        self.expect_keyword("HEADER")?;
        self.expect_semicolon()?;

        let mut header = StepHeader::default();

        loop {
            match self.peek() {
                Some(Token::Keyword(k)) if k == "ENDSEC" => {
                    self.advance()?;
                    self.expect_semicolon()?;
                    break;
                }
                Some(Token::Keyword(k)) => {
                    let kw = k.clone();
                    self.advance()?;

                    // Collect all strings from this header entity's arguments
                    let strings = self.collect_header_strings()?;
                    self.expect_semicolon()?;

                    match kw.as_str() {
                        "FILE_DESCRIPTION" => header.description = strings,
                        "FILE_NAME" => header.file_name = strings,
                        "FILE_SCHEMA" => header.file_schema = strings,
                        _ => {} // ignore unknown header entries
                    }
                }
                _ => {
                    self.advance()?;
                }
            }
        }

        Ok(header)
    }

    /// Collect string values from a header entity like `FILE_NAME('a','b',...)`.
    fn collect_header_strings(&mut self) -> Result<Vec<String>> {
        let mut strings = Vec::new();
        // Expect opening paren
        let mut depth = 0i32;
        loop {
            match self.peek() {
                Some(Token::Semicolon) | None => break,
                Some(Token::OpenParen) => {
                    depth += 1;
                    self.advance()?;
                }
                Some(Token::CloseParen) => {
                    depth -= 1;
                    self.advance()?;
                    if depth <= 0 {
                        break;
                    }
                }
                Some(Token::String(_)) => {
                    if let Token::String(s) = self.advance()?.clone() {
                        strings.push(s);
                    }
                }
                _ => {
                    self.advance()?;
                }
            }
        }
        Ok(strings)
    }

    /// Parse a single entity: `#id = TYPE_NAME(attr, attr, ...);`
    fn parse_entity(&mut self) -> Result<StepEntity> {
        let entity_id = match self.advance()? {
            Token::EntityId(id) => *id,
            other => {
                return Err(CstError::Parse(format!(
                    "Expected entity id, got {other:?}"
                )))
            }
        };

        // =
        match self.advance()? {
            Token::Equals => {}
            other => {
                return Err(CstError::Parse(format!(
                    "Expected '=', got {other:?}"
                )))
            }
        }

        // TYPE_NAME
        let type_name = match self.advance()? {
            Token::Keyword(k) => k.clone(),
            other => {
                return Err(CstError::Parse(format!(
                    "Expected type keyword, got {other:?}"
                )))
            }
        };

        // (attributes)
        match self.advance()? {
            Token::OpenParen => {}
            other => {
                return Err(CstError::Parse(format!(
                    "Expected '(' after type name, got {other:?}"
                )))
            }
        }

        let attributes = self.parse_attribute_list()?;

        // closing )
        match self.advance()? {
            Token::CloseParen => {}
            other => {
                return Err(CstError::Parse(format!(
                    "Expected ')' closing entity, got {other:?}"
                )))
            }
        }

        self.expect_semicolon()?;

        Ok(StepEntity {
            entity_id,
            type_name,
            attributes,
        })
    }

    /// Parse a comma-separated list of attributes (inside parentheses).
    fn parse_attribute_list(&mut self) -> Result<Vec<StepAttribute>> {
        let mut attrs = Vec::new();

        // Empty attribute list: immediately followed by ')'
        if let Some(Token::CloseParen) = self.peek() {
            return Ok(attrs);
        }

        attrs.push(self.parse_attribute()?);

        while let Some(Token::Comma) = self.peek() {
            self.advance()?; // consume comma
            attrs.push(self.parse_attribute()?);
        }

        Ok(attrs)
    }

    /// Parse a single attribute value.
    fn parse_attribute(&mut self) -> Result<StepAttribute> {
        match self.peek() {
            Some(Token::Integer(_)) => {
                if let Token::Integer(v) = self.advance()?.clone() {
                    Ok(StepAttribute::Integer(v))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Real(_)) => {
                if let Token::Real(v) = self.advance()?.clone() {
                    Ok(StepAttribute::Real(v))
                } else {
                    unreachable!()
                }
            }
            Some(Token::String(_)) => {
                if let Token::String(s) = self.advance()?.clone() {
                    Ok(StepAttribute::String(s))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Bool(_)) => {
                if let Token::Bool(b) = self.advance()?.clone() {
                    Ok(StepAttribute::Bool(b))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Enum(_)) => {
                if let Token::Enum(e) = self.advance()?.clone() {
                    Ok(StepAttribute::Enum(e))
                } else {
                    unreachable!()
                }
            }
            Some(Token::EntityId(_)) => {
                if let Token::EntityId(id) = self.advance()?.clone() {
                    Ok(StepAttribute::EntityRef(id))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Null) => {
                self.advance()?;
                Ok(StepAttribute::Null)
            }
            Some(Token::Derived) => {
                self.advance()?;
                Ok(StepAttribute::Derived)
            }
            Some(Token::OpenParen) => {
                self.advance()?; // consume '('
                let items = self.parse_attribute_list()?;
                match self.advance()? {
                    Token::CloseParen => {}
                    other => {
                        return Err(CstError::Parse(format!(
                            "Expected ')' closing list, got {other:?}"
                        )))
                    }
                }
                Ok(StepAttribute::List(items))
            }
            other => Err(CstError::Parse(format!(
                "Unexpected token in attribute: {other:?}"
            ))),
        }
    }
}

/// Parse a STEP Physical File string into a structured [`StepFile`].
pub fn parse_step(input: &str) -> Result<StepFile> {
    let tokens = crate::step_lexer::tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_file()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_entity() {
        let input = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('test'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#100=IFCCARTESIANPOINT((0.,0.,0.));
ENDSEC;
END-ISO-10303-21;
"#;
        let file = parse_step(input).unwrap();
        assert_eq!(file.entities.len(), 1);
        let e = &file.entities[0];
        assert_eq!(e.entity_id, 100);
        assert_eq!(e.type_name, "IFCCARTESIANPOINT");
        // Attribute is a list of 3 reals
        assert_eq!(e.attributes.len(), 1);
        if let StepAttribute::List(coords) = &e.attributes[0] {
            assert_eq!(coords.len(), 3);
            assert_eq!(coords[0], StepAttribute::Real(0.0));
        } else {
            panic!("Expected list attribute");
        }
    }

    #[test]
    fn test_parse_mini_ifc() {
        let input = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('example.ifc','2024-01-01',('Author'),('Org'),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,(#6),#9);
#2=IFCSITE('3De0M8d3X5bvVph9lFED7f',$,'Site',$,$,#10,$,$,.ELEMENT.,$,$,$,$,$);
#100=IFCCARTESIANPOINT((0.,0.,0.));
#101=IFCDIRECTION((0.,0.,1.));
#200=IFCEXTRUDEDAREASOLID(#210,#220,#101,3000.);
ENDSEC;
END-ISO-10303-21;
"#;
        let file = parse_step(input).unwrap();
        assert_eq!(file.entities.len(), 5);

        // Check header
        assert!(file.header.file_schema.contains(&"IFC4".to_string()));

        // Check IFCPROJECT
        let proj = &file.entities[0];
        assert_eq!(proj.type_name, "IFCPROJECT");
        assert_eq!(proj.entity_id, 1);
        assert_eq!(proj.attributes[0], StepAttribute::String("0YvctVUKr0kugbFTf53O9L".into()));
        assert_eq!(proj.attributes[1], StepAttribute::Null);

        // Check IFCSITE enum
        let site = &file.entities[1];
        assert_eq!(site.type_name, "IFCSITE");
        assert_eq!(site.attributes[8], StepAttribute::Enum("ELEMENT".into()));

        // Check IFCEXTRUDEDAREASOLID
        let extrude = &file.entities[4];
        assert_eq!(extrude.type_name, "IFCEXTRUDEDAREASOLID");
        assert_eq!(extrude.attributes[0], StepAttribute::EntityRef(210));
        assert_eq!(extrude.attributes[3], StepAttribute::Real(3000.0));
    }

    #[test]
    fn test_parse_entity_with_list() {
        let input = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('guid',$,'Name',$,$,$,$,(#2,#3),#4);
ENDSEC;
END-ISO-10303-21;
"#;
        let file = parse_step(input).unwrap();
        let proj = &file.entities[0];
        if let StepAttribute::List(refs) = &proj.attributes[7] {
            assert_eq!(refs.len(), 2);
            assert_eq!(refs[0], StepAttribute::EntityRef(2));
            assert_eq!(refs[1], StepAttribute::EntityRef(3));
        } else {
            panic!("Expected list attribute");
        }
    }

    #[test]
    fn test_parse_bool_attributes() {
        let input = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCTEST(.T.,.F.);
ENDSEC;
END-ISO-10303-21;
"#;
        let file = parse_step(input).unwrap();
        let e = &file.entities[0];
        assert_eq!(e.attributes[0], StepAttribute::Bool(true));
        assert_eq!(e.attributes[1], StepAttribute::Bool(false));
    }
}
