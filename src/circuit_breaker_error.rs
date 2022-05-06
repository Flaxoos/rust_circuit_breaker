use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub struct CircuitBreakerError {
  pub message: String,
  pub error_type: CircuitBreakerErrorType,
}

impl CircuitBreakerError {
  fn format_message(&self) -> String {
    format!("{}, {}", self.error_type, self.message)
  }
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum CircuitBreakerErrorType {
  ErrorWrapper,
  Open,
  HalfOpen,
}

impl Display for CircuitBreakerErrorType {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "{}", self)
  }
}
