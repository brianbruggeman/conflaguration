extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

/// One field-specific validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    /// Dotted field path, such as `database.workers`.
    pub path: String,
    /// The failed rule, such as `must be > 0`.
    pub message: String,
}

impl ValidationMessage {
    /// Retains a field path and failure reason in alloc-backed storage.
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Typed collection of field-specific validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Failures retained in validation order.
    pub errors: Vec<ValidationMessage>,
}

impl ValidationError {
    /// Retains one failure without requiring a caller-owned vector.
    pub fn single(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![ValidationMessage::new(path, message)],
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("validation failed:")?;
        for error in &self.errors {
            write!(formatter, "\n  - {}: {}", error.path, error.message)?;
        }
        Ok(())
    }
}

impl core::error::Error for ValidationError {}

/// Checks one settings value and returns retained field failures.
pub trait Validate {
    /// Returns each failed field rule as a typed error.
    fn validate(&self) -> Result<(), ValidationError>;
}
