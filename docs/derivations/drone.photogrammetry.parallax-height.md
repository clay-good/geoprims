<!-- Derivation note (trust/correctness-program layer A). -->
# Height from stereo parallax (`drone.photogrammetry.parallax-height`)

## Method

Two truly vertical photos taken from the same flying height a distance B apart image every point twice. The parallax p = x − x′, the shift of its image along the flight line, is inversely proportional to the lens's height above the point. That gives the point's elevation absolutely from p, B, and the focal length, and, more robustly, from the parallax difference to a nearby control point of known elevation, where systematic errors cancel.

## Equations

- h = H − f B / p; X = x B / p; Y = y B / p (EM 1110-1-1000 eq. 10-14)
- h_A = h_C + Δp (H − h_C) / p_A, with p_A = p_C + Δp (eq. 10-15)

The difference form follows from eq. 10-14 at both points: H − h_A = f B / p_A and H − h_C = f B / p_C, so h_A = h_C + (H − h_C)(p_A − p_C) / p_A. The manual prints the difference as P_c − P_a; the derivation needs p_A − p_C, positive when A is higher, which the tool uses.

## Symbols and units

H is the flying height above the datum; h_C the control point's elevation; p_C and p_A the parallaxes of the control point and the point; Δp = p_A − p_C; f the focal length; B the air base; x and y the point's coordinates on the left photo. Parallax, focal length, and photo coordinates share one photo unit; heights and B share a ground unit.

## Domain

The difference form needs p_C and Δp, with p_C and p_A more than zero and H above h_C. The absolute form needs p, f, and B, each more than zero.

## Approximations

Exact for truly vertical photos from equal flying heights with parallax measured along the flight line. Tilt, unequal heights, and lens distortion upset both forms, the absolute one most.

## Worked example

- sourcePublisher: U.S. Army Corps of Engineers
- sourceTitle: EM 1110-1-1000, Photogrammetric Mapping
- sourceEdition: 31 March 1993
- sourceLocator: Chapter 10, equation 10-15, h_A = h_C + Δp(H − h_C) / P_a, read in the open copy the University of Arizona hosts
- independent: yes
- inputs: H = 915 m, control point on the ground (0 m) with parallax 88.2 mm, Δp = 1.3 mm
- outputs: 13.29 m, from 1.3 × 915 / 89.5, the divisor being the unknown point's own parallax P_a = P_c + Δp. Read with 88.2 mm as the unknown point's parallax instead, the equation gives 13.5 m, so 88.2 mm is the control point's. Wolf, Dewitt, and Wilkinson's example 8-3 is the same case and prints about 13.3 m.
- tolerance: 0.05 m
- verifiedBy: golden vector v001 and `parallax_height_invariants`, with the same equation in feet and inches as v002 (0.6 × 4600 / 5.0 = 552 ft)
- verifiedOn: 2026-10-28

The equation is the authority for this vector's digits, and it was read at an open source. Wolf's example is the agreement check only: that book is copyrighted and not open, and its example was never read here.

The manual prints Δp as P_c − P_a, which is a sign slip against its own equation 10-14: a point higher than the control has the **greater** parallax, so Δp must be P_a − P_c for equation 10-15 to raise the elevation rather than lower it. This tool takes Δp with that sign, which is what makes both worked cases come out right.

## Differential tests

- `tools/vectors/gen_classic.py` works every vector in Python from the equations above.
- `core/vectors/drone.photogrammetry.parallax-height.jsonl` holds 21 vectors: the tree, the Washington Monument example (552 ft), eleven parallax differences of both signs, five absolute cases with ground X and Y, and three refusals.

## Invariants

- `core/crates/gp-drone/tests/classic.rs` `parallax_height_invariants`: a zero difference returns the control elevation; at four elevation pairs, parallaxes taken from eq. 10-14 and differenced give back the point's elevation through eq. 10-15, and the absolute form agrees; the tree to 0.1 m.
