use parking_lot::Mutex;
use std::borrow::{Borrow, BorrowMut};
use std::error::Error;
use std::fmt::{format, Display};
use std::ops::{Deref, DerefMut};
use std::os::macos::raw::stat;
use std::sync::atomic::{AtomicI8, AtomicUsize, Ordering};
use std::sync::{Arc, LockResult};
use std::time::Duration;
use std::{mem, thread};

type Action<T, E: Display> = Box<dyn Fn() -> Result<T, E>>;
type CircuitBreakerResult<T, E: Display> = Result<T, CircuitBreakerError<E>>;

use crate::circuit_breaker::CircuitBreakerState::{Closed, HalfOpen, Open};
use crate::circuit_breaker_error::{CircuitBreakerError, CircuitBreakerErrorType};

/// A Circuit Breaker
pub struct CircuitBreaker {
  failure_threshold: i8,
  half_open_attempts: i8,
  timeout: Duration,
  error_counter: AtomicI8,
  state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
  ///
  /// # Arguments
  ///
  /// * `failure_threshold` - How many failures are tolerated in the closed state before the circuit opens
  ///
  /// * `half_open_attempts` - How many attempts can be done in the half open state before the circuit opens again
  ///
  /// * `timeout` - How long is the cool off period before the circuit changes from open to half open
  pub fn new(failure_threshold: i8, half_open_attempts: i8, timeout: Duration) -> Self {
    CircuitBreaker {
      failure_threshold,
      half_open_attempts,
      timeout,
      error_counter: AtomicI8::new(0),
      state: Arc::new(Mutex::new(Closed)),
    }
  }

  /// Executes the given action through the circuit.
  ///
  /// Returns the action result or an error
  /// If the action fails in the closed state, the error will be returned wrapped in a [CircuitBreakerError]
  /// If it fails in half open or open, the appropriate [CircuitBreakerError] will be returned
  pub fn guard<T, E: Display>(&mut self, action: Action<T, E>) -> CircuitBreakerResult<T, E> {
    let state = *Arc::clone(&self.state).lock().deref();
    let state_clone = state.clone();
    mem::drop(state);
    match state_clone {
      Closed => self.attempt_action(self.failure_threshold, action),
      Open => Err(CircuitBreakerError::open(self.failure_threshold)),
      HalfOpen => match self.attempt_action(self.half_open_attempts, action) {
        Ok(action_result) => {
          let state = Arc::clone(&self.state);
          mem::replace(&mut *state.lock(), Closed);
          Ok(action_result)
        }
        Err(e) => {
          let error = if e.error_type == CircuitBreakerErrorType::ErrorWrapper {
            CircuitBreakerError::half_open(self.half_open_attempts)
          } else {
            e
          };
          Err(error)
        }
      },
    }
  }

  fn attempt_action<T, E: Display>(
    &mut self,
    threshold: i8,
    action: Action<T, E>,
  ) -> CircuitBreakerResult<T, E> {
    return if self.error_counter.load(Ordering::Relaxed) < threshold {
      match action() {
        Ok(t) => Ok(t),
        Err(e) => {
          self.error_counter.fetch_add(1, Ordering::Relaxed);
          Err(CircuitBreakerError::error_wrapper(e))
        }
      }
    } else {
      self.open_circuit();
      Err(CircuitBreakerError::open(self.failure_threshold))
    };
  }

  fn open_circuit(&mut self) {
    let mut state = self.state.lock();
    mem::replace(&mut *state, Open);
    self.error_counter.store(0, Ordering::Relaxed);

    let state = Arc::clone(&self.state);
    let timeout = self.timeout;
    thread::spawn(move || {
      thread::sleep(timeout);
      mem::replace(&mut *state.lock(), HalfOpen);
    });
  }
}

#[derive(Copy, Clone)]
enum CircuitBreakerState {
  Closed,
  Open,
  HalfOpen,
}
