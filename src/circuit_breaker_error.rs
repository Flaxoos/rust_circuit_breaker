use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub struct CircuitBreakerError {
    pub message: String,
    pub error_type: CircuitBreakerErrorType,
}

// This was setup to only derive PartialEq and Debug in test, but you were using it for normal code in circuit_breakers::CircuitBreaker::guard
// I've adjusted to always derive those features.
#[derive(PartialEq, Debug)]
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
