//! Cross-platform retry policy primitives.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Authentication,
    RateLimited,
    Network,
    Temporary,
    InvalidInput,
    Permanent,
}

impl ErrorClass {
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Network | Self::Temporary)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    pub const fn conservative() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(300),
        }
    }

    pub const fn allows(self, class: ErrorClass, attempt: u32) -> bool {
        class.is_retryable() && attempt < self.max_attempts
    }

    pub fn delay(self, attempt: u32) -> Duration {
        let exponent = attempt.min(31);
        let multiplier = 1_u64 << exponent;
        let delay = self.base_delay.saturating_mul(multiplier as u32);
        delay.min(self.max_delay)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDecision {
    pub retry: bool,
    pub delay: Duration,
}

pub fn decide_retry(policy: RetryPolicy, class: ErrorClass, attempt: u32) -> RetryDecision {
    RetryDecision {
        retry: policy.allows(class, attempt),
        delay: policy.delay(attempt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_transient_failures_within_budget() {
        let policy = RetryPolicy::conservative();
        assert!(decide_retry(policy, ErrorClass::Network, 0).retry);
        assert!(!decide_retry(policy, ErrorClass::Authentication, 0).retry);
        assert!(!decide_retry(policy, ErrorClass::Network, 5).retry);
    }

    #[test]
    fn caps_exponential_delay() {
        let policy = RetryPolicy {
            max_attempts: 10,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(5),
        };
        assert_eq!(policy.delay(0), Duration::from_secs(2));
        assert_eq!(policy.delay(1), Duration::from_secs(4));
        assert_eq!(policy.delay(2), Duration::from_secs(5));
    }
}
