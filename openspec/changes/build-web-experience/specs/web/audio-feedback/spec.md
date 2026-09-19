## Purpose

Adds optional, tasteful tactile sound (micro-clicks and clean tones for UI events) that makes the interface feel tactile without ever surprising, distracting, or excluding users.

## ADDED Requirements

### Requirement: Off by default, explicit opt-in
Audio SHALL be off by default. It SHALL be enabled only by an explicit user action (settings toggle, palette action, or the `m` shortcut), and the choice SHALL persist locally.

#### Scenario: First visit is silent
- **WHEN** a first-time visitor uses any tool
- **THEN** no sound plays

### Requirement: Synthesized sounds, no audio files
All sounds SHALL be synthesized at runtime with the Web Audio API (oscillators, noise, and gain envelopes). The app SHALL ship no audio asset files, and the audio module SHALL be at most 6 KB compressed and loaded only when audio is enabled.

#### Scenario: No audio downloads
- **WHEN** audio is enabled
- **THEN** no network request for an audio file occurs

### Requirement: Event-to-sound mapping
The app SHALL map a small fixed set of events to distinct sounds: key/toggle click, projection or mode switch, successful computation, warning result, error result, palette open, and copy-to-clipboard. Each sound SHALL be at most 120 ms long and SHALL peak at or below -12 dBFS at 100% volume.

#### Scenario: Error tone differs from success tone
- **WHEN** a computation fails validation
- **THEN** the error sound plays, and it is distinguishable in pitch and envelope from the success sound

### Requirement: Rate limiting
Sounds SHALL be rate limited so that no more than 8 sounds play per second and live recompute while typing plays at most one "computed" sound per 500 ms.

#### Scenario: Typing fast
- **WHEN** a user types 20 characters per second in an input with audio on
- **THEN** at most 2 computed sounds play per second

### Requirement: Mute and volume
Once audio is enabled, a mute toggle SHALL be reachable in one action from every page (header control and `m` shortcut); while audio is off, no audio control SHALL appear in the header, and a volume slider SHALL be available in settings. Muting SHALL silence sound within 50 ms.

#### Scenario: Mute during playback
- **WHEN** a user presses `m` while a tone is playing
- **THEN** the tone stops within 50 ms

### Requirement: Autoplay-policy compliance
The audio context SHALL be created or resumed only in response to a user gesture, and failures to start audio SHALL be silent (no errors shown, no retries in a loop).

#### Scenario: Blocked audio context
- **WHEN** the browser refuses to start audio
- **THEN** the app continues silently and the audio toggle shows "Audio unavailable in this browser"

### Requirement: Sound is never the only signal
Every event that produces a sound SHALL also produce a visual indication, so users who are deaf, hard of hearing, or muted receive the same information.

#### Scenario: Warning visible
- **WHEN** a warning result plays its sound
- **THEN** the warning is also shown in the result panel
