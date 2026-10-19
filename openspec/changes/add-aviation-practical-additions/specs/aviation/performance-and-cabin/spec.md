## Purpose

Answers four performance questions from the airplane's own numbers: maneuvering speed at today's weight, the cabin altitude a differential holds, the hydroplaning speed for the tires, and whether a departure climb gradient can be met.

## ADDED Requirements

### Requirement: Maneuvering speed by weight
The tool SHALL compute V_A at a weight as V_A,max × √(W / W_max), labeled as derived from 14 CFR 23.335(c) and V_S ∝ √W, with the AFM named as governing. When V_S is given it SHALL show the PHAK cross-check V_A ≈ 1.7 V_S. Speeds SHALL be reported to 1 kt with full precision in the machine value.

#### Scenario: Lighter weight
- **WHEN** V_A is 105 KIAS at 2,550 lb and the weight is 2,000 lb
- **THEN** V_A is 93 KIAS (92.99 exact, ±0.01 kt)

#### Scenario: Weight above maximum
- **WHEN** the weight exceeds W_max
- **THEN** the result is `OUT_OF_DOMAIN` stating the formula applies at or below maximum weight

### Requirement: Cabin altitude from differential
Given pressure altitude and a maximum differential (psi or hPa), the tool SHALL compute cabin pressure = ISA pressure at altitude + differential and the ISA altitude of that pressure, and, given a selected cabin altitude, SHALL report the differential needed and whether it is within the maximum. ISA SHALL come from the existing atmosphere engine.

#### Scenario: PHAK example
- **WHEN** the aircraft is at 28,000 ft with a 6.1 psi differential
- **THEN** cabin pressure is 10.9 psi (±0.05 psi) and the cabin altitude 8,000 ft (±50 ft), as PHAK Figure 7-41 shows

#### Scenario: Differential not needed
- **WHEN** a cabin altitude of 8,000 ft is selected at 12,000 ft pressure altitude with an 8.0 psi maximum
- **THEN** the differential in use is 1.6 psi (±0.05 psi) and the cabin holds 8,000 ft

### Requirement: Hydroplaning speed
The tool SHALL compute dynamic hydroplaning speed V_p = 9√p kt (p in psi; NASA TN D-2056 eq. 4) by default, offer the AFH 8.6√p variant by name, state the report's validity (24–150 psi; water deeper than the tread grooves; smooth or rib tires), and return `OUT_OF_DOMAIN` outside the pressure range. It SHALL show the rotating-tire note that spin-down begins near 70% of V_p.

#### Scenario: PHAK example
- **WHEN** tire pressure is 36 psi
- **THEN** V_p is 54 kt exactly, with the AFH variant 51.6 kt shown

#### Scenario: Pressure out of range
- **WHEN** tire pressure is 200 psi
- **THEN** the result is `OUT_OF_DOMAIN` naming 24–150 psi

### Requirement: Climb requirement versus available
Given a required gradient (ft/NM, or an altitude to reach over a distance), TAS or IAS with altitude and temperature, the wind component, and the aircraft's rate of climb, the tool SHALL compute groundspeed, the rate of climb required (gradient × GS / 60), the gradient available (rate × 60 / GS), and the margin, stating AIM 5-2-9's 200 ft/NM default and the 35 ft DER crossing. A negative margin SHALL carry `CLIMB_GRADIENT_SHORT` (caution).

#### Scenario: IPH example
- **WHEN** 297 ft/NM is required at 180 kt groundspeed
- **THEN** the required rate of climb is 891 fpm (±1 fpm)

#### Scenario: Gradient from altitude and distance
- **WHEN** 3,000 ft must be reached by 5 NM from a 1,221 ft airport
- **THEN** the required gradient is 356 ft/NM (355.8 exact)

#### Scenario: Tailwind shortfall
- **WHEN** the required rate exceeds the available rate because of a tailwind
- **THEN** the result carries `CLIMB_GRADIENT_SHORT` and states the shortfall in fpm
