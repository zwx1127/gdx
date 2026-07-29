use std::fmt;

use serde_json::Value;

pub type GdxResult<T> = Result<T, GdxError>;

#[derive(Debug, Clone)]
pub struct GdxError {
    pub error: String,
    pub message: String,
    pub exit_code: i32,
    pub context: Box<GdxErrorContext>,
}

#[derive(Debug, Clone, Default)]
pub struct GdxErrorContext {
    pub suggestion: Option<String>,
    pub details: Option<Value>,
    pub command: Option<String>,
    pub run_id: Option<String>,
    pub stage: Option<String>,
    pub subject: Option<String>,
    pub child_pid: Option<u32>,
    pub elapsed_ms: Option<u128>,
    pub timeout_secs: Option<u64>,
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

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new("cancelled", message, 130)
    }

    pub fn not_found(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(error, message, 5)
    }

    pub fn new(error: impl Into<String>, message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            exit_code,
            context: Box::default(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.context.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.context.details = Some(details);
        self
    }

    pub fn insert_detail(&mut self, key: &str, value: Value) {
        match &mut self.context.details {
            Some(details) if details.is_object() => details[key] = value,
            Some(details) => {
                *details = serde_json::json!({ "context": details.clone(), key: value });
            }
            None => self.context.details = Some(serde_json::json!({ key: value })),
        }
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.context.stage = Some(stage.into());
        self
    }

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.context.subject = Some(subject.into());
        self
    }

    pub fn with_child_pid(mut self, pid: u32) -> Self {
        self.context.child_pid = Some(pid);
        self
    }

    pub fn with_elapsed_ms(mut self, elapsed_ms: u128) -> Self {
        self.context.elapsed_ms = Some(elapsed_ms);
        self
    }

    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.context.timeout_secs = Some(timeout_secs);
        self
    }
}

impl fmt::Display for GdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl std::error::Error for GdxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_keeps_optional_context_out_of_result_layout() {
        assert!(std::mem::size_of::<GdxError>() <= 64);
    }
}
