<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Auxiliary latitudes (`geodesy.ellipsoid.auxiliary-latitude`)

## Method

Six latitudes stand beside the geodetic one, each the angle that makes some property of the ellipsoid behave like a sphere. The input is read as whichever of the seven it is said to be, converted to geodetic, and then all seven are reported. Going to geodetic is a closed form for the geocentric, parametric, and isometric latitudes; the conformal inverse runs Newton's method on Karney's τ′ substitution, and the rectifying and authalic inverses run Newton's method on their own defining relations. Going out from geodetic is closed form throughout, with the authalic latitude taking the difference qp − q in a cancellation-free form rather than subtracting two nearly equal numbers.

## Equations

With e the first eccentricity and f the flattening:

- Geocentric: tan θ = (1 − e²) tan φ.
- Parametric (reduced): tan β = (1 − f) tan φ = √(1 − e²) tan φ.
- Isometric: ψ = asinh(tan φ) − e·atanh(e sin φ).
- Conformal: tan χ = sinh ψ, so χ is the Gudermannian of the isometric latitude.
- Rectifying: μ = 90° · M(φ) / M(90°), with M the meridian arc from the equator.
- Authalic: sin ξ = q(φ) / q(90°), with q(φ) = (1 − e²)·[ sin φ / (1 − e² sin²φ) + atanh(e sin φ)/e ].

## Symbols and units

φ is the geodetic latitude and θ, β, μ, χ, ξ, ψ the geocentric, parametric, rectifying, conformal, authalic, and isometric latitudes, all in degrees. q is twice the area of the zone from the equator to φ divided by a², dimensionless. The ellipsoid is WGS 84 unless another catalog ellipsoid or a custom a and 1/f is given.

## Domain

Latitudes from −90° to 90° for six of the seven, and the poles are included: every latitude but the isometric reaches ±90° there. The isometric latitude has no bound, running to infinity at the poles, so as an input it is accepted out to ±10,000°, beyond which double precision holds nothing but the pole itself. A custom ellipsoid is accepted as long as its geodesic is well posed; the Newton iteration for the inverse conformal latitude is only settled for flattenings near the Earth's, and a wilder one is refused rather than answered badly.

## Approximations

None in the forward direction: all six are closed forms evaluated directly, and the meridian arc behind the rectifying latitude is the exact elliptic integral rather than a series. The inverses that need Newton's method are iterated to double-precision convergence. The authalic latitude's q is written to avoid subtracting two nearly equal numbers near the pole, which is where a naive qp − q loses its significant digits.

## Worked example

- sourcePublisher: Charles F. F. Karney and the GeographicLib contributors
- sourceTitle: GeographicLib command-line tools
- sourceEdition: GeographicLib 2.7
- sourceLocator: `CartConvert` on 45 0 0; `GeodSolve -i -p 6` on 0 0 → 45 0 and 0 0 → 90 0; `RhumbSolve -i -p 6` on 0 0 → 45 50.22746581671611; `ConicProj -a 0 0 -p 9` on 45 0 and 90 0
- independent: yes
- inputs: latitude 45, geodetic, WGS 84
- outputs: geocentric 44.80757678401804, parametric 44.903787849420226, rectifying 44.85568198890798, conformal 44.80768405608882, authalic 44.87170287343396, isometric 50.22746581671611
- tolerance: 1e-11°, which every one of the six meets with room to spare
- verifiedBy: golden vector v031, run by the core on every build
- verifiedOn: 2026-09-22

Each latitude is taken from the shipped tool that already depends on it, so nothing here re-implements a formula the core also has. `CartConvert` turns 45° into X = 4517590.878848932 and Z = 4487348.408865920: the geocentric latitude is atan2(Z, X) and the parametric is atan2(Z/b, X/a). `GeodSolve` measures the meridian from the equator as 4984944.377978 m to 45° and 10001965.729313 m to the pole, whose ratio times 90° is the rectifying latitude. `RhumbSolve` settles the isometric latitude by its defining property rather than by arithmetic: a rhumb line from 0, 0 to 45, 50.22746581671611 comes back with azimuth 45.00000000000, which is only true when the longitude difference equals the isometric latitude in degrees. The conformal latitude follows from it as the Gudermannian. `ConicProj -a 0 0`, the equal-area cylindrical case, gives northings 4489858.886948003 at 45° and 6363885.331926039 at the pole, whose ratio is sin of the authalic latitude. Geocentric, parametric, conformal, authalic, and isometric agree within 3e-14°; the rectifying latitude agrees within 1.1e-12°, which is where GeodSolve's six printed decimals of meters run out, not where either answer does.

## Differential tests

- `tools/vectors/gen_frames.py`: 29 vectors from a 40-digit mpmath evaluation of the defining relations, across latitudes and both directions
- `core/crates/gp-geodesy/tests/frames.rs` `auxiliary_latitude_invariants`: the ordering, the round trips, and the two latitudes that can be had from other tools in the catalog
- `core/vectors/geodesy.ellipsoid.auxiliary-latitude.jsonl`: 31 vectors, 29 from mpmath, one from the input rules, and one from the GeographicLib tools above

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `auxiliary_latitude_invariants`: every latitude is odd in φ and fixes 0°, and every one but the isometric fixes ±90°; the six are ordered θ ≤ χ ≤ μ ≤ ξ ≤ β ≤ φ at every whole degree of the northern hemisphere, which is the ellipsoid's own ordering and not an accident of one latitude; each converts back to the geodetic latitude it came from within 1e-12°; the geocentric latitude is the one `geodesy.frame.geodetic-to-ecef` implies from its own X and Z, and the parametric latitude likewise once X and Z are divided by a and b; and the conformal latitude is the Gudermannian of the isometric, tying two of the outputs to each other rather than each to the same source
