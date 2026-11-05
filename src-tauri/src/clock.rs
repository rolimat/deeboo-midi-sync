//! The single monotonic time base shared by every MIDI callback and timer event.

use std::time::Instant;

/// Microseconds since construction. midir timestamps are never mixed in.
#[derive(Debug, Clone, Copy)]
pub struct MonoClock {
    start: Instant,
}

impl MonoClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[must_use]
    pub fn now_us(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

impl Default for MonoClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn now_us_is_monotonic() {
        let clock = MonoClock::new();
        let a = clock.now_us();
        thread::sleep(Duration::from_millis(2));
        let b = clock.now_us();
        assert!(b - a >= 2000, "only {} us elapsed", b - a);
    }
}
