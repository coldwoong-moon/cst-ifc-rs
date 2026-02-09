use thiserror::Error;

#[derive(Debug, Error)]
pub enum CstError {
    #[error("Topology error: {0}")]
    Topology(String),

    #[error("Geometry error: {0}")]
    Geometry(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tolerance violation: {0}")]
    Tolerance(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, CstError>;
