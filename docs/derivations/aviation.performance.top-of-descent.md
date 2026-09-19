<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Top of descent (`aviation.performance.top-of-descent`)

## Method

A straight descent at a constant angle and groundspeed over flat ground. The distance to start down is the altitude to lose over the tangent of the descent angle. The vertical speed is the groundspeed times that tangent. For paths from 2.5° to 3.5°, the 3-to-1 rule (3 NM per 1,000 ft) and the 5 × groundspeed rule are shown with their errors.

## Equations

- Distance = Δh / tan θ; gradient = tan θ (ft per NM), time = distance / GS.
- Vertical speed = GS × tan θ (GS in ft/min).
- Rules of thumb: distance ≈ 3 NM × Δh / 1,000 ft, and vertical speed ≈ 5 × GS (kt) ft/min.

## Symbols and units

Δh altitude to lose (ft), θ descent angle (3° by default), GS groundspeed (kt). Distance in NM, vertical speed in ft/min, time in minutes.

## Domain

A start altitude above the target altitude, a positive groundspeed, and an angle above 0° up to 10°.

## Approximations

Constant angle and groundspeed. Real descents slow down, meet changing winds, and follow ATC restrictions, and the IPH's tailwind and headwind adjustment (2 NM per 10 kt) is not applied.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Procedures Handbook (FAA-H-8083-16B)
- sourceEdition: FAA-H-8083-16B
- sourceLocator: Chapter 3, descent planning: from FL 310 to an approach gate at 6,000 ft, 25 × 3 = 75 NM by the 3-to-1 rule
- independent: yes
- inputs: from 31,000 ft to 6,000 ft at 420 kt
- outputs: 3-to-1 rule 75 NM (the exact 3° path is 78.5 NM, and the tool shows the 3.5 NM difference)
- tolerance: exact
- verifiedBy: golden vector v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: a separate Python implementation at 19 descents, from 4,500 ft to FL 450 and 2.2° to 5° (within 1e-9 relative)
- `core/vectors/aviation.performance.top-of-descent.jsonl`: those vectors and the IPH example, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `descent_invariants`: distance × gradient is the altitude to lose, the vertical speed is GS × tan θ, time is distance over GS, and the 3-to-1 rule appears only for 2.5° to 3.5° paths
