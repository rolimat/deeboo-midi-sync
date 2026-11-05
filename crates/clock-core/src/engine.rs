//! The deterministic clock engine: bytes and commands in, bytes and state out.

use crate::bpm::BpmMeter;
use crate::{PPQN, SIGNAL_TIMEOUT_US};

const CLOCK: u8 = 0xF8;
const START: u8 = 0xFA;
const STOP: u8 = 0xFC;

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
    play_started_us: u64,
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
            play_started_us: 0,
            out_pulses: 0,
            strength_hundredths,
            nudge_acc: 0,
        }
    }

    /// Scans every byte of a packet; only `0xF8` and `0xFC` have an effect.
    /// Start, Continue and SPP (and its data bytes) are ignored.
    pub fn on_midi(&mut self, now_us: u64, bytes: &[u8]) -> Output {
        let mut out = self.on_tick(now_us);
        for &b in bytes {
            match b {
                CLOCK => {
                    self.on_clock(now_us);
                    if self.playing {
                        self.out_pulses += 1;
                        out.send.push(CLOCK);
                    }
                }
                STOP if self.playing => {
                    self.playing = false;
                    out.send.push(STOP);
                }
                _ => {}
            }
        }
        out
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

    pub fn on_command(&mut self, now_us: u64, cmd: Command) -> Output {
        match cmd {
            Command::Play if !self.playing => {
                self.playing = true;
                self.play_started_us = now_us;
                self.out_pulses = 0;
                Output { send: vec![START] }
            }
            Command::Pause if self.playing => {
                self.playing = false;
                Output { send: vec![STOP] }
            }
            _ => Output::default(),
        }
    }

    /// Sends one `Stop` and pauses when the clock was lost while playing.
    pub fn on_tick(&mut self, now_us: u64) -> Output {
        match self.next_deadline_us() {
            Some(d) if now_us > d => {
                self.playing = false;
                Output { send: vec![STOP] }
            }
            _ => Output::default(),
        }
    }

    /// While playing: when signal loss will be declared. `None` while paused.
    #[must_use]
    pub fn next_deadline_us(&self) -> Option<u64> {
        self.playing
            .then(|| self.last_clock_us.unwrap_or(0).max(self.play_started_us) + SIGNAL_TIMEOUT_US)
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
