// I have no idea why you had `extern crate core;` here.

// `pub use` reexports items so that users of your library can access them.
// you can also acheive this by adding `pub` to the module, but in this case it would make the api
// more complex than it needs to be.
pub use crate::circuit_breaker::*;
pub use crate::circuit_breaker_error::*;

mod circuit_breaker;
mod circuit_breaker_error;

#[cfg(test)]
mod tests {
    //cleaned up unused imports -- intellij rust's autocomplete is helpful and inserts them automatically.
    use std::error::Error;
    use std::fmt::{Debug, Display, Formatter};
    use std::time::Duration;
    use std::thread;

    use crate::CircuitBreakerError;
    use crate::circuit_breaker::CircuitBreaker;

    //Rust's constants are in UPPER_CASE_WITH_UNDERSCORES
    const FAILURE_THRESHOLD: i8 = 3;
    const HALF_OPEN_ATTEMPTS: i8 = 2;
    const TIMEOUT: Duration = Duration::new(1, 0);

    fn create_circuit_breaker() -> CircuitBreaker {
        return CircuitBreaker::new(FAILURE_THRESHOLD, HALF_OPEN_ATTEMPTS, TIMEOUT);
    }

    #[test]
    fn should_let_actions_through_when_open() {
        let cb = create_circuit_breaker();

        //Results must be used. It is almost always a logic error when you don't.
        let result = cb.guard::<String, ActionError>(Box::new(|| Ok("hello".to_string())));

        //panics in tests are by default considered failures.
        let result = result.unwrap();
        assert_eq!(result, "hello".to_string());
    }

    #[test]
    fn should_switch_to_open_after_failure_threshold() {
        let cb = create_circuit_breaker();
        for _ in 0..FAILURE_THRESHOLD {
            let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
            assert!(result.is_err());
            let want = CircuitBreakerError::Wrapped(ActionError {});
            let got = result.unwrap_err();
            assert_eq!(got, want)
        }

        // should switch to open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        let want = CircuitBreakerError::Open { threshold: FAILURE_THRESHOLD};
        let got = result.unwrap_err();
        assert_eq!(got, want);

        // should stay open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        let want = CircuitBreakerError::Open { threshold: FAILURE_THRESHOLD};
        let got = result.unwrap_err();
        assert_eq!(got, want)
    }

    #[test]
    fn should_switch_to_half_open_after_failure_threshold_exceeded_and_timeout_period_passed() {
        let cb = create_circuit_breaker();
        for _ in 0..FAILURE_THRESHOLD {
            let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
            assert!(result.is_err());
            let want = CircuitBreakerError::Wrapped(ActionError {});
            let got = result.unwrap_err();
            assert_eq!(got, want)
        }

        // should switch to open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        let want = CircuitBreakerError::Open { threshold: FAILURE_THRESHOLD};
        let got = result.unwrap_err();
        assert_eq!(got, want);

        thread::sleep(TIMEOUT.mul_f32(1.1));

        // should switch to half open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());

        // I don't know what the expected threshold should be in the half open state, so I'm using match to assert the error
        match result.unwrap_err() {
            CircuitBreakerError::HalfOpen { threshold: _ } => (), //Expected
            e => panic!("Wanted a `CircuitBreakerError::HalfOpen {{ .. }} got a {}", e),
        };
        // let want = CircuitBreakerError::HalfOpen { error_count: HALF_OPEN_ATTEMPTS };
        // let got = result.unwrap_err();
        // assert_eq!(got, want);
    }

    #[test]
    fn should_switch_to_open_after_failure_threshold_exceeded_and_timeout_period_passed_and_half_open_attempts_exceeded(
    ) {
        let cb = create_circuit_breaker();
        for _ in 0..FAILURE_THRESHOLD {
            let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
            assert!(result.is_err());
            let want = CircuitBreakerError::Wrapped(ActionError {});
            let got = result.unwrap_err();
            assert_eq!(got, want)
        }

        // should switch to open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        let want = CircuitBreakerError::Open { threshold: FAILURE_THRESHOLD};
        let got = result.unwrap_err();
        assert_eq!(got, want);

        thread::sleep(TIMEOUT.mul_f32(1.1));

        // should switch to half open
        for _ in 0..HALF_OPEN_ATTEMPTS {
            let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
            assert!(result.is_err());
            // I don't know what the expected threshold should be in the half open state, so I'm using match to assert the error
            match result.unwrap_err() {
                CircuitBreakerError::HalfOpen { threshold: _ } => (), //Expected
                e => panic!("Wanted a `CircuitBreakerError::HalfOpen {{ .. }} got a {}", e),
            };
            // let want = CircuitBreakerError::HalfOpen { error_count: HALF_OPEN_ATTEMPTS };
            // let got = result.unwrap_err();
            // assert_eq!(got, want);
        }

        // should switch back to open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        // I don't know what the expected threshold should be in the half open state, so I'm using match to assert the error
        match result.unwrap_err() {
            CircuitBreakerError::Open { threshold: _ } => (), //Expected
            e => panic!("Wanted a `CircuitBreakerError::Open {{ .. }} got a {}", e),
        };
        // let want = CircuitBreakerError::Open { error_count: FAILURE_THRESHOLD};
        // let got = result.unwrap_err();
        // assert_eq!(got, want);
    }

    #[test]
    fn should_switch_to_closed_after_failure_threshold_exceeded_and_timeout_period_passed_and_action_works_again(
    ) {
        let cb = create_circuit_breaker();
        for _ in 0..FAILURE_THRESHOLD {
            let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
            assert!(result.is_err());
            let want = CircuitBreakerError::Wrapped(ActionError {});
            let got = result.unwrap_err();
            assert_eq!(got, want)
        }

        // should switch to open
        let result = cb.guard::<String, ActionError>(Box::new(|| Err(ActionError {})));
        assert!(result.is_err());
        let want = CircuitBreakerError::Open { threshold: FAILURE_THRESHOLD};
        let got = result.unwrap_err();
        assert_eq!(got, want);

        thread::sleep(TIMEOUT.mul_f32(1.1));

        // should switch to closed
        let result = cb.guard::<String, ActionError>(Box::new(|| Ok("hello".to_string())));
        assert!(!result.is_err());
    }

    #[derive(PartialEq)]
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
