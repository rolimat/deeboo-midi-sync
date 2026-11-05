//! The deterministic clock engine: bytes and commands in, bytes and state out.

use crate::bpm::BpmMeter;
use crate::{PPQN, SIGNAL_TIMEOUT_US};

const CLOCK: u8 = 0xF8;
const START: u8 = 0xFA;
const STOP: u8 = 0xFC;
const NUDGE_THRESHOLD: u32 = 10_000;

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
    strength_hundredths: u32,
    /// Nudge accumulator, integer fixed-point, threshold `NUDGE_THRESHOLD`.
    nudge_acc: u32,
    nudge: Option<Direction>,
    /// When the inserted extra pulse is due.
    pending_extra_us: Option<u64>,
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
            nudge: None,
            pending_extra_us: None,
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
                    if self.playing && self.nudge_forwards(now_us) {
                        self.out_pulses += 1;
                        out.send.push(CLOCK);
                    }
                }
                STOP if self.playing => {
                    self.pause();
                    out.send.push(STOP);
                }
                _ => {}
            }
        }
        out
    }

    /// Applies the nudge to one incoming pulse; `false` means the pulse is dropped.
    fn nudge_forwards(&mut self, now_us: u64) -> bool {
        let Some(dir) = self.nudge else { return true };
        self.nudge_acc += self.strength_hundredths;
        if self.nudge_acc < NUDGE_THRESHOLD {
            return true;
        }
        self.nudge_acc -= NUDGE_THRESHOLD;
        match dir {
            Direction::Push => {
                if let Some(iv) = self.meter.interval_us() {
                    self.pending_extra_us = Some(now_us + iv / 2);
                }
                true
            }
            Direction::Slow => false,
        }
    }

    /// Leaves the playing state and cancels any nudge.
    fn pause(&mut self) {
        self.playing = false;
        self.pending_extra_us = None;
        self.nudge_acc = 0;
        self.nudge = None;
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
                self.pause();
                Output { send: vec![STOP] }
            }
            Command::NudgeStart(d) if self.playing => {
                self.nudge = Some(d);
                Output::default()
            }
            Command::NudgeStop if self.playing => {
                self.nudge = None;
                self.nudge_acc = 0;
                Output::default()
            }
            Command::SetNudgeStrength(p) if (1.0..=10.0).contains(&p) => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 100..=1000
                {
                    self.strength_hundredths = (p * 100.0).round() as u32;
                }
                Output::default()
            }
            _ => Output::default(),
        }
    }

    /// Sends one `Stop` and pauses when the clock was lost while playing,
    /// otherwise emits a due nudge pulse.
    pub fn on_tick(&mut self, now_us: u64) -> Output {
        if self.loss_deadline_us().is_some_and(|d| now_us > d) {
            self.pause();
            return Output { send: vec![STOP] };
        }
        if self.pending_extra_us.is_some_and(|p| p <= now_us) {
            self.pending_extra_us = None;
            self.out_pulses += 1;
            return Output { send: vec![CLOCK] };
        }
        Output::default()
    }

    /// While playing: when signal loss will be declared. `None` while paused.
    fn loss_deadline_us(&self) -> Option<u64> {
        self.playing
            .then(|| self.last_clock_us.unwrap_or(0).max(self.play_started_us) + SIGNAL_TIMEOUT_US)
    }

    /// The earlier of the pending nudge pulse and the signal-loss deadline.
    #[must_use]
    pub fn next_deadline_us(&self) -> Option<u64> {
        match (self.pending_extra_us, self.loss_deadline_us()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
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
