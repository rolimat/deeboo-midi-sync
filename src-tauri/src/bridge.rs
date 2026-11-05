//! Virtual MIDI ports wired to the clock engine, plus the timer thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::Duration;

use clock_core::{Command, Engine, Output, State};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::clock::MonoClock;

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
    out: Mutex<Option<MidiOutputConnection>>,
    clock: MonoClock,
}

struct Inner {
    shared: Arc<Shared>,
    in_conn: Mutex<Option<MidiInputConnection<()>>>,
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
    fn send(&self, output: &Output) {
        if output.send.is_empty() {
            return;
        }
        let mut guard = lock(&self.out);
        if let Some(conn) = guard.as_mut() {
            for b in &output.send {
                if let Err(e) = conn.send(&[*b]) {
                    log::warn!("cannot write to Out: {e}");
                }
            }
        }
    }

    fn on_midi(&self, msg: &[u8]) {
        let now = self.clock.now_us();
        let output = lock(&self.engine).on_midi(now, msg);
        self.send(&output);
    }

    fn on_tick(&self) {
        let now = self.clock.now_us();
        let output = lock(&self.engine).on_tick(now);
        self.send(&output);
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

#[cfg(unix)]
fn open_input(name: &str, shared: &Arc<Shared>) -> Option<MidiInputConnection<()>> {
    use midir::os::unix::VirtualInput;

    let mut input = match MidiInput::new(CLIENT_NAME) {
        Ok(i) => i,
        Err(e) => {
            log::error!("cannot create MIDI input client: {e}");
            return None;
        }
    };
    input.ignore(Ignore::None);
    let cb_shared = Arc::clone(shared);
    match input.create_virtual(name, move |_ts, msg, ()| cb_shared.on_midi(msg), ()) {
        Ok(conn) => {
            log::info!("virtual input '{name}' created");
            Some(conn)
        }
        Err(e) => {
            log::error!("cannot create virtual input '{name}': {e}");
            None
        }
    }
}

#[cfg(not(unix))]
fn open_input(_name: &str, _shared: &Arc<Shared>) -> Option<MidiInputConnection<()>> {
    log::error!("virtual MIDI ports are not supported on this platform");
    None
}

#[cfg(unix)]
fn open_output(name: &str) -> Option<MidiOutputConnection> {
    use midir::os::unix::VirtualOutput;

    let output = match MidiOutput::new(CLIENT_NAME) {
        Ok(o) => o,
        Err(e) => {
            log::error!("cannot create MIDI output client: {e}");
            return None;
        }
    };
    match output.create_virtual(name) {
        Ok(conn) => {
            log::info!("virtual output '{name}' created");
            Some(conn)
        }
        Err(e) => {
            log::error!("cannot create virtual output '{name}': {e}");
            None
        }
    }
}

#[cfg(not(unix))]
fn open_output(_name: &str) -> Option<MidiOutputConnection> {
    log::error!("virtual MIDI ports are not supported on this platform");
    None
}

impl Bridge {
    /// Never panics: port failures are logged and reported through the snapshot flags.
    #[must_use]
    pub fn start(in_name: &str, out_name: &str, nudge_pct: f64) -> Bridge {
        let shared = Arc::new(Shared {
            engine: Mutex::new(Engine::new(nudge_pct)),
            out: Mutex::new(None),
            clock: MonoClock::new(),
        });
        let bridge = Bridge {
            inner: Arc::new(Inner {
                shared: Arc::clone(&shared),
                in_conn: Mutex::new(None),
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
        let out = open_output(out_name);
        inner.out_ok.store(out.is_some(), Ordering::Relaxed);
        *lock(&inner.shared.out) = out;
        let input = open_input(in_name, &inner.shared);
        inner.in_ok.store(input.is_some(), Ordering::Relaxed);
        *lock(&inner.in_conn) = input;
    }

    pub fn command(&self, cmd: Command) {
        let shared = &self.inner.shared;
        let now = shared.clock.now_us();
        let output = lock(&shared.engine).on_command(now, cmd);
        shared.send(&output);
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
        drop(lock(&self.inner.in_conn).take());
        drop(lock(&self.inner.shared.out).take());
        self.open_ports(in_name, out_name);
    }
}
