<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Area by coordinates (`survey.cogo.area-by-coordinates`)

## Method

The coordinate (shoelace) method: twice the area is the sum of the cross products of successive corners around the parcel. Its sign gives the direction of travel. Acres and hectares follow from the length unit. US survey feet give US survey acres, and mixing US survey and international feet is refused. The perimeter is the sum of the sides.

## Equations

- 2A = Σ (Nᵢ Eᵢ₊₁ − Nᵢ₊₁ Eᵢ), with the last corner joined to the first.
- Area = |A|. Positive A in (easting, northing) axes is counterclockwise.
- Acres = ft² / 43,560 (US survey acres from US survey square feet), hectares = m² / 10,000.

## Symbols and units

N northing and E easting of each corner, in one length unit (ft, ftUS, or m). Area in that unit squared, plus acres and hectares.

## Domain

Three or more corners, in order around the parcel, not self-intersecting. A crossing parcel's signed area is not its land area, and the tool assumes the corners are in order.

## Approximations

None on the plane. The area is on the coordinate grid. For ground area, divide by the square of the combined factor, as NGS Manual 5 describes.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Shoelace formula (worked example)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: The pentagon (1, 6), (3, 1), (7, 2), (4, 4), (8, 5): the five determinants sum to 33, so the area is 16.5
- independent: yes
- inputs: easting, northing = (1, 6), (3, 1), (7, 2), (4, 4), (8, 5)
- outputs: area 16.5 square units
- tolerance: 1e-12 relative
- verifiedBy: golden vector v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python implementation at 18 parcels, from a 1 ft square to about 430 acres, with large state-plane coordinates, concave shapes, and both orientations (within 1e-11 relative)
- `core/vectors/survey.cogo.area-by-coordinates.jsonl`: those vectors, the US survey acre case, and the published example, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `area_by_coordinates_invariants`: area is unchanged by shifting the parcel far away, rotating it, or starting at another corner; reversing the order flips only the orientation
