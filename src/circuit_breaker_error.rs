use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// Error returned by [CircuitBreaker]
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum CircuitBreakerError<E: Display> {
  Wrapped(E),
  Open { threshold: i8 },
  HalfOpen { threshold: i8 },
}

impl<E> Display for CircuitBreakerError<E>
where
  E: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
        CircuitBreakerError::Wrapped(e) => write!(f, "Action failed {}", e),
        CircuitBreakerError::Open { threshold: error_count } => write!(f, "Action failed more than {} times, subsequent calls will be prevented until action is successful again", error_count),
        CircuitBreakerError::HalfOpen { threshold: error_count } => write!(f, "Action failed more than {} times, subsequent calls will be prevented until action is successful again", error_count),
      }
  }
}

impl<E> Error for CircuitBreakerError<E>
where
  E: Error,
{
  // if we wrapped an error, we should override the default implementation of [cause]
  // to provide that wrapped error
  fn cause(&self) -> Option<&dyn Error> {
    match self {
      CircuitBreakerError::Wrapped(e) => Some(e),
      _ => None,
    }
  }
}
