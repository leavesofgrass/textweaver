# ADR-0004: Rate, pitch, and volume

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as decided. Rate, pitch, volume, and the speed presets are the `[speech]` settings `rate`, `pitch`, `volume`, and `speed_presets`; each engine crate documents its own mapping. DECtalk takes words per minute natively, from 75 to 600 (ADR-0021).

## Context

Every engine has its own scale: espeak-ng takes words per minute, SAPI and WinRT take -10..10 or a multiplier, AVSpeech takes 0.0..1.0, Omnivox and Emacspeak servers take their own numbers. Star stored `tts_rate` in wpm (default 265, hotkeys clamp to 50–600, step ±20) and `tts_volume` as a 0.0–1.0 float, and had no pitch control at all.

## Decision

- **Rate** is canonical words per minute: `Rate::Wpm(u16)`, default 265, clamped to 50..=900 (wider than Star's 600 for fast screen-reader users). It serializes as a plain number (`rate = 265`). Rate keys step by 20 wpm, as in Star. Speed presets (`skim` 350, `normal` 265, `study` 200, `slow` 150) are kept.
- **Pitch** is a semitone offset from the voice's own pitch: `Pitch::Semitones(i8)`, default 0, clamped to ±12, step 1. It serializes as a plain number. New in textweaver.
- **Volume** is a percentage: `Volume(u8)`, 0..=100, default 100, step 5. Star's float is multiplied by 100 on migration.
- **Each backend maps** the canonical values onto its own scale in `set_params`, and reports what it achieves through `effective_wpm()`. The timer pacer (ADR-0003) uses `effective_wpm()`, not the requested rate, so an engine that caps or rounds the rate does not desynchronize the highlight.
- Engines without `PITCH` or `VOLUME` capability ignore those parameters; the app announces "pitch not supported by this voice" instead of pretending.
- Single characters (`speak_char`) use a scaled rate and raise pitch for capitals when `[speech] caps = "pitch"` (Omnivox's approach); other `CapsIndication` values are a tone or saying "cap".

## Consequences

- Settings files are engine-independent; switching backends keeps the user's speed.
- Backends own their mapping tables and document them in their module docs.

## See also

- [Speech engines and voices](../speech.md): changing rate, pitch, and volume.
- [Settings](../settings.md#speech): the `[speech]` settings.
- [Keyboard reference](../keyboard.md#voice): the voice keys.
- [Documentation index](../README.md)
