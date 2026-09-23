<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# True and magnetic bearings (`geodesy.magnetic.true-to-magnetic`)

## Method

One subtraction, and the sign is the whole problem. Magnetic variation — declination, to a geodesist — is the angle from true north to magnetic north, positive east. A magnetic bearing is the true bearing less that variation:

**magnetic = true − variation**, with east positive.

Which is the rule pilots carry as *east is least, west is best*: with easterly variation the magnetic bearing is the smaller number, with westerly the larger. Getting the sign backwards is a double error — twice the variation, which near the agonic line is nothing and in Iceland, where the variation is about eleven degrees west, is twenty-two.

The variation comes from one of two places, and the tool is explicit about which it used. Give a chart variation like `9.2W` and it uses that, because a chart is the legal source for a published procedure. Give a place and date and it uses the magnetic model. Give both and it does the sum with the chart figure, and reports the model's answer beside it with the difference — which is how you find out that a chart printed in 2015 is now three degrees out.

Bearings come back wrapped to [0, 360).

## Equations

- magnetic = (true − variation) mod 360.
- true = (magnetic + variation) mod 360.
- A chart variation written `12W` is −12°, `12E` is +12°.
- difference = chart variation − model declination, signed, so positive means the chart reads further east than the model.

## Symbols and units

`bearing` in degrees; `direction` is `true-to-magnetic` or `magnetic-to-true`; `variation` is a chart figure, or `lat`, `lon` and `date` for the model. Out come `result`, the `variation_used` and where it came from, and — when a chart figure was given — `model_declination`, `model_result` and `difference`.

## Domain

Any bearing, any place and date the model covers. A chart variation and a model can both be given; neither may be omitted.

## Approximations

The arithmetic is exact. The variation is not: the World Magnetic Model carries about half a degree of uncertainty and changes measurably from year to year, and a chart's printed figure is a rounded value that was right on its date of issue. The difference between the two is reported rather than reconciled.

## Worked example

- sourcePublisher: pygeomag contributors; NOAA/NCEI and the British Geological Survey (the model itself)
- sourceTitle: pygeomag, an independent Python implementation of the World Magnetic Model
- sourceEdition: WMM2025
- sourceLocator: magnetic = true − variation, east positive; the WMM spherical harmonic expansion for the declination
- independent: yes
- inputs: 26 bearings, in both directions, at nine places from Iceland to Singapore — westerly variation at most of them and easterly at Sydney — including bearings either side of north and four cases where a chart figure is set against the model
- outputs: the converted bearing, the variation used and its source, and the model comparison
- tolerance: 1e-4° against the model, exact on the chart arithmetic
- verifiedBy: golden vectors v001 to v026, run by the core on every build
- verifiedOn: 2026-09-23

The declination comes from `pygeomag`, a separate implementation of the same WMM: at Pittsburgh on 2026-07-02 it gives −9.2389527 and the core −9.2389552, two evaluations of the same spherical harmonic expansion agreeing to three millionths of a degree. The subtraction is then done in the generator from the rule, not read back from the core.

Writing the vectors turned up two things about the output that are easy to assume wrongly and are worth stating: `model_declination` appears only when a chart figure was also given, since without one the model's own value *is* the `variation_used`; and `difference` is signed as chart minus model, not the other way and not an absolute gap.

## Differential tests

- `tools/vectors/gen_magnetic_more.py`: 20 of the 26 vectors, declination from pygeomag
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.magnetic.true-to-magnetic.jsonl`: 26 bearings across both directions and both sources

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `true_to_magnetic_invariants`: the two directions undo each other, so a bearing converted and converted back is the bearing it started as; east is least — with an easterly variation the magnetic bearing is the smaller of the two, and with a westerly one the larger, which is the sign stated as a property rather than a formula; a chart figure and the model give different answers whose gap is the reported `difference`, signed chart minus model; every result is inside [0, 360), including one derived from a bearing a degree either side of north; and the source is named, `chart` when a figure is given and `model` when it is not
