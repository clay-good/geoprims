<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to a British National Grid reference (`geodesy.grid-ref.bng-forward`)

## Method

A National Grid reference is three things in sequence: a datum, a projection, and a lettering convention.

The **datum** comes first because the National Grid is drawn on OSGB36, and a phone gives WGS 84. Ordnance Survey publishes a seven-parameter transformation between them, registered as EPSG operation 1314, which this tool applies when the reader says their coordinates are WGS 84 and skips when they say OSGB36. It is a similarity transformation, so it cannot model the distortion of the retriangulation, and Ordnance Survey states it as about 2 m; that is the reason for the `LOW_ACCURACY_TRANSFORM` warning, and the reason the OSTN15 grid exists.

The **projection** is the National Grid's transverse Mercator, whose parameters the OS guide gives in Annexe A: the Airy 1830 ellipsoid, a true origin at 49°N 2°W, a central-meridian scale factor of 0.999 601 271 7, and a false origin 400 km west and 100 km north of it so that everything in Britain is positive.

The **lettering** divides the grid into 100 km squares, each with a two-letter code, and the letter I is not used, which is why the alphabet runs A to Z with 25 letters. The figures that follow are an easting and a northing inside that square, truncated rather than rounded: a reference names the square a point falls in, not the nearest square's center. Two figures name a 10 km square, four a 1 km square, six 100 m, eight 10 m, ten a metre.

## Equations

- Easting and northing: the transverse Mercator of the OS guide's Annexe C, with the parameters above.
- The 100 km square's letters, from the column `ce = ⌊E/100000⌋` and row `cn = ⌊N/100000⌋`: first letter index `(19 − cn) − (19 − cn) mod 5 + ⌊(ce + 10)/5⌋`, second `((19 − cn)·5) mod 25 + ce mod 5`, into the alphabet without I.
- The figures, for `f` of them: `⌊(E mod 100000)/(100000/10^(f/2))⌋`, zero-padded, and the same for the northing.

## Symbols and units

Latitude and longitude in degrees; eastings and northings in meters from the National Grid's false origin; the square's side in meters. The square's edges are reported as latitudes and longitudes on the datum asked for, so a map can draw it.

## Domain

- The lettered grid runs 0 to 700 km east and 0 to 1,300 km north of the false origin; outside it there is no reference to give and the result is `OUT_OF_DOMAIN`.
- Figures must be even, 2 to 10.
- The seven-parameter path has an area of use (Great Britain and the Isle of Man); outside it the transformation is extrapolation.

## Approximations

The seven-parameter transformation is about 2 m, so a ten-figure reference is more precise than it is accurate: the figures are exact for the easting and northing computed, and those depend on a datum step that is metres. Centimetre work needs OSTN15, which is a grid rather than seven numbers and is not yet carried here. The projection itself is the series Ordnance Survey publishes and is good to well under a millimetre over Britain. Heights are untouched: the National Grid is horizontal.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: A Guide to Coordinate Systems in Great Britain
- sourceEdition: Version 3.6 (D00659), © Crown copyright 2020
- sourceLocator: Annexe C, the worked example of the transverse Mercator formulae: 52°39′27.2531″N 1°43′04.5177″E on OSGB36 gives E 651409.903 N 313177.270
- independent: yes
- inputs: lat 52.657570305, lon 1.717921583, datum OSGB36, 10 figures
- outputs: E 651409.903 m, N 313177.270 m, reference TG 51409 13177
- tolerance: 0.001 m, the last digit the guide prints
- verifiedBy: golden vector v001 and `core/crates/gp-geo/src/gridref.rs` `the_conventions_own_example`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_bng.py` builds the 22 vectors in `core/vectors/geodesy.grid-ref.bng-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, with the National Grid written out as the OS guide gives it and EPSG 1314 as `+towgs84`, and with the lettering written again in Python from the guide's section 7.1. PROJ reproduces the guide's own Annexe C example to a tenth of a millimetre. The cases are ten places across Great Britain, each on WGS 84 and on OSGB36, at precisions from two to ten figures, and the refusal south of the lettered grid.
- The same parameters are read from the datum table rather than copied: a typed copy of EPSG 1314's rotations, rounded a decimal further, put this tool 3 mm from the datum tool before `crate::datum::legacy` existed.

## Invariants

- `core/crates/gp-geo/src/gridref.rs` `the_conventions_own_example`: `TQ 30 80` decodes to the 1 km square at E 530000 N 180000; E 530624.974 N 178388.464 encodes to `TQ 30624 78388`; the letter I is refused by name; and every one of the grid's 100 km squares round trips through its own two-letter code.
