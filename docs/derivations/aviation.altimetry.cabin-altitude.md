<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Cabin altitude from pressure differential (`aviation.altimetry.cabin-altitude`)

## Method

A pressurized cabin holds its pressure above the outside pressure by up to the airplane's maximum differential. The tool finds the outside pressure at the airplane's pressure altitude in the standard atmosphere, adds the differential, and turns that cabin pressure back into a pressure altitude. Both steps use the site's standard-atmosphere engine (`atmosphere::at` and `altitude_for_pressure`, ICAO Doc 7488). With a selected cabin altitude, the tool finds the differential that cabin needs. If that is no more than the maximum, it uses only that much and reports "yes". If not, it uses the full maximum and reports "no". A cabin at or above the airplane needs no differential.

## Equations

- Outside pressure: p_out = p_ISA(h).
- Without a selected cabin: p_cabin = p_out + Δp_max.
- With a selected cabin c: need = max(p_ISA(c) − p_out, 0); if need ≤ Δp_max, p_cabin = p_out + need ("yes"), else p_cabin = p_out + Δp_max ("no").
- Differential in use: Δp = p_cabin − p_out.
- Cabin altitude: the ISA pressure altitude of p_cabin.

## Symbols and units

h, c, and the cabin altitude in feet by default (any length unit accepted); pressures and differentials in psi by default (any pressure unit). The arithmetic runs in meters and pascals and is converted back for display. The cabin altitude is shown to whole feet and the pressures to two decimals.

## Domain

Pressure altitude from −2,000 to 65,000 ft, else OUT_OF_DOMAIN at `/pressure_altitude`. Maximum differential from 0 to 100,000 Pa (about 14.5 psi), else INVALID_INPUT at `/max_differential`. A selected cabin altitude from −2,000 to 65,000 ft, else OUT_OF_DOMAIN at `/cabin_selected`.

## Approximations

The tool works in pressure altitude through the standard atmosphere and assumes the cabin holds the full differential, or the part a selected cabin needs. Real cabins follow the controller's schedule and the airplane's limits, and the maximum differential comes from the flight manual. The tool says nothing about oxygen rules.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C
- sourceLocator: Chapter 7, cabin pressurization, the text beside Figure 7-41: at 28,000 ft the outside pressure is 4.8 psi, and adding a 6.1 psid differential gives 10.9 psi, the pressure at 8,000 ft (read at faa.gov on 2026-10-20)
- independent: yes
- inputs: pressure altitude 28,000 ft, maximum differential 6.1 psi
- outputs: cabin pressure 10.876 psi and cabin altitude 8,095 ft (8,094.646954938287); the figure gives about 10.9 psi and 8,000 ft
- tolerance: 0.05 psi and 120 ft against the figure; the vector holds the Python ISA values to 1e-6 psi and 1e-4 ft
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `cabin_altitude_invariants`
- verifiedOn: 2026-10-20

The figure rounds. Its 10.9 psi is 8,038 ft, and 0.05 psi of rounding in the differential moves the answer about 100 ft, so the test holds the figure to 120 ft rather than the spec's 50 ft.

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field says the standard atmosphere (Doc 7488) was worked independently in Python.
- `core/vectors/aviation.altimetry.cabin-altitude.jsonl`: 21 vectors. The PHAK example (v001, and again with the outside pressure as v014); 13 more flight altitudes from sea level to 51,000 ft; four selected cabins, one the maximum cannot hold (v017) and one above the airplane that needs no differential (v019); and two refusals: 70,000 ft (OUT_OF_DOMAIN) and a −1 psi differential (INVALID_INPUT).

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `cabin_altitude_invariants`: the PHAK example within 0.05 psi of 10.9 psi and within 120 ft of 8,000 ft; an 8,000 ft cabin selected at 12,000 ft with 8 psi available uses about 1.6 psi (within 0.05) and holds 8,000 ft within 1e-6 ft; zero differential leaves the cabin at the airplane's 20,000 ft; and at 30,000 ft, 8 psi gives a lower cabin than 6 psi.
