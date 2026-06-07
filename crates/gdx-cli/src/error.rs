use std::collections::BTreeMap;
use std::fmt;

pub type GdxResult<T> = Result<T, GdxError>;

#[derive(Debug, Clone)]
pub struct GdxError {
    pub error: String,
    pub message: String,
    pub exit_code: i32,
    pub suggestion: Option<String>,
    pub artifacts: BTreeMap<String, String>,
}

impl GdxError {
    pub fn user(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(error, message, 1)
    }

    pub fn tool(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(error, message, 2)
    }

    pub fn validation(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(error, message, 3)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new("timeout", message, 4)
    }

    pub fn not_found(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(error, message, 5)
    }

    pub fn new(error: impl Into<String>, message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            exit_code,
            suggestion: None,
            artifacts: BTreeMap::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_artifact(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.artifacts.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for GdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl std::error::Error for GdxError {}
