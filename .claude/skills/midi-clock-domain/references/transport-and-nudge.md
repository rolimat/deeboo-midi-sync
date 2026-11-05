# Transport and nudge detail

Spec: §5.2 (transport), §5.3 (nudge), §5.4 (BPM). Plan: Tasks 3-5.

## BPM
`bpm = 60e6 / (24 * mean_interval_us)` over the last 24 intervals. Intervals <= 0 are ignored, so
two `0xF8` with the same timestamp do not produce NaN. After a tempo change (125 -> 100) the
value converges within one beat. Fewer than 2 pulses, or no signal: `None` (UI shows `—`).

## Nudge algorithm (while playing and nudging)
```text
on each incoming 0xF8:
    acc += strength_hundredths            # 4 % = 400
    if acc >= 10_000:
        acc -= 10_000
        Push: forward this pulse, pending_extra = now + interval_us / 2   (skip if interval unknown)
        Slow: drop this pulse (not forwarded, not counted)
on_tick(now): if pending_extra <= now -> send 0xF8, count it, clear pending_extra
NudgeStop: acc = 0; an already pending extra pulse still fires
Pause / incoming Stop / clock loss: clear pending_extra and acc, nudge = None
SetNudgeStrength(p): outside 1.0..=10.0 is ignored, previous value kept
```
At 4 % and 125 BPM: 100 input pulses give 104 (Push) or 96 (Slow) outputs; the extra pulse lands
10 000 µs after the 25th input pulse.

## Edge cases
- Signal loss with a pending extra pulse: output is exactly one `0xFC`, later ticks are empty.
- `next_deadline_us()` is the earlier of the pending extra pulse and the signal-loss deadline
  (last pulse + 500 000 µs).
- Nudge commands while paused are ignored and leave no deadline.
- The UI must send `NudgeStop` on pointerup, pointerleave and window blur, and the popover hiding
  does the same in Rust; otherwise the slave drifts forever.
- Input beats (`input_beats`, received pulses / 24) drive the menu bar dot blink (its colour follows `out_ok`: green playing, red otherwise; steady red without input); the displayed beat
  1-4 counts output pulses.
