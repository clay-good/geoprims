## Context

Research gathered 2026-10-19. The free primary is USACE EM 1110-1-1000, Photogrammetric Mapping, 31 March 1993, chapter 10 "Principles of Photogrammetry," a US Government work with the equations numbered (10-2 scale, 10-3 ground distance, 10-6 relief displacement, 10-14 parallax equations, 10-15 parallax difference). Wolf, Dewitt & Wilkinson (4th ed., 2014) is the textbook whose examples circulate (example 6-7 tower by relief displacement; example 8-3 tree by parallax difference); it is copyrighted, so its examples are cited as vectors, not reproduced as prose. The vertical-precision relation σ_h = (H/B)(H/f)σ_p has no single primary; it is the derivative of the parallax height equation and is labeled derived.

## Goals / Non-Goals

**Goals:**
- The four classic relations, exactly, with the textbook examples reproduced and the geometry drawn.

**Non-Goals:**
- Accuracy prediction for products. Image processing.

## Decisions

### PC1. Flying height is always "above the point", not above takeoff
Every tool takes H as height above the ground point (or above datum with the point's elevation), reusing the typed heights of the drone suite (DR3), so the common error of using height above takeoff is caught at the input, with the terrain tools offered for the elevation.

### PC2. Formulas
- Scale: S = f / H′ with H′ = H − h_avg (eq. 10-2); ground distance D = d × H′ / f (10-3). Both directions: scale from height, height for a scale. Scale is shown as a ratio (1:n) and as GSD when pixel pitch is given, tying it to the existing GSD tool (same function).
- Relief displacement: d = r × h / H (10-6), solved for any one of d, r, h, H; radial distance r measured from the principal point; the note that the displacement is outward for points above datum.
- Parallax height: h = H − f × B / p (10-14) and the difference form h_A = h_C + Δp (H − h_C) / p_a (10-15), with parallax measured in the same units as f; X and Y ground coordinates from x B / p and y B / p offered as outputs.
- Stereo base: B = (1 − endlap) × (H × sensor length / f) along track; B/H; σ_h = (H/B)(H/f) σ_p labeled "derived from eq. 10-14"; the ASPRS note that this is a geometric precision, not a product accuracy.

### PC3. Verification
- Wolf et al. example 6-7 (H = 535 m, base 259 m above datum, d = 54.1 mm, r = 121.7 mm → h ≈ 122.7 m) and example 8-3 (Δp = 1.3 mm, H = 915 m, p_a = 88.2 mm → h ≈ 13.3 m), to the printed 0.1 m; the Washington Monument parallax example (H = 4,600 ft, Δp = 0.6 in, p = 4.4 in → 552 ft).
- Penn State lesson 4 flight plan (100 mm lens, 10 µm pixel, 1 ft GSD → 10,000 ft AGL; 60% endlap → 2,800 ft airbase).
- Invariants: scale × ground distance round trip; relief displacement solved each way round-trips to 1e-9; parallax height with Δp = 0 returns h_C; σ_h scales linearly with σ_p.

## Tool inventory

| Tool | Endpoints | Reference | Independent vector |
|---|---|---|---|
| photo-scale | 1 | EM 1110-1-1000 eq. 10-2, 10-3 | Penn State lesson 4 |
| relief-displacement | 1 | EM 1110-1-1000 eq. 10-6 | Wolf example 6-7 |
| parallax-height | 1 | EM 1110-1-1000 eq. 10-14, 10-15 | Wolf example 8-3; Washington Monument |
| stereo-base | 1 | Penn State lesson 4; derived σ_h | Penn State airbase |

Total: 4 operations, 4 endpoints. The drone rollup moves from 49 to 53 operations.

## Risks / Trade-offs

- **[σ_h read as product accuracy]** → Labeled derived and geometric; the ASPRS checkpoint rule is quoted; the sentence says "precision of one parallax measurement."
- **[Textbook examples are rounded]** → Tolerances follow the printed precision.

## Migration Plan

Not applicable. New tools start experimental.

## Open Questions

None.
