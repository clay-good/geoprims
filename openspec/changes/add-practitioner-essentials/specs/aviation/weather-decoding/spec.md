## Purpose

Turns pasted aviation weather text into plain language and structured values that feed the altimetry and wind tools. It never fetches weather, and always states that the user is responsible for obtaining a current, official briefing.

## ADDED Requirements

### Requirement: METAR and SPECI decoding
The decoder SHALL parse METAR and SPECI reports per WMO FM 15 with US conventions from FAA Order JO 7900.5, including:
- **Header:** station, time, AUTO/COR.
- **Wind:** `dddffKT`, gusts `G`, `VRB`, `00000KT`, and variable range `dddVddd`.
- **Visibility:** statute-mile fractions with spaces (`1 1/2SM`), `M1/4SM` (less than), `P6SM` (more than), RVR.
- **Weather and cloud:** weather phenomena with intensity and descriptors, cloud layers and vertical visibility, and `CAVOK` (outside the US).
- **Temperature and dew point:** with `M` negatives.
- **Altimeter:** `A` and `Q` groups.
- **Remarks:** `AO1`/`AO2`, `SLP` (reconstructing the leading 9 or 10 hundreds to the value nearest 1000 hPa), `T` group precise temperatures (sign digit `1` = negative), `PK WND`, `WSHFT`, `PRESRR`/`PRESFR`, and the `$` maintenance indicator.

Unknown groups SHALL be listed as "not decoded", never dropped silently.

#### Scenario: Full US METAR
- **WHEN** the input is `KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083`
- **THEN** the decoder reports wind 300° true at 15 kt gusting 25, visibility 10 SM, few at 8,000 ft and scattered at 20,000 ft AGL, temperature 30.0 °C, dew point 8.3 °C, altimeter 29.80 inHg, and sea-level pressure 1005.2 hPa

#### Scenario: Undecoded group listed
- **WHEN** a report contains a group the decoder does not recognize
- **THEN** the group is shown under "Not decoded" with its position

### Requirement: Wind reference is carried explicitly
Decoded METAR, TAF, and FB winds SHALL be labeled as true-north referenced. When sent to wind or runway tools, the hand-off SHALL carry `reference: true` so the heading chain applies variation correctly.

#### Scenario: Hand-off to runway components
- **WHEN** the user taps "Use this wind for runway components"
- **THEN** the runway tool opens with wind 300° true, 15 kt, gust 25, and asks for the runway heading reference (magnetic, with variation offered from the geomagnetism tool)

### Requirement: TAF decoding
The decoder SHALL parse TAF validity periods (including periods crossing midnight and month end), `FM`, `TEMPO`, `BECMG`, `PROB30`/`PROB40`, and the same weather groups. It SHALL present a timeline with local and UTC times.

#### Scenario: Validity across midnight
- **WHEN** a TAF valid `1818/1918` is decoded
- **THEN** the timeline spans 18:00Z on the 18th to 18:00Z on the 19th, with each change group placed correctly

### Requirement: FB winds and temperatures aloft
The FB decoder SHALL parse station lines and levels. It SHALL handle:
- `9900` (light and variable)
- speeds of 100–199 kt encoded by adding 50 to the direction (direction 51–86 → subtract 50, add 100 kt)
- implied negative temperatures above 24,000 ft
- no temperature at 3,000 ft
- no winds within 1,500 ft of station elevation

Decoded values SHALL feed the winds-aloft interpolation tool.

#### Scenario: High-speed encoding
- **WHEN** the group `731960` is decoded at FL340
- **THEN** the result is 230° true at 119 kt, temperature −60 °C

#### Scenario: Light and variable
- **WHEN** the group `9900+05` is decoded
- **THEN** the result is "light and variable (less than 5 kt)", temperature +5 °C

### Requirement: Decoder safety framing
Every decoder result SHALL display "Decoded from the text you pasted. Get a current official briefing before flight." It SHALL flag a report whose time is more than 2 hours older than a user-supplied current time as `OBSERVATION_OLD`. The tool SHALL never infer the current time itself.

#### Scenario: Old observation
- **WHEN** the user supplies a current time 3 hours after the METAR time
- **THEN** the result includes `OBSERVATION_OLD`
