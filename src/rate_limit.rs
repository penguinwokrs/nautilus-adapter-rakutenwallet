use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

/// Token Bucket rate limiter.
///
/// Rakuten Wallet rate limits:
/// - Minimum 200ms between requests per user (5 req/s)
#[derive(Clone)]
pub struct TokenBucket {
    inner: Arc<Mutex<TokenBucketInner>>,
}

struct TokenBucketInner {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new TokenBucket.
    ///
    /// - `capacity`: Maximum number of tokens (burst size)
    /// - `refill_rate`: Tokens added per second
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenBucketInner {
                tokens: capacity,
                capacity,
                refill_rate,
                last_refill: Instant::now(),
            })),
        }
    }

    /// Acquire a token, waiting if necessary.
    pub async fn acquire(&self) {
        loop {
            let wait_time = {
                let mut inner = self.inner.lock().await;
                inner.refill();

                if inner.tokens >= 1.0 {
                    inner.tokens -= 1.0;
                    return;
                }

                // Calculate time to wait for 1 token
                let deficit = 1.0 - inner.tokens;
                Duration::from_secs_f64(deficit / inner.refill_rate)
            };

            sleep(wait_time).await;
        }
    }
}

impl TokenBucketInner {
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}
