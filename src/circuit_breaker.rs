// You had a (unused) import for std::os::macos::raw::stat. Most things in std::os are platform specific.
// This looked like something autocomplete inserted automatically
// I've also removed a bunch of unused imports
use std::error::Error;
use std::ops::Deref;
use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{mem, thread};

//Trait bounds on the left side of this aren't enforced so the `E: Error` wasn't doing anything
type Action<T, E> = Box<dyn Fn() -> Result<T, E>>;
type CircuitBreakerResult<T> = Result<T, CircuitBreakerError>;

use crate::circuit_breaker::CircuitBreakerState::{Closed, HalfOpen, Open};
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
            state: Arc::new(Mutex::new(Closed)),
        }
    }

    pub fn guard<T, E: Error>(&mut self, action: Action<T, E>) -> CircuitBreakerResult<T> {
        let state = *Arc::clone(&self.state).lock().unwrap().deref();
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
            //you don't need to use mem::replace here.
            //Its much easier to deref the mutex guard to assign to it.
            //Cargo's warning tells you this much.
            *state.lock().unwrap() = Closed;
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

    //Rust's methods are formated as lowercase_text_with_underscores
    fn attempt_action<T, E: Error>(
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
        //again you don't need to use mem::replace here.
        *state = Open;
        self.error_counter.store(0, Ordering::Relaxed);

        let state = Arc::clone(&self.state);
        let timeout = self.timeout;
        thread::spawn(move || {
            thread::sleep(timeout);
            *state.lock().unwrap() = HalfOpen;
        });
    }
}

#[derive(Copy, Clone)]
enum CircuitBreakerState {
    //Enum variants are formatted in CamelCase.
    Closed,
    Open,
    HalfOpen,
}
