//! Storage failures that callers can map to HTTP statuses.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    NotFound(String),
    Conflict(String),
    Invalid(String),
    Internal(String),
}

impl StorageError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Invalid(message)
            | Self::Internal(message) => formatter.write_str(message),
        }
    }
}
