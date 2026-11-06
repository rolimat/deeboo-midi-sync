mod common;

use clock_core::{Command, Direction, Engine, Output};
use common::run_with_ticks;

const T0: u64 = 1_000_000;
const IV: u64 = 20_000; // 125 BPM

fn playing_engine() -> Engine {
    let mut e = Engine::new(4.0);
    e.on_command(T0, Command::Play);
    e
}

#[test]
fn push_adds_one_pulse_per_25_inputs() {
    let mut e = playing_engine();
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut e, T0, 100, IV), 104);
}

#[test]
fn extra_pulse_lands_mid_interval() {
    let mut e = playing_engine();
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    for i in 0..25 {
        e.on_midi(T0 + i * IV, &[0xF8]);
    }
    let t = T0 + 24 * IV;
    assert_eq!(e.next_deadline_us(), Some(t + 10_000));
}

#[test]
fn slow_drops_one_pulse_per_25_inputs() {
    let mut e = playing_engine();
    e.on_command(T0, Command::NudgeStart(Direction::Slow));
    assert_eq!(run_with_ticks(&mut e, T0, 100, IV), 96);
}

#[test]
fn release_returns_to_one_to_one_and_keeps_offset() {
    let mut e = playing_engine();
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    let first = run_with_ticks(&mut e, T0, 50, IV);
    assert_eq!(first, 52);
    let t1 = T0 + 50 * IV;
    e.on_command(t1, Command::NudgeStop);
    let second = run_with_ticks(&mut e, t1, 100, IV);
    assert_eq!(second, 100);
    assert_eq!(first + second, 152);
}

#[test]
fn nudge_ignored_while_paused() {
    let mut e = Engine::new(4.0);
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut e, T0, 50, IV), 0);
    assert_eq!(e.next_deadline_us(), None);
}

#[test]
fn signal_loss_during_push_cancels_pending_pulse() {
    let mut e = playing_engine();
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    for i in 0..25 {
        e.on_midi(T0 + i * IV, &[0xF8]);
    }
    let last = T0 + 24 * IV;
    assert_eq!(e.next_deadline_us(), Some(last + 10_000));
    assert_eq!(e.on_tick(last + 500_001).send, vec![0xFC]);
    assert_eq!(e.on_tick(last + 600_000), Output::default());
    assert_eq!(e.next_deadline_us(), None);
}

#[test]
fn strength_outside_range_is_ignored() {
    let mut e = playing_engine();
    e.on_command(T0, Command::SetNudgeStrength(0.5));
    e.on_command(T0, Command::SetNudgeStrength(11.0));
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut e, T0, 100, IV), 104);

    let mut e = playing_engine();
    e.on_command(T0, Command::SetNudgeStrength(10.0));
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut e, T0, 100, IV), 110);
}

#[test]
fn new_clamps_out_of_range_strength() {
    let mut huge = Engine::new(1e6);
    huge.on_command(T0, Command::Play);
    huge.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut huge, T0, 100, IV), 110);

    let mut nan = Engine::new(f64::NAN);
    nan.on_command(T0, Command::Play);
    nan.on_command(T0, Command::NudgeStart(Direction::Push));
    assert_eq!(run_with_ticks(&mut nan, T0, 100, IV), 104);
}
