//! Measures the jitter of incoming MIDI clock (0xF8) as a receiver sees it.
//!
//! Timestamps come from midir's callback (microseconds, CoreMIDI-derived) on purpose.

use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use jitter_probe::summarize;
use midir::{Ignore, MidiInput};

const USAGE: &str = "usage: jitter-probe [--seconds N] [--port-name NAME | --connect NAME]
  --seconds N      measure for N seconds (default 30)
  --port-name NAME create a virtual MIDI destination NAME (default \"Deeboo Probe\");
                   point the master clock's output at it (direct run)
  --connect NAME   instead connect to the existing MIDI source whose name contains NAME,
                   e.g. \"Deeboo Clock Sync Out\" (via-Deeboo run)";

struct Args {
    seconds: u64,
    port_name: String,
    connect: Option<String>,
}

fn parse_args(mut it: impl Iterator<Item = String>) -> Option<Args> {
    let mut a = Args {
        seconds: 30,
        port_name: "Deeboo Probe".into(),
        connect: None,
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--seconds" => a.seconds = it.next()?.parse().ok().filter(|s| *s > 0)?,
            "--port-name" => a.port_name = it.next()?,
            "--connect" => a.connect = Some(it.next()?),
            _ => return None,
        }
    }
    Some(a)
}

fn main() -> ExitCode {
    let Some(args) = parse_args(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("jitter-probe: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let stamps: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let cb_stamps = Arc::clone(&stamps);
    let callback = move |ts: u64, msg: &[u8], (): &mut ()| {
        #[allow(clippy::naive_bytecount)] // a handful of bytes per message
        let ticks = msg.iter().filter(|b| **b == 0xF8).count();
        if ticks > 0 {
            let mut v = cb_stamps
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            v.extend(std::iter::repeat_n(ts, ticks));
        }
    };

    let mut input = MidiInput::new("deeboo-jitter-probe").map_err(|e| e.to_string())?;
    input.ignore(Ignore::None);
    let conn = match &args.connect {
        Some(name) => {
            let port = input
                .ports()
                .into_iter()
                .find(|p| input.port_name(p).is_ok_and(|n| n.contains(name.as_str())))
                .ok_or_else(|| format!("no MIDI source matching '{name}'"))?;
            println!("connected to source '{name}'");
            input
                .connect(&port, "jitter-probe", callback, ())
                .map_err(|e| e.to_string())?
        }
        None => create_virtual(input, &args.port_name, callback)?,
    };

    println!("measuring for {} s ...", args.seconds);
    thread::sleep(Duration::from_secs(args.seconds));
    drop(conn);

    let stamps = stamps
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    #[allow(clippy::cast_precision_loss)]
    let intervals: Vec<f64> = stamps.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    let Some(s) = summarize(&intervals) else {
        println!("no clock received ({} pulses)", stamps.len());
        return Ok(());
    };
    println!("intervals: {}", s.count);
    println!("mean:      {:.1} us", s.mean_us);
    println!("stddev:    {:.1} us", s.stddev_us);
    println!("p99 |dev|: {:.1} us", s.p99_abs_dev_us);
    println!("bpm:       {:.2}", 60e6 / (24.0 * s.mean_us));
    Ok(())
}

#[cfg(unix)]
fn create_virtual(
    input: MidiInput,
    name: &str,
    callback: impl FnMut(u64, &[u8], &mut ()) + Send + 'static,
) -> Result<midir::MidiInputConnection<()>, String> {
    use midir::os::unix::VirtualInput;
    let conn = input
        .create_virtual(name, callback, ())
        .map_err(|e| e.to_string())?;
    println!("virtual input '{name}' created");
    Ok(conn)
}

#[cfg(not(unix))]
fn create_virtual(
    _input: MidiInput,
    _name: &str,
    _callback: impl FnMut(u64, &[u8], &mut ()) + Send + 'static,
) -> Result<midir::MidiInputConnection<()>, String> {
    Err("virtual MIDI ports are not supported on this platform; use --connect".into())
}
