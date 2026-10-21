<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Tidal datum conversion (`navigation.tide.datums`)

## Method

A NOAA datum sheet lists each tidal datum's height above the station datum (STND) for one station and one datum epoch. A height above one datum becomes a height above another by adding the first datum's height and subtracting the second's. The tool reads the sheet the reader pastes, either the station page's table (which needs its units chosen) or the metadata API's `datums.json`, and leaves out the rows that are ranges or intervals rather than heights (GT, MN, DHQ, DLQ, HWI, LWI). LAT and HAT, which `datums.json` gives beside its list, are read too.

## Equations

- Height above the to datum = height above the from datum + (from datum − to datum), each datum's height above STND from the sheet.

## Symbols and units

Heights in meters by default (any length unit is accepted); sheet values in meters or feet as the sheet states, converted to meters.

## Domain

Both datums must be on the sheet, else INVALID_INPUT naming the datums it has. A pasted table needs its units. A sheet with fewer than two datums is refused.

## Approximations

None in the arithmetic. The sheet is for one station and one epoch (Boston's, for example, is 1983 to 2001); heights between stations or across epochs need NOAA's VDatum.

## Worked example

- sourcePublisher: NOAA Center for Operational Oceanographic Products and Services
- sourceTitle: Datums for Boston, MA (8443970), metadata API datums.json
- sourceEdition: epoch 1983-2001, accepted 17 April 2003, fetched 2026-10-20
- sourceLocator: MLLW 1.074 m and NAVD88 2.752 m above STND
- independent: yes
- inputs: 3.000 m above MLLW, to NAVD88
- outputs: 1.322 m above NAVD88
- tolerance: 1e-9 m
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/tide.rs` `datum_conversion`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_tide.py datums`: 20 conversions between the datums of NOAA's Boston sheet in meters, by subtraction, and two refusals (LAT, which the sheet lacks, and GT, a range); 22 vectors in `core/vectors/navigation.tide.datums.jsonl`.

## Invariants

- `core/crates/gp-navigation/tests/tide.rs` `datum_conversion`: the Boston scenario to 1.322 m; a missing datum is refused naming the sheet's datums; a range row is not a datum; a pasted table in feet converts and round-trips; a table without units is refused rather than guessed.
