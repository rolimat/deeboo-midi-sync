//! The deterministic clock engine: bytes and commands in, bytes and state out.

use crate::bpm::BpmMeter;
use crate::{PPQN, SIGNAL_TIMEOUT_US};

const CLOCK: u8 = 0xF8;

/// Nudge direction: `Push` inserts a pulse, `Slow` drops one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Push,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Play,
    Pause,
    NudgeStart(Direction),
    NudgeStop,
    SetNudgeStrength(f64),
}

/// Bytes to write to Out, in order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Output {
    pub send: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// `None` without signal or with fewer than 2 pulses.
    pub bpm: Option<f64>,
    /// `1..=4` while playing, `None` while paused.
    pub beat: Option<u8>,
    pub playing: bool,
    /// A `0xF8` arrived within `SIGNAL_TIMEOUT_US`.
    pub in_ok: bool,
    /// Received `0xF8` count / 24.
    pub input_beats: u64,
}

#[derive(Debug)]
pub struct Engine {
    meter: BpmMeter,
    last_clock_us: Option<u64>,
    input_pulses: u64,
    playing: bool,
    out_pulses: u64,
    /// Nudge strength in hundredths of a percent.
    #[allow(dead_code)] // used by nudge (task 5)
    strength_hundredths: u32,
    /// Nudge accumulator, integer fixed-point, threshold `10_000`.
    #[allow(dead_code)] // used by nudge (task 5)
    nudge_acc: i64,
}

impl Engine {
    #[must_use]
    pub fn new(nudge_strength_pct: f64) -> Self {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped below
        let strength_hundredths = (nudge_strength_pct * 100.0).round().clamp(0.0, 1e6) as u32;
        Self {
            meter: BpmMeter::default(),
            last_clock_us: None,
            input_pulses: 0,
            playing: false,
            out_pulses: 0,
            strength_hundredths,
            nudge_acc: 0,
        }
    }

    /// Scans every byte of a packet; only `0xF8` has an effect so far.
    pub fn on_midi(&mut self, now_us: u64, bytes: &[u8]) -> Output {
        for &b in bytes {
            if b == CLOCK {
                self.on_clock(now_us);
            }
        }
        Output::default()
    }

    fn on_clock(&mut self, now_us: u64) {
        if self
            .last_clock_us
            .is_some_and(|l| now_us.saturating_sub(l) > SIGNAL_TIMEOUT_US)
        {
            self.meter.reset();
        }
        self.meter.record(now_us);
        self.last_clock_us = Some(now_us);
        self.input_pulses += 1;
    }

    pub fn on_command(&mut self, _now_us: u64, _cmd: Command) -> Output {
        Output::default()
    }

    pub fn on_tick(&mut self, _now_us: u64) -> Output {
        Output::default()
    }

    #[must_use]
    pub fn next_deadline_us(&self) -> Option<u64> {
        None
    }

    #[must_use]
    pub fn state(&self, now_us: u64) -> State {
        let in_ok = self
            .last_clock_us
            .is_some_and(|l| now_us.saturating_sub(l) <= SIGNAL_TIMEOUT_US);
        let beat = self.playing.then(|| {
            #[allow(clippy::cast_possible_truncation)] // value is 1..=4
            let b = (self.out_pulses / PPQN % 4) as u8 + 1;
            b
        });
        State {
            bpm: if in_ok { self.meter.bpm() } else { None },
            beat,
            playing: self.playing,
            in_ok,
            input_beats: self.input_pulses / PPQN,
        }
    }
}
