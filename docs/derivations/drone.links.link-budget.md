<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Radio link budget (`drone.links.link-budget`)

## Method

A radio link holds when the signal that arrives is stronger than the receiver needs. The tool adds up the link in decibels: the transmit power and both antenna gains, less the cable losses and the loss of spreading through free space. What is left over above the receiver's sensitivity is the fade margin. It also works out the radiated power (EIRP) and, in the 2.4 GHz band, compares it with the limit for the United States or the European Union, read from dated entries in `data/regulations.json`.

## Equations

- Free-space loss: L = 20 log₁₀(4πd ÷ λ) = 20 log₁₀(4πdf ÷ c). With d in kilometers and f in megahertz, L = 20 log₁₀ d + 20 log₁₀ f + 32.4478 dB.
- Received power = transmit power + transmit gain + receive gain − cable loss − L.
- Fade margin = received power − receiver sensitivity.
- EIRP = transmit power + transmit gain − half the cable loss (the half taken as the transmit side).
- EIRP verdict, for 2,400 to 2,483.5 MHz only: beyond the limit, near it (within 10% of the limit in milliwatts, about 0.46 dB), or within it. The limit is 36 dBm in the US and 20 dBm in the EU.

## Symbols and units

c = 299,792,458 m/s. Powers in dBm, gains in dBi, losses in dB. Frequency in any frequency unit (MHz by default), distance in any distance unit (km by default). The constant 32.4478 is 20 log₁₀(4π × 10⁹ ÷ c); ITU prints it as 32.4.

## Domain

Frequency above 0 and up to 1 THz; distance above 0 and up to 100,000 km. A sensitivity without a transmit power is INVALID_INPUT.

## Approximations

Free space only. Terrain, the ground reflection, bodies, vegetation, and rain add loss, and a clear Fresnel zone and line of sight are assumed; the tool's related tools check those. The EIRP figure splits the cable loss evenly between the two ends unless only one end has any. The limits are the general ones for the band: the US rule allows more gain on fixed point-to-point links, and other bands have other rules, which the tool does not hold. Equipment certification governs.

## Worked example

- sourcePublisher: International Telecommunication Union; Federal Communications Commission (eCFR); ETSI
- sourceTitle: Recommendation ITU-R P.525-5, Calculation of free-space attenuation; 47 CFR 15.247; ETSI EN 300 328 V2.2.2
- sourceEdition: P.525-5 (11/2024); eCFR as in force 2026-09-01; EN 300 328 V2.2.2 (2019-07); all read 2026-10-10
- sourceLocator: P.525-5 equation 5, L = 20 log (4πd/λ), and equation 6, L = 32.4 + 20 log f + 20 log d (MHz, km); 15.247(b)(3) "1 Watt" and (b)(4) antennas "that do not exceed 6 dBi", so 36 dBm EIRP; EN 300 328 clause 4.3.2.2.3, "equal to or less than 20 dBm"
- independent: yes
- inputs: 2,400 MHz over 5 km; and at 2,450 MHz, 30 dBm into a 6 dBi antenna in the US, and 14 dBm into a 6 dBi antenna in the EU
- outputs: 114.03 dB of free-space loss (equation 6 gives 32.4 + 67.6 + 14.0 = 114.0); EIRP 36 dBm, at the US limit; EIRP 20 dBm, at the EU limit
- tolerance: 0.05 dB against equation 6 as printed; 1e-9 dB against equation 5
- verifiedBy: golden vectors v006, v010, and v015, run by the core on every build
- verifiedOn: 2026-10-10

Equation 6 is equation 5 with its constant rounded to one decimal, so it checks the loss to a tenth of a decibel; the vectors work equation 5 itself, in meters and hertz.

## Differential tests

- `tools/vectors/gen_sensing.py`: the loss worked from equation 5 in meters and hertz, where the core adds logarithms in kilometers and megahertz; ten links placed within, near, and beyond each 2.4 GHz limit; five other bands and distance units; and a refused zero distance
- `core/vectors/drone.links.link-budget.jsonl`: those vectors, run through the core on every build; v001 to v004 are superseded, as they pinned the rounded constant version 1.0 used

## Invariants

- `core/crates/gp-drone/tests/planning.rs` `link_budget_invariants`: the loss equals equation 5 and rises 6.02 dB per doubling of distance or frequency, the received power and fade margin are the sums of their parts, and the EIRP verdict turns at 36 dBm and 20 dBm inside the 2.4 GHz band and is withheld outside it
