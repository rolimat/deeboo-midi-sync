//! Tempo measurement from clock pulse timestamps.

use crate::PPQN;

const RING: usize = 24;

/// Rolling mean of the last [`RING`] pulse intervals.
#[derive(Debug, Default)]
pub(crate) struct BpmMeter {
    intervals: [u64; RING],
    len: usize,
    next: usize,
    last_us: Option<u64>,
}

impl BpmMeter {
    /// Records a pulse. Intervals that are zero or negative are ignored.
    pub(crate) fn record(&mut self, now_us: u64) {
        if let Some(last) = self.last_us {
            if now_us <= last {
                return;
            }
            self.intervals[self.next] = now_us - last;
            self.next = (self.next + 1) % RING;
            self.len = (self.len + 1).min(RING);
        }
        self.last_us = Some(now_us);
    }

    /// Forgets all history (used after signal loss).
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn mean_us(&self) -> Option<f64> {
        if self.len == 0 {
            return None;
        }
        let sum: u64 = self.intervals[..self.len].iter().sum();
        #[allow(clippy::cast_precision_loss)] // sums are far below 2^52 µs
        Some(sum as f64 / self.len as f64)
    }

    /// Beats per minute, `None` with fewer than two pulses.
    pub(crate) fn bpm(&self) -> Option<f64> {
        #[allow(clippy::cast_precision_loss)] // PPQN is 24
        self.mean_us().map(|m| 60e6 / (PPQN as f64 * m))
    }

    /// Mean pulse interval in µs, `None` with fewer than two pulses.
    #[allow(dead_code)] // used by nudge (task 5)
    pub(crate) fn interval_us(&self) -> Option<u64> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive, small
        self.mean_us().map(|m| m.round() as u64)
    }
}
