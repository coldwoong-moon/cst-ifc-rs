//! STEP Physical File (SPF) lexer / tokenizer.
//!
//! Converts raw IFC text into a flat stream of [`Token`]s that the parser consumes.

use cst_core::{CstError, Result};

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

/// A single lexical token from a STEP Physical File.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Entity instance id, e.g. `#123`
    EntityId(u64),
    /// Upper-case keyword, e.g. `IFCWALL`, `FILE_DESCRIPTION`
    Keyword(String),
    /// Single-quoted string literal, e.g. `'hello'`
    String(String),
    /// Integer literal, e.g. `42`, `-7`
    Integer(i64),
    /// Real (floating-point) literal, e.g. `3.14`, `1.5E-3`
    Real(f64),
    /// Enumeration value, e.g. `.ELEMENT.`
    Enum(String),
    /// Boolean `.T.` or `.F.`
    Bool(bool),
    /// Derived attribute `*`
    Derived,
    /// Null / omitted attribute `$`
    Null,
    // Delimiters
    OpenParen,
    CloseParen,
    Comma,
    Semicolon,
    Equals,
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

/// Tokenize a STEP Physical File string into a vector of tokens.
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut pos: usize = 0;
    let mut tokens = Vec::new();

    while pos < len {
        // Skip whitespace
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        // Skip C-style block comments  /* ... */
        if pos + 1 < len && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
            pos += 2;
            while pos + 1 < len && !(bytes[pos] == b'*' && bytes[pos + 1] == b'/') {
                pos += 1;
            }
            if pos + 1 < len {
                pos += 2; // skip */
            }
            continue;
        }

        match bytes[pos] {
            b'(' => {
                tokens.push(Token::OpenParen);
                pos += 1;
            }
            b')' => {
                tokens.push(Token::CloseParen);
                pos += 1;
            }
            b',' => {
                tokens.push(Token::Comma);
                pos += 1;
            }
            b';' => {
                tokens.push(Token::Semicolon);
                pos += 1;
            }
            b'=' => {
                tokens.push(Token::Equals);
                pos += 1;
            }
            b'$' => {
                tokens.push(Token::Null);
                pos += 1;
            }
            b'*' => {
                tokens.push(Token::Derived);
                pos += 1;
            }

            // Entity id: #<digits>
            b'#' => {
                pos += 1;
                let start = pos;
                while pos < len && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                if start == pos {
                    return Err(CstError::Parse(
                        "Expected digits after '#'".into(),
                    ));
                }
                let id: u64 = input[start..pos]
                    .parse()
                    .map_err(|e| CstError::Parse(format!("Invalid entity id: {e}")))?;
                tokens.push(Token::EntityId(id));
            }

            // String literal: '...'  ('' is escaped single-quote inside)
            b'\'' => {
                pos += 1;
                let mut s = std::string::String::new();
                loop {
                    if pos >= len {
                        return Err(CstError::Parse("Unterminated string literal".into()));
                    }
                    if bytes[pos] == b'\'' {
                        // Check for escaped ''
                        if pos + 1 < len && bytes[pos + 1] == b'\'' {
                            s.push('\'');
                            pos += 2;
                        } else {
                            pos += 1; // closing quote
                            break;
                        }
                    } else {
                        s.push(bytes[pos] as char);
                        pos += 1;
                    }
                }
                tokens.push(Token::String(s));
            }

            // Enum or Bool: .XXX.
            b'.' => {
                pos += 1;
                let start = pos;
                while pos < len && bytes[pos] != b'.' {
                    pos += 1;
                }
                if pos >= len {
                    return Err(CstError::Parse("Unterminated enum value".into()));
                }
                let val = &input[start..pos];
                pos += 1; // skip closing '.'
                match val {
                    "T" => tokens.push(Token::Bool(true)),
                    "F" => tokens.push(Token::Bool(false)),
                    _ => tokens.push(Token::Enum(val.to_string())),
                }
            }

            // Number (integer or real), possibly negative
            c if c.is_ascii_digit() || c == b'-' || c == b'+' => {
                let start = pos;
                if c == b'-' || c == b'+' {
                    pos += 1;
                }
                // digits
                while pos < len && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                let mut is_real = false;
                // fractional part
                if pos < len && bytes[pos] == b'.' {
                    // Distinguish `0.` (real) from `.ENUM.`
                    // If digit follows or nothing, it's a real
                    is_real = true;
                    pos += 1;
                    while pos < len && bytes[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }
                // exponent
                if pos < len && (bytes[pos] == b'E' || bytes[pos] == b'e') {
                    is_real = true;
                    pos += 1;
                    if pos < len && (bytes[pos] == b'+' || bytes[pos] == b'-') {
                        pos += 1;
                    }
                    while pos < len && bytes[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }

                let text = &input[start..pos];
                if is_real {
                    let v: f64 = text
                        .parse()
                        .map_err(|e| CstError::Parse(format!("Invalid real: {e}")))?;
                    tokens.push(Token::Real(v));
                } else {
                    let v: i64 = text
                        .parse()
                        .map_err(|e| CstError::Parse(format!("Invalid integer: {e}")))?;
                    tokens.push(Token::Integer(v));
                }
            }

            // Keyword: upper-case letters, digits, underscore, hyphen
            c if c.is_ascii_alphabetic() => {
                let start = pos;
                while pos < len
                    && (bytes[pos].is_ascii_alphanumeric()
                        || bytes[pos] == b'_'
                        || bytes[pos] == b'-')
                {
                    pos += 1;
                }
                let word = input[start..pos].to_uppercase();
                tokens.push(Token::Keyword(word));
            }

            other => {
                return Err(CstError::Parse(format!(
                    "Unexpected character '{}' at position {pos}",
                    other as char
                )));
            }
        }
    }

    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id() {
        let tokens = tokenize("#123").unwrap();
        assert_eq!(tokens, vec![Token::EntityId(123)]);
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize("'hello world'").unwrap();
        assert_eq!(tokens, vec![Token::String("hello world".into())]);
    }

    #[test]
    fn test_escaped_string() {
        let tokens = tokenize("'it''s'").unwrap();
        assert_eq!(tokens, vec![Token::String("it's".into())]);
    }

    #[test]
    fn test_integer() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens, vec![Token::Integer(42)]);
    }

    #[test]
    fn test_negative_integer() {
        let tokens = tokenize("-7").unwrap();
        assert_eq!(tokens, vec![Token::Integer(-7)]);
    }

    #[test]
    fn test_real() {
        let tokens = tokenize("3.14").unwrap();
        assert_eq!(tokens, vec![Token::Real(3.14)]);
    }

    #[test]
    fn test_real_exponent() {
        let tokens = tokenize("1.5E-3").unwrap();
        assert_eq!(tokens, vec![Token::Real(1.5e-3)]);
    }

    #[test]
    fn test_real_trailing_dot() {
        // In STEP files, `0.` is a valid real
        let tokens = tokenize("0.").unwrap();
        assert_eq!(tokens, vec![Token::Real(0.0)]);
    }

    #[test]
    fn test_enum() {
        let tokens = tokenize(".ELEMENT.").unwrap();
        assert_eq!(tokens, vec![Token::Enum("ELEMENT".into())]);
    }

    #[test]
    fn test_bool_true() {
        let tokens = tokenize(".T.").unwrap();
        assert_eq!(tokens, vec![Token::Bool(true)]);
    }

    #[test]
    fn test_bool_false() {
        let tokens = tokenize(".F.").unwrap();
        assert_eq!(tokens, vec![Token::Bool(false)]);
    }

    #[test]
    fn test_null_and_derived() {
        let tokens = tokenize("$ *").unwrap();
        assert_eq!(tokens, vec![Token::Null, Token::Derived]);
    }

    #[test]
    fn test_delimiters() {
        let tokens = tokenize("()=,;").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::OpenParen,
                Token::CloseParen,
                Token::Equals,
                Token::Comma,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn test_keyword() {
        let tokens = tokenize("IFCWALL").unwrap();
        assert_eq!(tokens, vec![Token::Keyword("IFCWALL".into())]);
    }

    #[test]
    fn test_cartesian_point() {
        let tokens = tokenize("#100=IFCCARTESIANPOINT((0.,0.,0.));").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::EntityId(100),
                Token::Equals,
                Token::Keyword("IFCCARTESIANPOINT".into()),
                Token::OpenParen,
                Token::OpenParen,
                Token::Real(0.0),
                Token::Comma,
                Token::Real(0.0),
                Token::Comma,
                Token::Real(0.0),
                Token::CloseParen,
                Token::CloseParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn test_mixed_line() {
        let input = "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,(#6),#9);";
        let tokens = tokenize(input).unwrap();
        // Should start with EntityId(1), Equals, Keyword(IFCPROJECT), OpenParen ...
        assert_eq!(tokens[0], Token::EntityId(1));
        assert_eq!(tokens[1], Token::Equals);
        assert_eq!(tokens[2], Token::Keyword("IFCPROJECT".into()));
        assert_eq!(tokens[3], Token::OpenParen);
        assert_eq!(tokens[4], Token::String("0YvctVUKr0kugbFTf53O9L".into()));
    }
}
