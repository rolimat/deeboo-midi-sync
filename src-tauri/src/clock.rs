//! The single time base shared by every MIDI callback and timer event: `CoreMIDI` host time
//! (`mach_absolute_time`) in microseconds, so `CoreMIDI` event stamps and our clock are the same clock.

use clock_core::SIGNAL_TIMEOUT_US;

/// Mach timebase: `ns = ticks * numer / denom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timebase {
    pub numer: u32,
    pub denom: u32,
}

impl Timebase {
    /// Ticks to microseconds, rounding down.
    #[must_use]
    pub fn ticks_to_us(self, ticks: u64) -> u64 {
        let ns = u128::from(ticks) * u128::from(self.numer) / u128::from(self.denom);
        u64::try_from(ns / 1000).unwrap_or(u64::MAX)
    }

    /// Microseconds to ticks, rounding up, so a round trip never loses the microsecond.
    #[must_use]
    pub fn us_to_ticks(self, us: u64) -> u64 {
        let num = u128::from(us) * 1000 * u128::from(self.denom);
        let numer = u128::from(self.numer);
        u64::try_from(num.div_ceil(numer)).unwrap_or(u64::MAX)
    }
}

#[cfg(target_os = "macos")]
mod sys {
    #[repr(C)]
    pub struct MachTimebaseInfo {
        pub numer: u32,
        pub denom: u32,
    }

    unsafe extern "C" {
        pub safe fn mach_absolute_time() -> u64;
        pub fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
    }
}

/// Host time in microseconds. `CoreMIDI` packet stamps share this clock.
#[derive(Debug, Clone, Copy)]
pub struct HostClock {
    timebase: Timebase,
    #[cfg(not(target_os = "macos"))]
    start: std::time::Instant,
}

impl HostClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            timebase: Self::read_timebase(),
            #[cfg(not(target_os = "macos"))]
            start: std::time::Instant::now(),
        }
    }

    #[cfg(target_os = "macos")]
    fn read_timebase() -> Timebase {
        let mut info = sys::MachTimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: `info` is a valid, writable struct matching the C layout.
        let status = unsafe { sys::mach_timebase_info(&raw mut info) };
        if status != 0 || info.numer == 0 || info.denom == 0 {
            log::error!("mach_timebase_info failed (status {status}); assuming 1/1");
            return Timebase { numer: 1, denom: 1 };
        }
        Timebase {
            numer: info.numer,
            denom: info.denom,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn read_timebase() -> Timebase {
        Timebase { numer: 1, denom: 1 }
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::unused_self)]
    fn now_ticks(self) -> u64 {
        sys::mach_absolute_time()
    }

    #[cfg(not(target_os = "macos"))]
    fn now_ticks(self) -> u64 {
        u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn now_us(self) -> u64 {
        self.timebase.ticks_to_us(self.now_ticks())
    }

    #[must_use]
    pub fn ticks_to_us(self, ticks: u64) -> u64 {
        self.timebase.ticks_to_us(ticks)
    }

    #[must_use]
    pub fn us_to_ticks(self, us: u64) -> u64 {
        self.timebase.us_to_ticks(us)
    }
}

impl Default for HostClock {
    fn default() -> Self {
        Self::new()
    }
}

/// The master's stamp when it is plausible (not in the future, not older than the signal
/// timeout); otherwise `now_us`.
#[must_use]
pub fn event_time_us(stamp_us: Option<u64>, now_us: u64) -> u64 {
    match stamp_us {
        Some(s) if s <= now_us && now_us - s <= SIGNAL_TIMEOUT_US => s,
        _ => now_us,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASES: [(u32, u32); 3] = [(1, 1), (125, 3), (3, 7)];

    #[test]
    fn apple_silicon_ticks_convert_to_us() {
        let tb = Timebase {
            numer: 125,
            denom: 3,
        };
        assert_eq!(tb.ticks_to_us(24), 1);
        assert_eq!(tb.ticks_to_us(24_000_000), 1_000_000);
    }

    #[test]
    fn us_round_trips_through_ticks() {
        for (numer, denom) in BASES {
            let tb = Timebase { numer, denom };
            for us in [0, 1, 999, 20_000, 1_000_000_000_000] {
                assert_eq!(
                    tb.ticks_to_us(tb.us_to_ticks(us)),
                    us,
                    "{numer}/{denom} {us}"
                );
            }
        }
    }

    #[test]
    fn stamp_from_now_never_lands_after_now() {
        for (numer, denom) in BASES {
            let tb = Timebase { numer, denom };
            for t in [0, 1, 23, 24, 25, 123_456_789, u64::MAX / 2] {
                assert!(
                    tb.us_to_ticks(tb.ticks_to_us(t)) <= t,
                    "{numer}/{denom} {t}"
                );
            }
        }
    }

    #[test]
    fn event_time_keeps_valid_stamps() {
        let now = 10_000_000;
        assert_eq!(event_time_us(Some(now - 70), now), now - 70);
        assert_eq!(event_time_us(Some(now - 500_000), now), now - 500_000);
        assert_eq!(event_time_us(Some(900), 1_000), 900);
    }

    #[test]
    fn event_time_replaces_missing_future_or_stale_stamps() {
        let now = 10_000_000;
        assert_eq!(event_time_us(None, now), now);
        assert_eq!(event_time_us(Some(now + 1), now), now);
        assert_eq!(event_time_us(Some(now - 500_001), now), now);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn host_clock_tracks_wall_time() {
        let clock = HostClock::new();
        let a = clock.now_us();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = clock.now_us();
        assert!((2_000..50_000).contains(&(b - a)), "{} us elapsed", b - a);
    }
}
