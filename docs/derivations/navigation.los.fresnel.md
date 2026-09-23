<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fresnel zone clearance (`navigation.los.fresnel`)

## Method

A radio signal does not travel along the straight line between two antennas but through a family of ellipsoids around it, and the first of them carries most of the energy. An obstacle that pushes into that zone costs signal even when the line itself is clear, so a path is planned to keep the zone clear rather than the line. The first zone's radius at a point on the path is where the detour is half a wavelength longer than the direct route, which for a path long against its width gives the familiar square root. The Earth's own curve lifts the ground into the path as well, by an amount that depends on how the atmosphere bends the ray, so the two are computed on the same path and added: the radius of the zone the link needs, at the usual 60% planning fraction, plus the bulge underneath it.

## Equations

- First Fresnel radius at a point splitting the path into d₁ and d₂: F₁ = √( λ d₁ d₂ / d ), with d = d₁ + d₂ and λ the wavelength.
- Wavelength from frequency: λ = c / f, with c = 299,792,458 m/s.
- Planning fraction: 0.6 F₁, the clearance a link is normally designed to.
- Earth bulge on the effective radius: b = d₁ d₂ / (2 Rₑ), with Rₑ = K·R, K = 4/3 by default.
- Required clearance above a smooth Earth: 0.6 F₁ + b.

## Symbols and units

f is the frequency and λ the wavelength; d₁ and d₂ the distances from each end to the point, d the whole path, all in meters and shown in the chosen unit; F₁ the first Fresnel radius in meters; R the Earth's radius (6,371,000 m) and K the refractive factor, 4/3 unless given, which is the same quantity the rest of this family writes as k through K = 1/(1 − k).

## Domain

Any positive frequency and path length, and any point along the path; the default is the midpoint, where the zone is widest and the bulge greatest, which is where a path is normally checked. The square-root form assumes the path is long against the zone's width, which holds by an enormous margin for any real radio link — at 5.8 GHz over 10 km the zone is 11 m across a 10,000 m path.

## Approximations

Two, both small and both the standard ones. The Fresnel radius uses the far-field form rather than the exact ellipsoid, which is exact to the precision shown for any path whose length exceeds its zone width by orders of magnitude. The bulge uses the effective-Earth model, replacing real refraction with a scaled radius; that is an approximation of the atmosphere rather than of the arithmetic, and it is the dominant uncertainty here, since real K varies with the weather.

## Worked example

- sourcePublisher: International Telecommunication Union
- sourceTitle: Recommendation ITU-R P.530, propagation data and prediction methods for terrestrial line-of-sight systems
- sourceEdition: P.530
- sourceLocator: the printed working form F₁ = 17.3 √( d₁ d₂ / (f d) ), with d in km, f in GHz and F₁ in meters, alongside the microwave-path Earth bulge d₁ d₂ / (12.75 K)
- independent: yes
- inputs: 5.8 GHz over 10 km, at the midpoint, K = 4/3
- outputs: first Fresnel radius 11.3675 m, 60% clearance 6.8205 m, Earth bulge 1.4715 m, required clearance 8.2920 m
- tolerance: 0.1%, which is what the published constants' rounding costs
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-23

Both published constants are checked, and neither is a restatement of the core's formula: each is a different quantity's worth of arithmetic already folded into a printed number. ITU's 17.3 is √(c/10⁶) with the units absorbed, whose unrounded value is 17.3145; at 5.8 GHz over 10 km it gives 11.3580 m against the tool's 11.3675 m. The microwave-path 12.75 is 2R/1000 with R taken as 6,375 km; it gives a bulge of 1.4706 m against the tool's 1.4715 m.

What makes this a check rather than a coincidence is that both shortfalls are constant. ITU's form is 0.084% low at 5.8 GHz over 10 km, at 2.4 GHz over 10 km, and at 0.9 GHz over 50 km — the same figure, because 17.3/17.3145 is a constant and nothing else differs. The bulge constant is 0.063% low at 10 km, at 50 km, and at K = 1 as well as K = 4/3, because 6371/6375 is a constant. A formula error would not hold still like that across frequency, distance, and K.

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `fresnel_matches_the_published_constants`: the ITU 17.3 form and the 12.75 bulge form over a grid of frequencies, path lengths, split points, and K factors, each held to its own constant shortfall rather than to a loose band
- `tools/vectors/gen_los.py`: 22 vectors across frequency, distance, and position on the path
- `core/vectors/navigation.los.fresnel.jsonl`: 23 vectors, the last from the two published constants

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `fresnel_invariants`: the required clearance is the 60% figure plus the bulge, and the 60% figure is six tenths of the radius; the zone is widest at the midpoint and narrows to nothing at either end; the radius scales as the inverse square root of frequency and as the square root of path length, both checked by doubling rather than by recomputing the formula; the bulge is proportional to d₁d₂ and inversely proportional to K; and the Fresnel radius does not depend on K, nor the bulge on frequency, which is what keeps the two terms separable
