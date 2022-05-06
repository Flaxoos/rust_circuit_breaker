use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

//I've made quite a few changes to CircuitBreakerError here
//
// First off, I've merged CircuitBreakerError and CircuitBreakerErrorType.
// Rust's enums are powerful. I don't know of any direct equivelent in Java.
// Java's enums are definately not it.
// By switching CircuitBreakerError to being an enum I can store info specific to that type of error.
// So I replaced the generic msg field with specifics on the type of error
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum CircuitBreakerError<E> {
    // For the ErrorWrapper type I removed the 'Error' prefix. It doesn't add any new context.
    // Also, it now stores the wrapped error instead of just a formatted method.
    Wrapped(E),
    Open { threshold: i8 },
    HalfOpen { threshold: i8 },
}

// Unfortinately I made a compromise when including the inner error in E,
// we can either print a generic message for the wrapped error, or we can only print our errors nicely when the inner error
// can also be printed.
// Since `CircuitBreaker::guard` enforces the `Error` trait on the returned value, the latter made more sense
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
    // if we are wrapped an error, we should override the default implentation of cause
    // to provide that wrapped error
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            CircuitBreakerError::Wrapped(e) => Some(e),
            _ => None,
        }
    }
}
