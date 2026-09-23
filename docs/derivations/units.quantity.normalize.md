<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Quantity normalizer (`units.quantity.normalize`)

## Method

This is the converter you reach for when you do not know which converter you need. Give it a value with a unit written the way a document writes it — `145 kts`, `29.92 inHg`, `5280'`, `15 C` — and the quantity it measures, and it returns the same value in that quantity's canonical unit: SI throughout, with degrees for angles because radians are not how a bearing is written.

Two steps do the work. The unit is parsed: symbols, aliases and the shorthands people actually type, including the prime and double-prime for feet and inches. Then the value is converted by exactly the same registry the individual converters use — one multiplication by a ratio of two exact definitions, rounded once — so a length normalized here and the same length through the length converter are the same bits, not two answers that happen to agree.

Two things are refused rather than guessed. A unit that does not belong to the stated quantity is an error, not a reinterpretation: `1600 mil` asked for as an angle fails, because "mil" is three different angles and a thousandth of an inch. And a bare number with no unit is refused outright rather than read with a default, since the whole job here is to be told what the value is. What is not refused is an ambiguous spelling the context settles: `12 nm` asked for as a distance is a nautical mile, and the result carries a `UNIT_ASSUMED` warning saying so, because to anyone outside navigation it is a nanometre.

## Equations

- y = x · (source unit in canonical units) / 1, rounded once — the canonical unit's factor is 1 by construction.
- Canonical units: m, m², m³, kg, m/s, m/s², Pa, K, deg, deg/s, s, J, W, C, V, kg/m³, Hz, bit/s.

## Symbols and units

`value` is the text as written and `quantity` names what it measures — one of length, distance, area, volume, mass, speed, vertical-speed, acceleration, pressure, temperature, temperature-difference, angle, angular-rate, time, energy, power, electric-charge, electric-potential, density, frequency, data-rate or slope. `normalized` comes back in the canonical unit, and `input` echoes the value as it was read, so a misparse is visible rather than silent.

## Domain

Any finite value in any unit the registry knows for the stated quantity. Temperature is the one quantity with a floor: below absolute zero is refused, as it is in the temperature converter.

## Approximations

None beyond the single rounding, for every quantity whose units are exactly defined — which is all of them except the angle units built on π, where the ratio is irrational and carries the same looser bound the angle converter does.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: the exact definitions of the knot, the foot, the pound, the acre, the gallon, standard gravity, the inch of mercury and the rest, as cited by each quantity's own note
- independent: yes
- inputs: 24 cases, at least one from each of eighteen quantities, including 145 kts to m/s, 2,550 lb to kg, 40 ac to m², 180 hp to W, 5280′ to m, and a mil asked for as an angle, which is refused
- outputs: the canonical value and unit in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py` and `gen_units_special.py`, which restate each unit from its published definition and convert in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. Because the canonical unit's own factor is 1, each of these is a single ratio, which is the cheapest case the registry has.

## Differential tests

- `tools/vectors/gen_units.py` and `gen_units_special.py`: all 24 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.quantity.normalize.jsonl`: 24 vectors, covering eighteen of the twenty-two quantities it accepts

## Invariants

- `core/crates/gp-units/tests/units.rs` `normalize_invariants`: for every quantity it accepts, a value already in the canonical unit comes back untouched and still in that unit, which is what pins the canonical choice itself; normalizing agrees bit for bit with the matching converter asked for the same target, so the two surfaces cannot drift apart; a bare number is refused rather than read with a default, while an ambiguous spelling the quantity settles -- `12 nm` as a distance -- is accepted with a `UNIT_ASSUMED` warning and an unambiguous one is not; a unit from the wrong quantity is refused with `UNIT_MISMATCH`; and the angle canon is degrees rather than radians, which a normalizer written straight from SI would get wrong
