mod common;

use clock_core::{Command, Engine, Output};
use common::clock_pulses;

fn playing_engine() -> Engine {
    let mut e = Engine::new(4.0);
    assert_eq!(e.on_command(0, Command::Play).send, vec![0xFA]);
    e
}

#[test]
fn play_sends_start_then_forwards_clock() {
    let mut e = Engine::new(4.0);
    assert_eq!(e.on_command(0, Command::Play).send, vec![0xFA]);
    assert_eq!(clock_pulses(&mut e, 0, 3, 20_000), vec![0xF8; 3]);
    assert_eq!(e.state(60_000).beat, Some(1));
}

#[test]
fn beat_counter_wraps_every_four_beats() {
    let mut e = playing_engine();
    clock_pulses(&mut e, 0, 24 * 5, 20_000);
    assert_eq!(e.state(120 * 20_000).beat, Some(2));
}

#[test]
fn pause_sends_stop_and_halts_forwarding() {
    let mut e = playing_engine();
    clock_pulses(&mut e, 0, 5, 20_000);
    assert_eq!(e.on_command(100_000, Command::Pause).send, vec![0xFC]);
    assert_eq!(clock_pulses(&mut e, 120_000, 3, 20_000), Vec::<u8>::new());
    assert_eq!(e.state(180_000).beat, None);
}

#[test]
fn repeated_play_or_pause_is_noop() {
    let mut e = Engine::new(4.0);
    assert_eq!(e.on_command(0, Command::Pause).send, Vec::<u8>::new());
    e.on_command(0, Command::Play);
    assert_eq!(e.on_command(10, Command::Play).send, Vec::<u8>::new());
    e.on_command(20, Command::Pause);
    assert_eq!(e.on_command(30, Command::Pause).send, Vec::<u8>::new());
}

#[test]
fn incoming_stop_forwards_and_pauses() {
    let mut e = playing_engine();
    assert_eq!(e.on_midi(10, &[0xFC]), Output { send: vec![0xFC] });
    assert!(!e.state(10).playing);
}

#[test]
fn incoming_stop_while_paused_is_not_forwarded() {
    let mut e = Engine::new(4.0);
    assert_eq!(e.on_midi(10, &[0xFC]).send, Vec::<u8>::new());
}

#[test]
fn incoming_start_continue_spp_never_forwarded() {
    let mut e = playing_engine();
    for packet in [&[0xFA][..], &[0xFB], &[0xF2, 0x10, 0x00]] {
        assert_eq!(e.on_midi(10, packet).send, Vec::<u8>::new());
        assert!(e.state(10).playing);
    }
}

#[test]
fn play_resets_beat_counter() {
    let mut e = playing_engine();
    clock_pulses(&mut e, 0, 30, 20_000);
    e.on_command(600_000, Command::Pause);
    e.on_command(600_000, Command::Play);
    assert_eq!(e.state(600_000).beat, Some(1));
}

#[test]
fn clock_loss_while_playing_sends_single_stop() {
    let mut e = playing_engine();
    clock_pulses(&mut e, 0, 10, 20_000);
    let t = 9 * 20_000;
    assert_eq!(e.next_deadline_us(), Some(t + 500_000));
    assert_eq!(e.on_tick(t + 500_000).send, Vec::<u8>::new());
    assert_eq!(e.on_tick(t + 500_001).send, vec![0xFC]);
    assert_eq!(e.on_tick(t + 600_000).send, Vec::<u8>::new());
    assert!(!e.state(t + 600_000).playing);
    assert_eq!(e.next_deadline_us(), None);
}

#[test]
fn clock_loss_measured_from_play_when_no_clock_arrived() {
    let mut e = Engine::new(4.0);
    e.on_command(1_000, Command::Play);
    assert_eq!(e.next_deadline_us(), Some(501_000));
    assert_eq!(e.on_tick(501_001).send, vec![0xFC]);
}

#[test]
fn clock_returning_does_not_resume() {
    let mut e = playing_engine();
    clock_pulses(&mut e, 0, 10, 20_000);
    e.on_tick(9 * 20_000 + 500_001);
    assert_eq!(clock_pulses(&mut e, 800_000, 5, 20_000), Vec::<u8>::new());
    assert!(!e.state(900_000).playing);
}

#[test]
fn paused_engine_has_no_deadline() {
    let mut e = Engine::new(4.0);
    clock_pulses(&mut e, 0, 5, 20_000);
    assert_eq!(e.next_deadline_us(), None);
}
