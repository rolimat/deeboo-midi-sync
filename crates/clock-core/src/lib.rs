//! Pure MIDI clock logic. No I/O, no threads: time is always passed in.

mod bpm;
mod engine;

pub use engine::{Command, Direction, Engine, Output, State};

/// MIDI clock pulses per quarter note.
pub const PPQN: u64 = 24;

/// No clock for this long (µs) means "no signal".
pub const SIGNAL_TIMEOUT_US: u64 = 500_000;

#[test]
fn constants_match_spec() {
    assert_eq!(PPQN, 24);
    assert_eq!(SIGNAL_TIMEOUT_US, 500_000);
}
