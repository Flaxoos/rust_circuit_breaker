use std::borrow::{Borrow, BorrowMut};
use std::error::Error;
use std::fmt::{format, Display};
use std::ops::{Deref, DerefMut};
use std::os::macos::raw::stat;
use std::sync::atomic::{AtomicI8, AtomicUsize, Ordering};
use std::sync::{Arc, LockResult};
use std::time::Duration;
use std::{mem, thread};
use parking_lot::Mutex;

type Action<T, E: Error> = Box<dyn Fn() -> Result<T, E>>;
type CircuitBreakerResult<T> = Result<T, CircuitBreakerError>;

use crate::circuit_breaker::CircuitBreakerState::{Closed, HalfOpen, Open};
use crate::circuit_breaker_error::{CircuitBreakerError, CircuitBreakerErrorType};

pub struct CircuitBreaker {
  failure_threshold: i8,
  half_open_attempts: i8,
  timeout: Duration,
  error_counter: AtomicI8,
  state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
  pub fn new(failure_threshold:i8, half_open_attempts: i8, timeout: Duration) -> Self {
    CircuitBreaker {
      failure_threshold,
      half_open_attempts,
      timeout,
      error_counter: AtomicI8::new(0),
      state: Arc::new(Mutex::new(Closed)),
    }
  }

  pub fn guard<T, E: Error>(&mut self, action: Action<T, E>) -> CircuitBreakerResult<T> {
    let state = *Arc::clone(&self.state).lock().deref();
    let state_clone = state.clone();
    mem::drop(state);
    match state_clone{
      Closed => {
        self.attempt_action(self.failure_threshold, action)
      },
      Open => Err(CircuitBreakerError{
        error_type: CircuitBreakerErrorType::Open,
        message : format!("Action failed more than {} times, subsequent calls will be prevented until action is successful again", self.failure_threshold),
      }),
      HalfOpen => {
        match self.attempt_action(self.half_open_attempts, action){
          Ok(action_result) => {
            let state = Arc::clone(&self.state);
            mem::replace(&mut *state.lock(), Closed);
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

  fn attempt_action<T, E: Error>(
    &mut self,
    threshold: i8,
    action: Action<T, E>,
  ) -> CircuitBreakerResult<T> {
    return if self.error_counter.load(Ordering::Relaxed) < threshold {
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
