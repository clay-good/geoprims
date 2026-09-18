## Purpose

Computes turn, climb, descent, and glide performance from first principles and user-supplied aircraft data, with the rules of thumb shown and labeled beside the exact values.

## ADDED Requirements

### Requirement: Turn performance
The turn tool SHALL relate true airspeed, bank angle, turn rate, turn radius, load factor, and time for a given heading change, solving for any one from sufficient inputs. It SHALL use r = V²/(g0·tan φ), ω = g0·tan φ / V, n = 1/cos φ, and the standard-rate definition 3°/s. It SHALL also give the stall speed in the turn (Vs·√n) from a user-entered 1g stall speed. The constant used in feet-and-knots forms (r = V²/(11.294·tan φ)) SHALL be derived from g0 and documented, with the common textbook constant 11.26 noted.

#### Scenario: Standard-rate bank at 100 kt
- **WHEN** a standard-rate turn is requested at 100 KTAS
- **THEN** bank ≈ 15.36° and the rule of thumb (KTAS/10 + 7 = 17°) is shown as an approximation

#### Scenario: 60° bank
- **WHEN** bank = 60° with 1g stall speed 50 KCAS
- **THEN** load factor = 2.0 and stall speed in the turn ≈ 70.7 KCAS

#### Scenario: Load factor limit
- **WHEN** a user-entered limit load factor of 3.8 is exceeded by the requested bank
- **THEN** the result flags `LOAD_LIMIT_EXCEEDED` with the maximum bank for the limit

### Requirement: Climb and descent
Tools SHALL convert between climb/descent gradient (ft/NM, %, degrees) and vertical speed (fpm) at a groundspeed. They SHALL also compute top-of-descent distance for an altitude change and a descent angle or vertical speed, showing the 3:1 rule and the ≈ 5 × GS fpm rule beside the exact values.

#### Scenario: Descent from FL350
- **WHEN** descending from 35,000 ft to 3,000 ft on a 3° path at 420 kt groundspeed
- **THEN** exact distance ≈ 100.5 NM (3:1 rule: 96 NM) and vertical speed ≈ 2,229 fpm (5 × GS rule: 2,100 fpm)

#### Scenario: Climb gradient to fpm
- **WHEN** a departure requires 200 ft/NM at 120 kt groundspeed
- **THEN** the required climb rate is 400 fpm

### Requirement: Glide range
Given height above terrain, glide ratio (L/D) or best-glide data, TAS, and wind, the glide tool SHALL compute still-air and wind-corrected glide distance. It SHALL draw the glide range as a wind-shifted range ring on the map.

#### Scenario: Glide into headwind
- **WHEN** height = 5,000 ft AGL, L/D = 9, TAS = 70 kt, headwind = 20 kt
- **THEN** still-air range ≈ 7.41 NM and headwind range ≈ 5.29 NM

### Requirement: Pivotal altitude and ground-reference geometry
A tool SHALL compute pivotal altitude from groundspeed (h = V²/g0), showing the kt²/11.3 rule.

#### Scenario: Pivotal altitude at 100 kt
- **WHEN** groundspeed = 100 kt
- **THEN** pivotal altitude ≈ 885 ft AGL

### Requirement: Density-altitude effects need aircraft data
Takeoff and landing distance tools SHALL operate only on user-supplied POH/AFM tables (via fuel-and-loading table interpolation) and SHALL NOT provide generic aircraft performance estimates (the generic Koch-chart estimate is excluded because it would be mistaken for aircraft data).

#### Scenario: No POH table
- **WHEN** a user requests takeoff distance without a POH table
- **THEN** the tool explains that aircraft data from the POH/AFM is required, offers the table-interpolation tool, and links the density-altitude explainer
