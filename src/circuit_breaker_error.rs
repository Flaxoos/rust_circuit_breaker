use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub struct CircuitBreakerError {
  pub message: String,
  pub error_type: CircuitBreakerErrorType,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum CircuitBreakerErrorType {
  ErrorWrapper,
  Open,
  HalfOpen,
}

impl Debug for CircuitBreakerError {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "{}", self.message)
  }
}

impl Display for CircuitBreakerError {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "{}", self.message)
  }
}

impl Error for CircuitBreakerError {}
