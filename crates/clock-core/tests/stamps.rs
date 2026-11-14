//! Every output byte carries the time it should leave at; stamps never decrease.

use clock_core::{Command, Direction, Engine, TimedByte};

const T0: u64 = 1_000_000;
const IV: u64 = 20_000;

fn tb(at_us: u64, byte: u8) -> TimedByte {
    TimedByte { at_us, byte }
}

fn playing() -> Engine {
    let mut e = Engine::new(4.0);
    e.on_command(T0, Command::Play);
    e
}

/// Playing with a push nudge after 25 pulses; returns the engine and the last pulse time.
fn pushed() -> (Engine, u64) {
    let mut e = playing();
    e.on_command(T0, Command::NudgeStart(Direction::Push));
    for i in 0..25 {
        e.on_midi(T0 + i * IV, &[0xF8]);
    }
    (e, T0 + 24 * IV)
}

#[test]
fn forwarded_clock_keeps_its_time() {
    let mut e = playing();
    assert_eq!(e.on_midi(T0 + 100, &[0xF8]).send, vec![tb(T0 + 100, 0xF8)]);
}

#[test]
fn packet_bytes_share_the_packet_time() {
    let mut e = playing();
    assert_eq!(
        e.on_midi(T0 + 5, &[0xF8, 0xF8]).send,
        vec![tb(T0 + 5, 0xF8), tb(T0 + 5, 0xF8)]
    );
}

#[test]
fn commands_incoming_stop_and_loss_stop_use_their_time() {
    let mut e = Engine::new(4.0);
    assert_eq!(e.on_command(T0, Command::Play).send, vec![tb(T0, 0xFA)]);
    assert_eq!(
        e.on_command(T0 + 100_000, Command::Pause).send,
        vec![tb(T0 + 100_000, 0xFC)]
    );

    let mut e = playing();
    assert_eq!(e.on_midi(T0 + 10, &[0xFC]).send, vec![tb(T0 + 10, 0xFC)]);

    let mut e = playing();
    let t = T0 + 3 * IV;
    for i in 0..4 {
        e.on_midi(T0 + i * IV, &[0xF8]);
    }
    assert_eq!(e.on_tick(t + 500_001).send, vec![tb(t + 500_001, 0xFC)]);
}

#[test]
fn extra_pulse_is_stamped_at_its_due_time_even_if_tick_is_late() {
    let (mut e, t) = pushed();
    assert_eq!(e.on_tick(t + 11_500).send, vec![tb(t + 10_000, 0xF8)]);
}

#[test]
fn due_extra_pulse_is_sent_before_the_next_pulse_when_no_tick_ran() {
    let (mut e, t) = pushed();
    assert_eq!(
        e.on_midi(t + IV, &[0xF8]).send,
        vec![tb(t + 10_000, 0xF8), tb(t + IV, 0xF8)]
    );
}

#[test]
fn clock_stamped_before_play_is_not_forwarded() {
    let mut e = Engine::new(4.0);
    assert_eq!(
        e.on_command(T0 + 500, Command::Play).send,
        vec![tb(T0 + 500, 0xFA)]
    );
    assert_eq!(e.on_midi(T0 + 450, &[0xF8]).send, vec![]);
    assert_eq!(
        e.on_midi(T0 + 450 + IV, &[0xF8]).send,
        vec![tb(T0 + 450 + IV, 0xF8)]
    );
    let s = e.state(T0 + 450 + IV);
    assert_eq!(s.beat, Some(1));
    assert!((s.bpm.unwrap() - 125.0).abs() < 0.05);
}

#[test]
fn pulse_time_slightly_before_last_tick_is_accepted() {
    let mut e = playing();
    let mut stamps = Vec::new();
    for i in 0..48 {
        e.on_tick(T0 + i * IV + 80);
        for b in e.on_midi(T0 + i * IV, &[0xF8]).send {
            assert_eq!(b.at_us, T0 + i * IV);
            stamps.push(b.at_us);
        }
    }
    assert!(stamps.windows(2).all(|w| w[0] <= w[1]));
    let s = e.state(T0 + 47 * IV);
    assert!((s.bpm.unwrap() - 125.0).abs() < 0.05);
    assert_eq!(s.input_beats, 2);
}
