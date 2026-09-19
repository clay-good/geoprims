<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Time, speed, and distance (`navigation.route.time-speed-distance`)

## Method

The navigator's first relation: distance is speed times time. Give any two and the third comes back. Every quantity is converted to its canonical unit (meters, meters per second, seconds) before the arithmetic and back for display, so a leg given in knots and minutes and one given in miles per hour and hours are the same leg. With a departure time and a UTC offset, the elapsed time is added to the clock to give the arrival, local and Zulu, rounded to the minute and marked when it crosses midnight.

## Equations

- d = v × t, v = d / t, t = d / v.
- ETE: the elapsed time written in hours and minutes.
- ETA = departure + ETE, wrapped into the next day when it passes 24:00; ETA (Zulu) = ETA − UTC offset.

## Symbols and units

d distance, v speed, t time, in any length, speed, and time units the catalog knows; canonically meters, meters per second, and seconds. Departure is a local clock time, and the UTC offset is in hours.

## Domain

Exactly two of distance, speed, and time (a third is refused, and so is a single one). None may be negative. A zero speed or a zero time is refused when it is the divisor (solving for time at no speed, or for speed in no time); a zero speed given with a time is simply no distance covered. The clock outputs need both a departure and a UTC offset.

## Approximations

None: multiplication and division in double precision, exact to the last place for any input the unit converters accept. Only the printed arrival time is rounded, to the minute.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Bowditch, The American Practical Navigator (NGA Pub. 9), Volume II, Table 11, Speed, Time, and Distance
- sourceEdition: 2024 edition
- sourceLocator: The table's cells, for example 60 minutes at 12.0 knots is 12.0 miles, 45 minutes at 8.5 knots is 6.4 miles, and 7 minutes at 40.0 knots is 4.7 miles
- independent: yes
- inputs: seven cells spread across the table, each as its speed in knots and its time in minutes
- outputs: the distance each cell prints, in nautical miles
- tolerance: 0.05 nm, the tenth of a mile the table is printed to
- verifiedBy: golden vectors v023 to v029, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/tsd_parity.rs` `distance_matches_bowditch_table_11`: all 4,800 printed cells of Table 11 (1 to 60 minutes against 0.5 to 40.0 knots) through the public tool, every one within the tenth of a mile it is printed to. One cell is pinned as a misprint instead: the table gives 6.8 nm for 38 minutes at 10.5 knots, where the arithmetic is 6.65 nm and every neighbor in that row follows speed × time, so the core does not repeat it
- `tools/vectors/gen_tsd_diff.py`: regenerates that fixture from the published PDF
- `core/vectors/navigation.route.time-speed-distance.jsonl`: 29 vectors, 22 from an independent evaluation of the relation in Python and 7 from the table

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `time_speed_distance_invariants`: whichever value is left out returns consistent with the other two, the same leg in knots-and-minutes and miles-per-hour-and-hours gives the same distance, and the arrival clock adds the elapsed time (including across midnight and into Zulu)
- `core/crates/gp-navigation/tests/tsd_parity.rs` `every_unknown_is_solved_the_same_way`: across the table's own cells, solving for distance, speed, or time agrees with speed × time
