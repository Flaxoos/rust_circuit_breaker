use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// Error returned by [CircuitBreaker]
pub struct CircuitBreakerError<E: Display> {
  pub message: String,
  pub error_type: CircuitBreakerErrorType,
  pub cause: Option<E>,
}

impl<E: Display> CircuitBreakerError<E> {

  /// Returns a [CircuitBreakerError] indicating the [CircuitBreaker] is [CircuitBreakerErrorType::Open]
  pub fn open(failure_threshold: i8) -> Self {
    CircuitBreakerError{
      error_type: CircuitBreakerErrorType::Open,
      message : format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", failure_threshold),
      cause: None
    }
  }


  /// Returns a [CircuitBreakerError] indicating the [CircuitBreaker] is [CircuitBreakerErrorType::Closed]
  pub fn half_open(half_open_attempts: i8) -> Self {
    CircuitBreakerError {
      error_type: CircuitBreakerErrorType::HalfOpen,
      message: format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", half_open_attempts),
      cause: None
    }
  }

  /// Returns a [CircuitBreakerError] wrapping an error
  pub fn error_wrapper(error: E) -> Self {
    CircuitBreakerError {
      error_type: CircuitBreakerErrorType::ErrorWrapper,
      message: format!("Action failed {}", error),
      cause: Some(error),
    }
  }

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
