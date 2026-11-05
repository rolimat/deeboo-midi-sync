use clock_core::Engine;

/// Feeds `n` single `0xF8` packets spaced by `interval_us`; returns all output bytes.
#[allow(dead_code)] // not every test binary uses it
pub fn clock_pulses(e: &mut Engine, start_us: u64, n: u64, interval_us: u64) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..n {
        out.extend(e.on_midi(start_us + i * interval_us, &[0xF8]).send);
    }
    out
}

/// Feeds `n` pulses spaced by `interval_us`, firing every engine deadline that falls strictly
/// before each pulse (and before the next, absent, one). Returns the count of output `0xF8`.
#[allow(dead_code)] // not every test binary uses it
pub fn run_with_ticks(e: &mut Engine, start_us: u64, n: u64, interval_us: u64) -> u64 {
    let mut count = 0;
    #[allow(clippy::naive_bytecount)] // tiny slices, no extra dependency
    let count_clocks = |bytes: &[u8]| bytes.iter().filter(|&&b| b == 0xF8).count() as u64;
    for i in 0..=n {
        let t = start_us + i * interval_us;
        while let Some(d) = e.next_deadline_us().filter(|&d| d < t) {
            count += count_clocks(&e.on_tick(d).send);
        }
        if i < n {
            count += count_clocks(&e.on_midi(t, &[0xF8]).send);
        }
    }
    count
}
