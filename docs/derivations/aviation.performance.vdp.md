<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Visual descent point (`aviation.performance.vdp`)

## Method

The point on a non-precision approach where a normal descent from the MDA reaches the threshold at the threshold crossing height: a straight path at the descent angle. The HAT/300 rule of thumb (300 ft per NM on a three-degree path) is shown with its error. With a groundspeed, the vertical speed and the time to the threshold are added.

## Equations

- Distance = (HAT − TCH) / tan θ.
- Rule of thumb: distance ≈ HAT / 300 NM (and HAT / 318 for the exact 3° gradient of 318 ft/NM).
- Vertical speed = GS × tan θ.

## Symbols and units

HAT height above touchdown at the MDA (ft), TCH threshold crossing height (ft, 0 by default), θ descent angle (3° by default), GS groundspeed (kt). Distance in NM.

## Domain

A HAT above the TCH, and an angle above 0° up to 10°.

## Approximations

A straight path over a level runway environment. A VDP published on the approach chart always governs, and the tool says so.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Procedures Handbook (FAA-H-8083-16B)
- sourceEdition: FAA-H-8083-16B
- sourceLocator: Chapter 4, descent to the touchdown zone: on an approximate three-degree glidepath the height above the TDZE is 300 ft per NM (3,000 ft at 10 NM, 1,500 ft at 5 NM, 600 ft at 2 NM, 450 ft at 1.5 NM)
- independent: yes
- inputs: HAT 450 ft; HAT 600 ft
- outputs: HAT/300 rule 1.5 NM and 2 NM (the exact 3° path gives 1.41 NM and 1.89 NM, with the difference shown)
- tolerance: exact
- verifiedBy: golden vectors v019 and v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: a separate Python implementation at 18 approaches, HAT 250 ft to 900 ft, TCH 0 ft to 60 ft, angles 2.5° to 3.5° (within 1e-9 relative)
- `core/vectors/aviation.performance.vdp.jsonl`: those vectors and the IPH examples, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `descent_invariants`: distance × tan θ equals HAT − TCH, and the HAT/300 rule is HAT/300 on a 3° path
