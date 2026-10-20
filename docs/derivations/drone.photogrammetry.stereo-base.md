<!-- Derivation note (trust/correctness-program layer A). -->
# Air base and base-to-height ratio (`drone.photogrammetry.stereo-base`)

## Method

Each photo covers a ground length set by the frame's length along track and the scale. End lap is the share of that length the next photo repeats, so the air base, the distance between exposures, is the rest. The base-to-height ratio is the geometry of the stereo pair. Differentiating the parallax height equation shows how a parallax measurement error becomes a height error, which is why more end lap, with its shorter base, gives weaker heights from each pair.

## Equations

- G = d H / f (EM 1110-1-1000 eq. 10-9 with S = f / H)
- B = G (1 − E / 100) (eq. 10-11)
- B / H = (1 − E / 100) d / f
- σ_h = (H / B)(H / f) σ_p, from h = H − f B / p (eq. 10-14): |dh / dp| = f B / p² = (H − h)² / (f B)

## Symbols and units

H is the height above ground, f the focal length, d the frame's length along track, E the end lap in percent, G the ground coverage along track, B the air base, σ_p the precision of one parallax measurement on the photo, and σ_h the height precision it implies. Lengths take any unit.

## Domain

H, f, and d must be more than zero; E is 0 to 95 percent; σ_p cannot be negative.

## Approximations

Vertical photos over flat ground. σ_h is the geometric precision of one parallax measurement in one pair, not the accuracy of a mapping product; ASPRS assesses product accuracy with checkpoints, and processing that combines many photos does better than one pair.

## Worked example

- sourcePublisher: U.S. Army Corps of Engineers
- sourceTitle: EM 1110-1-1000, Photogrammetric Mapping
- sourceEdition: 31 March 1993
- sourceLocator: Chapter 10, eq. 10-9 and 10-11, applied by hand; the Penn State plan the design names (10,000 ft, 60% end lap, 2,800 ft) was not found in the lesson
- independent: no
- inputs: 10,000 ft above ground, 100 mm lens, 70 mm frame along track, 60% end lap
- outputs: 7,000 ft coverage, 2,800 ft air base, B/H 0.28
- tolerance: 1e-9 relative
- verifiedBy: golden vector v001 and `stereo_base_invariants`
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_classic.py` works every vector in Python from the equations above.
- `core/vectors/drone.photogrammetry.stereo-base.jsonl` holds 22 vectors over film and drone cameras, end laps from 0 to 95%, with and without a parallax precision, and two refusals.

## Invariants

- `core/crates/gp-drone/tests/classic.rs` `stereo_base_invariants`: the 2,800 ft base at 60%; from 60% to 80% end lap B/H halves and σ_h doubles; σ_h is linear in σ_p.
