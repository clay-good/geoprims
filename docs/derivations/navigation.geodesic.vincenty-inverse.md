<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vincenty inverse (`navigation.geodesic.vincenty-inverse`)

## Method

Distance and azimuths between two points on the ellipsoid by **Vincenty's 1975 method** — not by Karney's, which is what `navigation.geodesic.inverse` uses and is better.

That is the whole point. Vincenty was the standard for forty years; it is what most surveying software, most GIS libraries and most published figures still use. When you are checking one of those, or reproducing a number from a report, you need Vincenty's answer.

The method reduces both latitudes to the auxiliary sphere, then iterates on λ, the longitude difference on that sphere, until it settles to 1e-12 radians. The distance follows from a series in u² = cos²α (a² − b²)/b², with the A and B coefficients the paper gives, and a σ correction term.

Its known failure is nearly antipodal points: the λ iteration does not converge there, and the tool says so rather than returning a plausible number from a truncated loop. Karney's method converges everywhere, which is why it is the default and this one is the comparison.

The tool reports `karney_difference` alongside — how far Vincenty's distance is from the exact one — which is usually sub-millimetre and is the quantity that tells you whether a discrepancy in someone else's output is the algorithm or a mistake.

## Equations

- Reduced latitude: tan U = (1 − f) tan φ.
- Iterate λ = L + (1 − C) f sin α (σ + C sin σ (cos 2σₘ + C cos σ (−1 + 2 cos²2σₘ))) to 1e-12 rad.
- u² = cos²α (a² − b²)/b²; A and B are the paper's series in u².
- s = b A (σ − Δσ), with Δσ the correction term.
- α₁ = atan2(cos U₂ sin λ, cos U₁ sin U₂ − sin U₁ cos U₂ cos λ).

## Symbols and units

`lat1`, `lon1`, `lat2`, `lon2`, with an optional `ellipsoid` or explicit `a` and `inverse_flattening`. Out come the `distance`, `azimuth1`, `azimuth2` and `karney_difference`.

## Domain

Any two points except nearly antipodal ones, where the iteration does not converge. That is a property of the 1975 method, not a limitation of this implementation, and it is reported rather than worked around.

## Approximations

Vincenty's series is truncated, so it carries about a millimetre against the exact answer on a long line — which is why `karney_difference` is reported and why the exact tool exists.

## Worked example

- sourcePublisher: T. Vincenty, Directorate of Overseas Surveys
- sourceTitle: Direct and Inverse Solutions of Geodesics on the Ellipsoid with Application of Nested Equations
- sourceEdition: Survey Review XXIII, no. 176, April 1975
- sourceLocator: the inverse method's nested equations, iterated on lambda
- independent: yes
- inputs: 21 pairs, including a quarter of the equator, a meridian, transatlantic and transpacific routes, a 7 km line, and pairs in both hemispheres
- outputs: the distance and both azimuths
- tolerance: 1e-8 km on the distance, 1e-9° on the azimuths
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The reference is Vincenty's own paper, written out in full in `tools/vectors/gen_nav_sphere_vincenty.py` — the reduced latitudes, the λ iteration, the A and B series, the σ correction. Checking against GeographicLib would be checking the wrong thing: it computes a different, better answer on purpose, and the whole value of this tool is that it does not.

## Differential tests

- `tools/vectors/gen_nav_sphere_vincenty.py`: 16 of the 21 vectors, from the 1975 paper
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.vincenty-inverse.jsonl`: 21 pairs across the globe

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `vincenty_inverse_invariants`: a quarter of the equator is a quarter of the ellipsoid's equatorial circumference, 2πa/4, which is exact arithmetic and independent of the series; the distance is symmetric and the azimuths swap when the points do; it agrees with Karney's to under a millimetre on every ordinary line, which is what makes the two comparable rather than merely different; the reported `karney_difference` is exactly that gap; and a nearly antipodal pair is refused rather than answered, since the iteration does not converge there
