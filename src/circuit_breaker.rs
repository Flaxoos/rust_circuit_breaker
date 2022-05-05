use std::borrow::{Borrow, BorrowMut};
use std::error::Error;
use std::fmt::{format, Display};
use std::ops::{Deref, DerefMut};
use std::os::macos::raw::stat;
use std::sync::atomic::{AtomicI8, AtomicUsize, Ordering};
use std::sync::{Arc, LockResult, Mutex};
use std::time::Duration;
use std::{mem, thread};

type Action<T, E: Error> = Box<dyn Fn() -> Result<T, E>>;
type CircuitBreakerResult<T> = Result<T, CircuitBreakerError>;

use crate::circuit_breaker::CircuitBreakerState::{CLOSED, HALF_OPEN, OPEN};
use crate::circuit_breaker_error::{CircuitBreakerError, CircuitBreakerErrorType};

pub struct CircuitBreaker<'cb> {
  failure_threshold: &'cb i8,
  half_open_attempts: &'cb i8,
  timeout: Duration,
  error_counter: AtomicI8,
  state: Arc<Mutex<CircuitBreakerState>>,
}

impl<'cb> CircuitBreaker<'cb> {
  pub fn new(failure_threshold: &'cb i8, half_open_attempts: &'cb i8, timeout: Duration) -> Self {
    CircuitBreaker {
      failure_threshold,
      half_open_attempts,
      timeout,
      error_counter: AtomicI8::new(0),
      state: Arc::new(Mutex::new(CLOSED)),
    }
  }

  pub fn guard<T, E: Error>(&mut self, action: Action<T, E>) -> CircuitBreakerResult<T> {
    let state = *Arc::clone(&self.state).lock().unwrap().deref();
    let state_clone = state.clone();
    mem::drop(state);
    match state_clone{
      CLOSED => {
        self.attemptAction(self.failure_threshold, action)
      },
      OPEN => Err(CircuitBreakerError{
        error_type: CircuitBreakerErrorType::Open,
        message : format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", self.failure_threshold),
      }),
      HALF_OPEN => {
        match self.attemptAction(self.half_open_attempts, action){
          Ok(action_result) => {
            let state = Arc::clone(&self.state);
            mem::replace(&mut *state.lock().unwrap(), CLOSED);
            Ok(action_result)
          }
          Err(e) => {
            let error = if e.error_type == CircuitBreakerErrorType::ErrorWrapper {
              CircuitBreakerError {
                error_type: CircuitBreakerErrorType::HalfOpen,
                message: format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", self.failure_threshold),
              }
            }else {
              e
            };
            Err(error)
          }
        }
      }
    }
  }

  fn attemptAction<T, E: Error>(
    &mut self,
    threshold: &'cb i8,
    action: Action<T, E>,
  ) -> CircuitBreakerResult<T> {
    return if &self.error_counter.load(Ordering::Relaxed) < threshold {
      match action() {
        Ok(t) => CircuitBreakerResult::Ok(t),
        Err(e) => {
          self.error_counter.fetch_add(1, Ordering::Relaxed);
          CircuitBreakerResult::Err(CircuitBreakerError {
            error_type: CircuitBreakerErrorType::ErrorWrapper,
            message: format!("Action failed {}", e),
          })
        }
      }
    } else {
      self.open_circuit();
      Result::Err(CircuitBreakerError{
        error_type: CircuitBreakerErrorType::Open,
        message : format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", self.failure_threshold),
      })
    };
  }

  fn open_circuit(&mut self) {
    let mut state = self.state.lock().unwrap();
    mem::replace(&mut *state, OPEN);
    self.error_counter.store(0, Ordering::Relaxed);

    let state = Arc::clone(&self.state);
    let timeout = self.timeout;
    thread::spawn(move || {
      thread::sleep(timeout);
      mem::replace(&mut *state.lock().unwrap(), HALF_OPEN);
    });
  }
}

#[derive(Copy, Clone)]
enum CircuitBreakerState {
  CLOSED,
  OPEN,
  HALF_OPEN,
}
