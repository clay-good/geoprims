<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to Swiss LV95 (`geodesy.projection.lv95-forward`)

## Method

The Swiss national grid is not a transverse Mercator. It is an oblique Mercator, built so that the cylinder touches the ellipsoid along a line through Bern rather than along a meridian, which is what keeps distortion small over a country that is wider east to west than north to south. swisstopo computes it in three steps, and this tool follows them exactly as their "Formulas and constants" document (December 2016) writes them.

First the Bessel ellipsoid is mapped conformally onto a sphere of radius 6,378,815.90365 m — a Gauss projection, with the latitude carried by swisstopo's `S` expression and its constant `K`, and the longitude stretched by α = 1.000 729 138 430 38. Then the sphere is rotated so that the fundamental point in Bern (46°57′08.66″N 7°26′22.50″E, the "old values", which swisstopo says remain valid for every geodetic purpose) lands on its equator. Finally a Mercator of that rotated sphere gives the grid, with LV95's false origin added: 2,600,000 m east and 1,200,000 m north.

LV03, the older grid, is the same projection with a false origin of 600,000 and 200,000, so the tool reports the LV03 coordinates beside the LV95 ones: exactly two million and one million less.

## Equations

With `E` the first eccentricity of Bessel, `φ₀`, `λ₀` the fundamental point, `R` the sphere's radius, `α` the longitude ratio, `b₀` the fundamental point's spherical latitude and `K` the latitude constant (all in §3.1 of the source):

- `S = α·ln(tan(π/4 + φ/2)) − (α·E/2)·ln((1 + E·sin φ)/(1 − E·sin φ)) + K`
- `b = 2·(arctan(e^S) − π/4)`, `l = α·(λ − λ₀)`
- `l̄ = arctan( sin l / (sin b₀·tan b + cos b₀·cos l) )`
- `b̄ = arcsin( cos b₀·sin b − sin b₀·cos b·cos l )`
- `E = R·l̄ + 2,600,000`, `N = (R/2)·ln((1 + sin b̄)/(1 − sin b̄)) + 1,200,000`

## Symbols and units

Latitude and longitude in degrees on CH1903 (Bessel 1841); easting and northing in meters from LV95's false origin, and in meters from LV03's beside them.

## Domain

Any latitude and longitude the formulas are finite for; they are meant for Switzerland and its surroundings, where the rotation that makes them conformal is centred. A point that projects to infinity is refused with `OUT_OF_DOMAIN`.

## Approximations

None in the projection: these are swisstopo's rigorous formulas, not the approximate ones their document also prints and warns against for cadastral work. The approximation to state is the datum: the projection is defined for CH1903, and a satellite position on WGS 84 or ETRS89 is a datum step away, which swisstopo publishes a grid for and which is a separate tool. Heights are untouched.

## Worked example

- sourcePublisher: Federal Office of Topography swisstopo
- sourceTitle: Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems
- sourceEdition: December 2016 edition
- sourceLocator: Section 3.2, the numerical example for the station Rigi: φ = 47°03′28.95659233″, λ = 8°29′11.11127154″ on Bessel gives E = 2,679,520.05, N = 1,212,273.44, with every intermediate printed
- independent: yes
- inputs: lat 47°03′28.95659233″N, lon 8°29′11.11127154″E
- outputs: E 2,679,520.05 m, N 1,212,273.44 m
- tolerance: 0.005 m, half the centimetre they print
- verifiedBy: golden vector v001 and `core/crates/gp-geo/src/proj.rs` `swisstopo_rigi_example`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_lv95.py` builds the 12 vectors in `core/vectors/geodesy.projection.lv95-forward.jsonl` from PROJ 9.9.0 through `cs2cs` applying EPSG:2056, so the second implementation is PROJ's oblique Mercator with the EPSG dataset's parameters rather than swisstopo's own formulas: Rigi, the fundamental point in Bern, and ten places from Geneva to Davos and from Basel to Lugano, all within a millimetre.

## Invariants

- `core/crates/gp-geo/src/proj.rs` `swisstopo_rigi_example`: swisstopo's worked example reproduces in both directions, with the spherical latitude, the spherical longitude, and both rotated coordinates matching the intermediates they print; and the projection and its inverse return four places across the country to within a nanodegree.
