---
name: midi-clock-domain
description: Use when touching MIDI clock bytes, Play/Pause transport, BPM measurement, hold-to-nudge, the beat counter or signal-loss behaviour in clock-core, the bridge or the UI.
---

# MIDI clock domain

Spec §5 is the source of truth: [docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md](../../../docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md#5-comportamiento). Do not restate it; check it.

## Bytes (24 PPQN)
| Byte | Meaning | Deeboo does |
|---|---|---|
| `0xF8` | Clock | forwards while playing; always measures BPM |
| `0xFA` | Start | never forwarded |
| `0xFB` | Continue | never forwarded |
| `0xFC` | Stop | forwarded if playing, then pause |
| `0xF2` | SPP (+2 data bytes) | never forwarded |

1 beat = 24 pulses. 125 BPM = 20 000 µs per pulse. Packets may hold several bytes or other
messages: scan every byte.

## Transport rules (spec §5.2)
- Starts paused; `playing` changes only on Play/Pause button, incoming Stop, or clock loss.
- Play: send `Start`, beat counter = 1, resume forwarding `0xF8`.
- Pause: send `Stop`, stop forwarding; BPM and In state keep being measured.
- Incoming Start/Continue/SPP: never forwarded; the slave starts only with Deeboo's Play.
- Incoming Stop while playing: forward `Stop`, pause.
- No `0xF8` for 500 ms while playing: one `Stop`, pause; returning clock does not resume.

## Nudge (spec §5.3)
- Push inserts a pulse mid-interval, Slow drops one; only while playing.
- Accumulator is integer fixed-point (strength in hundredths of a percent, threshold 10 000), never `f64`.
- Strength outside 1-10 % is rejected. Release keeps the accumulated phase offset.
- Beat counter counts output pulses. Algorithm and edge cases: [references/transport-and-nudge.md](references/transport-and-nudge.md) (read before changing nudge or transport code).

## Checklist
- [ ] Every `0xF8` in a packet is counted; nothing else is.
- [ ] No division by zero with equal timestamps; BPM never NaN.
- [ ] Pause, incoming Stop and clock loss clear the pending pulse and accumulator.
- [ ] Constants (`24`, `500_000` µs) come from `clock-core`, not literals.
- [ ] Behaviour change updates this skill and its reference.
