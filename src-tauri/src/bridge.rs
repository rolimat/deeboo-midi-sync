//! Virtual `CoreMIDI` ports wired to the clock engine, plus the timer thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::Duration;

use crate::clock::{HostClock, event_time_us};
use crate::midi::{MidiClient, MidiIn, MidiOut};
use clock_core::{Command, Engine, Output, State};

const CLIENT_NAME: &str = "Deeboo MIDI Sync";
const MAX_SLEEP: Duration = Duration::from_millis(5);

#[derive(Clone, Debug, PartialEq)]
pub struct BridgeSnapshot {
    pub state: State,
    pub in_port_ok: bool,
    pub out_port_ok: bool,
}

/// State shared with the MIDI callback and the timer thread.
struct Shared {
    engine: Mutex<Engine>,
    out: Mutex<Option<MidiOut>>,
    clock: HostClock,
}

struct Inner {
    shared: Arc<Shared>,
    client: Option<MidiClient>,
    in_port: Mutex<Option<MidiIn>>,
    in_ok: AtomicBool,
    out_ok: AtomicBool,
}

/// Cheap to clone; the timer thread stops when the last clone is dropped.
#[derive(Clone)]
pub struct Bridge {
    inner: Arc<Inner>,
}

/// A poisoned lock only means another thread panicked; the data is still usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Shared {
    /// Runs `f` on the engine and writes its output while still holding the engine lock,
    /// so bytes leave Out in engine order. Lock order is always engine, then out.
    fn run(&self, f: impl FnOnce(&mut Engine, u64) -> Output) {
        let mut engine = lock(&self.engine);
        let output = f(&mut engine, self.clock.now_us());
        if output.send.is_empty() {
            return;
        }
        let guard = lock(&self.out);
        if let Some(out) = guard.as_ref() {
            let packets: Vec<(u64, u8)> = output
                .send
                .iter()
                .map(|b| (self.clock.timebase.us_to_ticks(b.at_us), b.byte))
                .collect();
            if let Err(e) = out.send(&packets) {
                log::warn!("cannot write to Out: {e}");
            }
        }
    }

    /// `stamp_ticks` is the packet's host-time stamp; 0 means "unstamped".
    fn on_packet(&self, stamp_ticks: u64, msg: &[u8]) {
        let stamp_us = (stamp_ticks != 0).then(|| self.clock.timebase.ticks_to_us(stamp_ticks));
        self.run(|e, now| e.on_midi(event_time_us(stamp_us, now), msg));
    }

    fn on_tick(&self) {
        self.run(Engine::on_tick);
    }

    /// How long the timer thread may sleep.
    fn sleep_for(&self) -> Duration {
        let now = self.clock.now_us();
        lock(&self.engine)
            .next_deadline_us()
            .map_or(MAX_SLEEP, |d| {
                Duration::from_micros(d.saturating_sub(now)).min(MAX_SLEEP)
            })
    }
}

fn spawn_timer(shared: &Arc<Shared>) {
    let weak: Weak<Shared> = Arc::downgrade(shared);
    let spawned = thread::Builder::new()
        .name("deeboo-timer".into())
        .spawn(move || {
            loop {
                let Some(shared) = weak.upgrade() else { return };
                let sleep = shared.sleep_for();
                drop(shared);
                thread::sleep(sleep);
                let Some(shared) = weak.upgrade() else { return };
                shared.on_tick();
            }
        });
    if let Err(e) = spawned {
        log::error!("cannot start timer thread: {e}");
    }
}

impl Bridge {
    /// Never panics: port failures are logged and reported through the snapshot flags.
    #[must_use]
    pub fn start(in_name: &str, out_name: &str, nudge_pct: f64) -> Bridge {
        let shared = Arc::new(Shared {
            engine: Mutex::new(Engine::new(nudge_pct)),
            out: Mutex::new(None),
            clock: HostClock::new(),
        });
        let client = MidiClient::new(CLIENT_NAME)
            .inspect_err(|e| log::error!("{e}"))
            .ok();
        let bridge = Bridge {
            inner: Arc::new(Inner {
                shared: Arc::clone(&shared),
                client,
                in_port: Mutex::new(None),
                in_ok: AtomicBool::new(false),
                out_ok: AtomicBool::new(false),
            }),
        };
        bridge.open_ports(in_name, out_name);
        spawn_timer(&shared);
        bridge
    }

    fn open_ports(&self, in_name: &str, out_name: &str) {
        let inner = &self.inner;
        let out = inner.client.as_ref().and_then(|c| {
            c.open_output(out_name)
                .inspect(|_| log::info!("virtual output '{out_name}' created"))
                .inspect_err(|e| log::error!("{e}"))
                .ok()
        });
        inner.out_ok.store(out.is_some(), Ordering::Relaxed);
        *lock(&inner.shared.out) = out;
        let cb_shared = Arc::clone(&inner.shared);
        let input = inner.client.as_ref().and_then(|c| {
            c.open_input(in_name, move |ts, msg| cb_shared.on_packet(ts, msg))
                .inspect(|_| log::info!("virtual input '{in_name}' created"))
                .inspect_err(|e| log::error!("{e}"))
                .ok()
        });
        inner.in_ok.store(input.is_some(), Ordering::Relaxed);
        *lock(&inner.in_port) = input;
    }

    pub fn command(&self, cmd: Command) {
        self.inner.shared.run(|e, now| e.on_command(now, cmd));
    }

    #[must_use]
    pub fn snapshot(&self) -> BridgeSnapshot {
        let shared = &self.inner.shared;
        let state = lock(&shared.engine).state(shared.clock.now_us());
        BridgeSnapshot {
            state,
            in_port_ok: self.inner.in_ok.load(Ordering::Relaxed),
            out_port_ok: self.inner.out_ok.load(Ordering::Relaxed),
        }
    }

    /// Drops both connections and recreates them under the new names; engine state is kept.
    pub fn rename_ports(&self, in_name: &str, out_name: &str) {
        *lock(&self.inner.in_port) = None;
        *lock(&self.inner.shared.out) = None;
        self.open_ports(in_name, out_name);
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;

    #[test]
    fn without_coremidi_ports_report_down_and_commands_still_work() {
        let bridge = Bridge::start("a", "b", 4.0);
        let snap = bridge.snapshot();
        assert!(!snap.in_port_ok && !snap.out_port_ok);
        bridge.command(Command::Play);
        assert!(bridge.snapshot().state.playing);
    }
}
