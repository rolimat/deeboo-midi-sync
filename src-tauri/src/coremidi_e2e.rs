//! End-to-end tests against the real `CoreMIDI` server: a second client plays the master and the
//! slave around a started `Bridge` and checks the timestamps that come out.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clock_core::{Command, Direction};
use coremidi::{Client, Destination, Destinations, OutputPort, PacketBuffer, Source, Sources};

use crate::bridge::Bridge;
use crate::clock::HostClock;

const IV: u64 = 20_000;
type Received = Arc<Mutex<Vec<(u64, Vec<u8>)>>>;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn unique(kind: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("Deeboo Test {kind} {}-{n}", std::process::id())
}

struct Harness {
    bridge: Bridge,
    clock: HostClock,
    _client: Client,
    port: OutputPort,
    dest: Destination,
    received: Received,
    _input: coremidi::InputPort,
}

fn wait_for<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(v) = f() {
            return v;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "port not found");
        thread::sleep(Duration::from_millis(5));
    }
}

impl Harness {
    fn new() -> Self {
        let (in_name, out_name) = (unique("In"), unique("Out"));
        let bridge = Bridge::start(&in_name, &out_name, 4.0);
        Self::around(bridge, &in_name, &out_name)
    }

    fn around(bridge: Bridge, in_name: &str, out_name: &str) -> Self {
        let client = Client::new("Deeboo Test Client").expect("client");
        let received: Received = Arc::default();
        let sink = Arc::clone(&received);
        let input = client
            .input_port("test in", move |list: &coremidi::PacketList| {
                let mut v = sink.lock().unwrap();
                for p in list {
                    v.push((p.timestamp(), p.data().to_vec()));
                }
            })
            .expect("input port");
        let source = wait_for(|| Source::from_name(out_name));
        input.connect_source(&source).expect("connect");
        let dest = wait_for(|| {
            Destinations
                .into_iter()
                .find(|d| d.name().as_deref() == Some(in_name))
        });
        let port = client.output_port("test out").expect("output port");
        Self {
            bridge,
            clock: HostClock::new(),
            _client: client,
            port,
            dest,
            received,
            _input: input,
        }
    }

    fn send_at(&self, at_us: u64, bytes: &[u8]) {
        self.sleep_until(at_us);
        let buf = PacketBuffer::new(self.clock.us_to_ticks(at_us), bytes);
        self.port.send(&self.dest, &buf).expect("send");
    }

    fn sleep_until(&self, at_us: u64) {
        while self.clock.now_us() < at_us {
            thread::sleep(Duration::from_micros(200));
        }
    }

    fn pulses(&self, n: u64, t0: u64) {
        for k in 0..n {
            self.send_at(t0 + k * IV, &[0xF8]);
        }
    }

    /// Received `(stamp in µs, byte)` once `count` bytes arrived (or 5 s passed).
    fn bytes(&self, count: usize) -> Vec<(u64, u8)> {
        let start = Instant::now();
        loop {
            let flat: Vec<(u64, u8)> = self
                .received
                .lock()
                .unwrap()
                .iter()
                .flat_map(|(ts, d)| d.iter().map(|b| (self.clock.ticks_to_us(*ts), *b)))
                .collect();
            if flat.len() >= count || start.elapsed() > Duration::from_secs(5) {
                return flat;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

fn t0(h: &Harness) -> u64 {
    h.clock.now_us() + 20_000
}

fn near(a: u64, b: u64, tol: u64) -> bool {
    a.abs_diff(b) <= tol
}

#[test]
fn forwards_each_pulse_with_the_input_stamp() {
    let h = Harness::new();
    h.bridge.command(Command::Play);
    let t0 = t0(&h);
    h.pulses(30, t0);
    let got = h.bytes(31);
    let bytes: Vec<u8> = got.iter().map(|(_, b)| *b).collect();
    let mut want = vec![0xFA];
    want.extend([0xF8; 30]);
    assert_eq!(bytes, want);
    for (k, (ts, _)) in got[1..].iter().enumerate() {
        let k = k as u64;
        assert!(
            near(*ts, t0 + k * IV, 1),
            "pulse {k}: {ts} vs {}",
            t0 + k * IV
        );
    }
    assert!(got[0].0 <= got[1].0);
}

#[test]
fn push_extra_pulse_is_stamped_mid_interval() {
    let h = Harness::new();
    h.bridge.command(Command::Play);
    h.bridge.command(Command::NudgeStart(Direction::Push));
    let t0 = t0(&h);
    h.pulses(30, t0);
    let got = h.bytes(32);
    let pulses: Vec<u64> = got
        .iter()
        .filter(|(_, b)| *b == 0xF8)
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(pulses.len(), 31);
    let want = t0 + 24 * IV + 10_000;
    assert!(near(pulses[25], want, 2), "{} vs {want}", pulses[25]);
    assert!(got.windows(2).all(|w| w[0].0 <= w[1].0), "stamps decrease");
}

#[test]
fn zero_stamp_input_is_forwarded_with_arrival_time() {
    let h = Harness::new();
    h.bridge.command(Command::Play);
    let s = h.clock.now_us();
    let buf = PacketBuffer::new(0, &[0xF8]);
    h.port.send(&h.dest, &buf).expect("send");
    let got = h.bytes(2);
    let (ts, _) = got.iter().find(|(_, b)| *b == 0xF8).expect("pulse");
    assert!((s..=s + 50_000).contains(ts), "{ts} vs {s}");
}

#[test]
fn packets_in_one_list_keep_their_own_stamps() {
    let h = Harness::new();
    h.bridge.command(Command::Play);
    // The engine raises stamps older than Play to the Play time, so let Play age first.
    thread::sleep(Duration::from_millis(5));
    let now = h.clock.now_us();
    let (a, b) = (now - 2_000, now - 1_000);
    let mut buf = PacketBuffer::new(h.clock.us_to_ticks(a), &[0xF8]);
    buf.push_data(h.clock.us_to_ticks(b), &[0xF8]);
    h.port.send(&h.dest, &buf).expect("send");
    let got = h.bytes(3);
    let pulses: Vec<u64> = got
        .iter()
        .filter(|(_, b)| *b == 0xF8)
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(pulses.len(), 2);
    assert!(near(pulses[0], a, 1) && near(pulses[1], b, 1), "{pulses:?}");
}

#[test]
fn renamed_ports_forward_and_old_ones_disappear() {
    let (in_name, out_name) = (unique("In"), unique("Out"));
    let bridge = Bridge::start(&in_name, &out_name, 4.0);
    let (new_in, new_out) = (unique("In"), unique("Out"));
    bridge.rename_ports(&new_in, &new_out);
    thread::sleep(Duration::from_millis(50));
    assert!(
        Destinations
            .into_iter()
            .all(|d| d.name().as_deref() != Some(in_name.as_str()))
    );
    assert!(
        Sources
            .into_iter()
            .all(|s| s.display_name().as_deref() != Some(out_name.as_str()))
    );
    let h = Harness::around(bridge, &new_in, &new_out);
    h.bridge.command(Command::Play);
    let t0 = t0(&h);
    h.pulses(5, t0);
    let got = h.bytes(6);
    assert_eq!(got.len(), 6);
    for (k, (ts, _)) in got[1..].iter().enumerate() {
        assert!(near(*ts, t0 + k as u64 * IV, 1));
    }
}
