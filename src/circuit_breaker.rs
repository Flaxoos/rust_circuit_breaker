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
type CircuitBreakerResult<T, E> = Result<T, CircuitBreakerError<E>>;

use crate::circuit_breaker::CircuitBreakerState::{Closed, HalfOpen, Open};
use crate::circuit_breaker_error::CircuitBreakerError;

// Ok, first issue that cargo didn't warn about.
// `failure_threshold` and `half_open_attempts` don't need to be references.
// References in structs generally make things more complicated. Its better to avoid them.
// The three reasons I'd use a immutable reference in a struct are:
// * The type of the field is not clonable
// * The type is too expensive to clone
// * The type uses interior mutabilty (such as as Mutex or Cell), and I need to change the original value.
// In this case the reference takes up 8x more memory on x64 machines than just copying the value.
// Also, by avoiding having a lifetime on the CircuitBreaker, it makes the struct easier to use.
pub struct CircuitBreaker {
    failure_threshold: i8,
    half_open_attempts: i8,
    timeout: Duration,
    error_counter: AtomicI8,
    state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: i8, half_open_attempts: i8, timeout: Duration) -> Self {
        CircuitBreaker {
            failure_threshold,
            half_open_attempts,
            timeout,
            error_counter: AtomicI8::new(0),
            state: Arc::new(Mutex::new(Closed)),
        }
    }

    //`guard` nor `attempt_action` nor `open_circuit` needed to be mut.
    // Since you've gone through the effort to use atomics/Arc/Mutexes I figured removing the mut requirement was appropriate.
    // An alternative would be to cache a `SystemTime` inside `CircuitBreakerState::Open` and compare the current time to that to determine
    // when to go to the HalfOpen state
    pub fn guard<T, E: Error>(&self, action: Action<T, E>) -> CircuitBreakerResult<T, E> {
        let state = *Arc::clone(&self.state).lock().unwrap().deref();
        let state_clone = state.clone();
        mem::drop(state);
        match state_clone {
            Closed => self.attempt_action(self.failure_threshold, action),
            Open => Err(CircuitBreakerError::Open {
                threshold: self.failure_threshold,
            }),
            HalfOpen => {
                match self.attempt_action(self.half_open_attempts, action) {
                    Ok(action_result) => {
                        let state = Arc::clone(&self.state);
                        //you don't need to use mem::replace here.
                        //Its much easier to deref the mutex guard to assign to it.
                        //Cargo's warning tells you this much.
                        *state.lock().unwrap() = Closed;
                        Ok(action_result)
                    }
                    Err(e) => {
                        let error = if let CircuitBreakerError::Wrapped(_) = e {
                            CircuitBreakerError::HalfOpen {
                                threshold: self.failure_threshold,
                            }
                        } else {
                            e
                        };
                        Err(error)
                    }
                }
            }
        }
    }

    //Rust's methods are formated as lowercase_text_with_underscores
    //
    // threshold didn't need to be a reference since `i8` is trivially copyable
    fn attempt_action<T, E: Error>(
        &self,
        threshold: i8,
        action: Action<T, E>,
    ) -> CircuitBreakerResult<T, E> {
        return if self.error_counter.load(Ordering::Relaxed) < threshold {
            match action() {
                Ok(t) => CircuitBreakerResult::Ok(t),
                Err(e) => {
                    self.error_counter.fetch_add(1, Ordering::Relaxed);
                    CircuitBreakerResult::Err(CircuitBreakerError::Wrapped(e))
                }
            }
        } else {
            self.open_circuit();
            Result::Err(CircuitBreakerError::Open { threshold })
        };
    }

    fn open_circuit(&self) {
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
