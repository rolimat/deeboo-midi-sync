use clock_core::Engine;

/// Feeds `n` single `0xF8` packets spaced by `interval_us`; returns all output bytes.
pub fn clock_pulses(e: &mut Engine, start_us: u64, n: u64, interval_us: u64) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..n {
        out.extend(e.on_midi(start_us + i * interval_us, &[0xF8]).send);
    }
    out
}
