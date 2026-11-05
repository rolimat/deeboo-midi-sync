mod common;

use clock_core::Engine;
use common::clock_pulses;

#[test]
fn bpm_from_steady_clock() {
    let mut e = Engine::new(4.0);
    clock_pulses(&mut e, 0, 48, 20_000);
    let s = e.state(47 * 20_000);
    assert!((s.bpm.unwrap() - 125.0).abs() < 0.05);
    assert!(s.in_ok);
    assert_eq!(s.input_beats, 2);
}

#[test]
fn counts_every_clock_byte_in_mixed_packet() {
    let mut e = Engine::new(4.0);
    e.on_midi(0, &[0xF8, 0xF8]);
    e.on_midi(20_000, &[0x90, 60, 100, 0xF8]);
    e.on_midi(40_000, &[0xF0, 0x7E, 0xF8, 0x01, 0xF7]); // realtime inside SysEx
    for i in 0..20 {
        e.on_midi(60_000 + i * 20_000, &[0xF8]);
    }
    assert_eq!(e.state(440_000).input_beats, 1); // 4 + 20 = 24 pulses
}

#[test]
fn no_signal_after_500ms() {
    let mut e = Engine::new(4.0);
    clock_pulses(&mut e, 0, 24, 20_000);
    let s = e.state(23 * 20_000 + 500_001);
    assert!(!s.in_ok);
    assert_eq!(s.bpm, None);
}

#[test]
fn bpm_follows_tempo_change_within_one_beat() {
    // 125 -> 100 BPM (25_000 us)
    let mut e = Engine::new(4.0);
    clock_pulses(&mut e, 0, 48, 20_000);
    clock_pulses(&mut e, 48 * 20_000, 25, 25_000);
    let bpm = e.state(48 * 20_000 + 24 * 25_000).bpm.unwrap();
    assert!((bpm - 100.0).abs() < 0.05);
}

#[test]
fn zero_interval_does_not_produce_nan() {
    let mut e = Engine::new(4.0);
    e.on_midi(1_000, &[0xF8, 0xF8]);
    assert!(e.state(1_000).bpm.is_none_or(f64::is_finite));
}

#[test]
fn paused_engine_forwards_nothing() {
    let mut e = Engine::new(4.0);
    assert_eq!(clock_pulses(&mut e, 0, 24, 20_000), Vec::<u8>::new());
}
