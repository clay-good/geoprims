<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ITRF or WGS 84 to ETRS89 (`geodesy.datum.etrf`)

## Method

A satellite gives a position in a global frame — ITRF, or the WGS 84 realization aligned with it — and a European national grid is built on ETRS89, which is pinned to the Eurasian plate as it sat in 1989. The plate has been moving ever since, about 2.5 cm a year, so the two frames drift apart: a position measured today is most of a metre from the ETRS89 the grid expects. This tool carries a position across that gap, and back.

It goes in two steps, both published. First the frame given is carried onto ITRF2014 by the IERS parameters the neighbouring ITRF tool uses, because that is the frame the European tie is published from. Then ITRF2014 goes to ETRF2000 by EPSG's own operation: a time-dependent position-vector transformation (EPSG method 1053) with a reference epoch of 2010.0, whose rotation rates are what hold ETRS89 to the plate. The years between 2010 and the coordinates' epoch are what make the answer depend on when the position was measured — which is why the epoch is a required input and why "today's date" is the wrong thing to put there unless the coordinates were measured today.

ETRF2000 is the realization the EUREF Technical Working Group recommends as ETRS89, which is why it is the one here; ETRF2014 and ETRF2020 exist and are technically better, and a grid tied to one of those is a different step.

## Equations

- `V_ITRF2014 = IERS tie(frame → ITRF2014, t)` applied to the geocentric position, as the ITRF tool does it.
- `V_ETRF2000 = T(t) + (1 + dS(t))·R(r(t))·V_ITRF2014`, position-vector convention, with each parameter `p(t) = p₀ + ṗ·(t − 2010.0)`: translations (0.0547, 0.0522, −0.0741) m with rates (0.0001, 0.0001, −0.0019) m/yr; rotations (0.001701, 0.01029, −0.016632)″ with rates (0.000081, 0.00049, −0.000792)″/yr; scale 0.00212 ppm with rate 0.00011 ppm/yr.
- The shift reported is the east-north offset between the two coordinates of the point.

## Symbols and units

Latitude and longitude in degrees; ellipsoidal height in meters above GRS 80; the epoch as a decimal year or a date; translations in meters, rotations in arc-seconds, scale in ppm, each with a rate per year; the shift in meters and its direction in degrees from true north.

## Domain

Epochs from 1980 to 2100, else `OUT_OF_DOMAIN`. EPSG publishes the tie for the ETRF extent, about 16°W to 33°E and 33°N to 84°N; outside it the answer carries `OUTSIDE_AREA_OF_USE` because the transformation is then extrapolation. An unqualified "WGS 84" is taken as the current realization and says so.

## Approximations

The tie is a rigid transformation of the whole plate: it knows nothing of local deformation, and EPSG states it at 0.01 m. The ITRF-to-ITRF leg adds the few millimetres the IERS parameters carry. Heights are ellipsoidal; a national height datum is its own step. The result depends on the epoch, and an epoch guessed wrong by a decade is a quarter of a metre wrong.

## Worked example

- sourcePublisher: IOGP, from the EUREF Technical Working Group, through PROJ
- sourceTitle: EPSG Geodetic Parameter Dataset, ITRF2014 to ETRF2000 (1)
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: EPSG method 1053, time-dependent position vector, reference epoch 2010.0, with the parameters above; applied by PROJ after the IERS tie from ITRF2020
- independent: yes
- inputs: direction to-etrs89, frame ITRF2020, epoch 2026.0, lat 53.349806, lon −6.260247, height 85 m
- outputs: the Dublin point moves 0.8382 m at 217.4° true, south-west, which is the plate's motion since 1989 as the frames record it
- tolerance: 1e-9° and 0.1 mm
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/datum.rs` `etrs89_drifts_from_the_global_frames_with_the_epoch`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_etrf.py` builds the 26 vectors in `core/vectors/geodesy.datum.etrf.jsonl` from PROJ 9.9.0 through `cct`, applying the same two steps in the same order, so the parameters are EPSG's and IERS's rather than this repository's: eight places from Lisbon to Helsinki at epoch 2026, Dublin and Zurich at 2015 and 2000 as well, each one also run backwards to the point it started from, a point outside the published area, and the epoch refusal.

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `etrs89_drifts_from_the_global_frames_with_the_epoch`: the shift grows with the epoch, measured at Dublin as 0.4823 m in 2010 and 0.8382 m in 2026, which is about 2.2 cm a year; it runs south-west across Europe; the two directions are exact inverses; and a point outside the published area is answered and flagged.
