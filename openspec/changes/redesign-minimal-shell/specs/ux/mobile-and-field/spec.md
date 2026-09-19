## REMOVED Requirements

### Requirement: Sunlight and night use
**Reason**: The `sunlight` and `night` modes are removed in `web/visual-theme`, so a requirement that they be one tap from every header cannot hold. The header now carries one light/dark control.

**Migration**: Outdoor legibility rests on `paper`'s contrast (black-on-white at 4.5:1 or better), the 48 px targets and 8 px spacing of "Targets and spacing", and the browser's own contrast and zoom settings, which the site no longer overrides. Dark-adapted use rests on `ink`. This is a real reduction for cockpit use, and it is recorded here rather than implied: `ink` is not a red-light mode, and a pilot who needs one should dim the device.

## ADDED Requirements

### Requirement: One display control in the field
The light/dark control SHALL be reachable in one tap from every page, SHALL meet the 48 px target size, and SHALL take effect immediately with no bright frame in either direction.

#### Scenario: One-tap switch outdoors
- **WHEN** a user taps the display control on a phone
- **THEN** the page switches modes immediately, with no bright frame and no navigation
