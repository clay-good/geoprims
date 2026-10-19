## Purpose

Quotes the FAA's oxygen rules and time-of-useful-consciousness tables for an altitude, dated and with the FAA's own caution, so a pilot checks the rule rather than remembering it.

## ADDED Requirements

### Requirement: Oxygen requirement
Given a cabin pressure altitude, duration above thresholds, and the operating part (91 by default; 135 or 121 by selection), the tool SHALL state which paragraph applies and what it requires, from dated entries in `data/regulations.json`: for 14 CFR 91.211(a), above 12,500 ft to 14,000 ft MSL the crew uses oxygen for the part of the flight over 30 minutes, above 14,000 ft for the whole time, above 15,000 ft every occupant is provided oxygen; for 91.211(b) pressurized aircraft, the FL250 10-minute supply and the FL350 and FL410 mask rules. It SHALL show "rules as of <date>" and the eCFR link.

#### Scenario: 13,500 ft for 45 minutes
- **WHEN** cabin altitude is 13,500 ft for 45 minutes under Part 91
- **THEN** the result cites 91.211(a)(1): the crew must use oxygen for the 15 minutes beyond 30

#### Scenario: Part 135
- **WHEN** Part 135 is selected at 11,000 ft for 40 minutes
- **THEN** the result cites 135.89's 10,000–12,000 ft rule and the time over 30 minutes

### Requirement: Time of useful consciousness
The tool SHALL return, for an altitude, the AC 61-107B Change 1 Figure 2-3 range (with its rapid-decompression column) and the PHAK-25C Figure 17-1 range beside it, each named, without interpolation (an altitude between rows reports both bracketing rows), and SHALL always display the AC's caution that individual times vary.

#### Scenario: 25,000 ft
- **WHEN** 25,000 ft is entered
- **THEN** the result shows 3–5 minutes (AC 61-107B; 1.5–2.5 minutes after rapid decompression) and 3–5 minutes (PHAK), with the caution

#### Scenario: Between rows
- **WHEN** 23,000 ft is entered
- **THEN** the result shows the 22,000 ft and 25,000 ft rows of each table and states that no interpolation is made

### Requirement: Reference-tool framing
Both tools SHALL display "Dated reference, not medical or legal advice. The regulation and your aeromedical training govern."

#### Scenario: Notice present
- **WHEN** either tool shows a result
- **THEN** the notice is visible with it
