extern crate core;

mod circuit_breaker;
mod circuit_breaker_error;

#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::ffi::NulError;
  use std::fmt::{format, write, Debug, Display, Formatter, Pointer};
  use std::io::ErrorKind;
  use std::thread::Thread;
  use std::time::Duration;
  use std::{io, thread};

  use crate::circuit_breaker::CircuitBreaker;
  use crate::circuit_breaker_error::{CircuitBreakerError, CircuitBreakerErrorType};

  const FAILURE_THRESHOLD: i8 = 3;
  const HALF_OPEN_ATTEMPTS: i8 = 2;
  const TIMEOUT: Duration = Duration::new(0, 5000000);

  fn create_circuit_breaker() -> CircuitBreaker {
    return CircuitBreaker::new(FAILURE_THRESHOLD, HALF_OPEN_ATTEMPTS, TIMEOUT);
  }

  #[test]
  fn should_let_actions_through_when_open() {
    let mut cb = create_circuit_breaker();
    cb.guard::<String, ActionError>(Box::new(|| Ok("hello".to_string())));
  }

  #[test]
  fn should_switch_to_open_after_failure_threshold() {
    let mut cb = create_circuit_breaker();
    for _ in 0..FAILURE_THRESHOLD {
      let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
      assert!(result.is_err());
      let want = CircuitBreakerErrorType::ErrorWrapper;
      let got = result.unwrap_err().error_type;
      assert_eq!(got, want)
    }

    // should switch to open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);

    // should stay open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want)
  }

  #[test]
  fn should_switch_to_half_open_after_failure_threshold_exceeded_and_timeout_period_passed() {
    let mut cb = create_circuit_breaker();
    for _ in 0..FAILURE_THRESHOLD {
      let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
      assert!(result.is_err());
      let want = CircuitBreakerErrorType::ErrorWrapper;
      let got = result.unwrap_err().error_type;
      assert_eq!(got, want)
    }

    // should switch to open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);

    thread::sleep(TIMEOUT.mul_f32(1.1));

    // should switch to half open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::HalfOpen;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);
  }

  #[test]
  fn should_switch_to_open_after_failure_threshold_exceeded_and_timeout_period_passed_and_half_open_attempts_exceeded(
  ) {
    let mut cb = create_circuit_breaker();
    for _ in 0..FAILURE_THRESHOLD {
      let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
      assert!(result.is_err());
      let want = CircuitBreakerErrorType::ErrorWrapper;
      let got = result.unwrap_err().error_type;
      assert_eq!(got, want)
    }

    // should switch to open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);

    thread::sleep(TIMEOUT.mul_f32(1.1));

    // should switch to half open
    for _ in 0..HALF_OPEN_ATTEMPTS {
      let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
      assert!(result.is_err());
      let want = CircuitBreakerErrorType::HalfOpen;
      let got = result.unwrap_err().error_type;
      assert_eq!(got, want)
    }

    // should switch back to open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);
  }

  #[test]
  fn should_switch_to_closed_after_failure_threshold_exceeded_and_timeout_period_passed_and_action_works_again(
  ) {
    let mut cb = create_circuit_breaker();
    for _ in 0..FAILURE_THRESHOLD {
      let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
      assert!(result.is_err());
      let want = CircuitBreakerErrorType::ErrorWrapper;
      let got = result.unwrap_err().error_type;
      assert_eq!(got, want)
    }

    // should switch to open
    let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
    assert!(result.is_err());
    let want = CircuitBreakerErrorType::Open;
    let got = result.unwrap_err().error_type;
    assert_eq!(got, want);

    thread::sleep(TIMEOUT.mul_f32(1.1));

    // should switch to closed
    let result = cb.guard::<String, ActionError>(Box::new(|| Ok("hello".to_string())));
    assert!(!result.is_err());
  }

  struct ActionError {}

  impl Debug for ActionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", "Action Error")
    }
  }

  impl Display for ActionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", "Action Error")
    }
  }

  impl Error for ActionError {}
}
