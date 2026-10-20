<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Parallel sailing (`navigation.sailing.parallel`)

## Method

On a parallel of latitude L the departure p and the difference of longitude DLo are interconverted by the cosine of the latitude, on Bowditch's sphere of one nautical mile per minute of arc (§1304).

## Equations

- p = DLo cos L (DLo in minutes).
- DLo = p sec L.

## Symbols and units

L latitude (degrees, DMS with a hemisphere letter accepted); DLo degrees (reported also in minutes); p nautical miles.

## Domain

Any latitude short of a pole when converting departure to DLo (at a pole every meridian meets, refused with `OUT_OF_DOMAIN`).

## Approximations

Bowditch's sphere. On WGS 84 a minute of longitude is up to about 0.5% longer than this; geodesic and rhumb tools give the ellipsoidal value.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1304: DLo 3°30' on latitude 49°30' N
- independent: yes
- inputs: L = 49°30' N, DLo = 3°30'
- outputs: p = 136.4 NM E
- tolerance: 0.05 NM
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py parallel`: Python at 19 random latitudes in both directions, exact to 1e-9 NM and 1e-11°, and the polar refusal.
- `core/vectors/navigation.sailing.parallel.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `bowditch_chapter_13_examples`: the §1304 example and its inverse (to 1e-4°, as Bowditch rounds cos L to 0.64945).
