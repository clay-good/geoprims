<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GPS satellite visibility and DOP (`survey.gnss.dop`)

## Method

A GPS almanac gives each satellite's orbit as eight Keplerian values at a reference time. The tool reads the YUMA or SEM text the Coast Guard Navigation Center publishes (SEM gives angles in semicircles and the inclination as an offset from 0.30 semicircles, IS-GPS-200 Table 20-VI), resolves its ten-bit week to the full week nearest the start, and leaves out satellites marked unhealthy. At each sample in the window it places every satellite with the IS-GPS-200 user algorithm for almanac data: the mean anomaly advanced at the orbit's mean motion, Kepler's equation solved for the eccentric anomaly, the position in the orbital plane, and that plane turned by the inclination and by a node that regresses at the almanac's rate while the Earth turns underneath. The site is placed on WGS 84, and each satellite is seen from it as an azimuth, an elevation, and a unit line of sight in east, north, and up. Satellites at or above the elevation mask, and above the skyline where one is entered, make the rows of the geometry matrix, and the DOP values come from the diagonal of the inverse of GᵀG.

## Equations

- Time: GPS seconds = UTC seconds since 1980-01-06 + (TAI − UTC) − 19 s; tₖ = t − (week × 604,800 + t_oa).
- Mean motion n = √(μ / A³), A = (√A)², μ = 3.986005 × 10¹⁴ m³/s².
- Mₖ = M₀ + n tₖ; Eₖ − e sin Eₖ = Mₖ (Newton's method); νₖ = atan2(√(1 − e²) sin Eₖ, cos Eₖ − e).
- rₖ = A (1 − e cos Eₖ), uₖ = νₖ + ω; in-plane x′ = rₖ cos uₖ, y′ = rₖ sin uₖ.
- Ωₖ = Ω₀ + (Ω̇ − Ω̇ₑ) tₖ − Ω̇ₑ t_oa, Ω̇ₑ = 7.2921151467 × 10⁻⁵ rad/s.
- ECEF: x = x′ cos Ωₖ − y′ cos i sin Ωₖ, y = x′ sin Ωₖ + y′ cos i cos Ωₖ, z = y′ sin i.
- Line of sight in ENU, u = (e, n, u)/ρ; G rows [−uₑ, −uₙ, −uᵤ, 1]; Q = (GᵀG)⁻¹.
- GDOP = √(q₁₁ + q₂₂ + q₃₃ + q₄₄), PDOP = √(q₁₁ + q₂₂ + q₃₃), HDOP = √(q₁₁ + q₂₂), VDOP = √q₃₃.

## Symbols and units

e is the eccentricity, i the inclination, Ω₀ the right ascension at the weekly epoch, Ω̇ its rate, ω the argument of perigee, M₀ the mean anomaly, √A the square root of the semi-major axis in √m, and t_oa the time of applicability in seconds of the GPS week; angles in radians as the almanac prints them. Azimuth is clockwise from true north and elevation is above the ellipsoid's tangent plane, both in degrees. DOP values are unitless.

## Domain

Starts from 1980-01-06 on, windows up to 14 days at a step of a minute or more and at most 2,000 samples, elevation masks from −5° to 60°, and a skyline of up to 360 azimuth and elevation points, linear between them and around through north. DOP needs four or more satellites; with fewer, the sample reports its count and no DOP. An almanac reference more than 7 days before the start raises ALMANAC_OLD.

## Approximations

The almanac orbit leaves out the harmonic corrections and the precise terms of the broadcast ephemeris, so a satellite is placed to a few kilometers when the almanac is current, and that error grows as it ages; from 20,000 km away a few kilometers is hundredths of a degree. The light time and the Earth's rotation during it are left out, a few hundredths of a degree more. The satellite clock terms (Af0, Af1) do not affect geometry and are not used. GPS time comes from the leap-second table, so a start past its expiry could be a second off, which does not matter for planning.

## Worked example

- sourcePublisher: geoprims (computed, not published)
- sourceTitle: a synthetic 24-satellite constellation written as a YUMA almanac, worked in Python with numpy (tools/vectors/gen_dop.py)
- sourceEdition: IS-GPS-200N
- sourceLocator: core/vectors/survey.gnss.dop.jsonl, v001
- independent: no
- inputs: the constellation over Denver (39.74 N, 104.99 W) from 2026-10-03T14:00Z for 6 hours every 30 minutes, 10° mask
- outputs: 7 of 8 satellites in the sky used at the start, PDOP 2.0; worst PDOP 3.2 at 15:00Z
- tolerance: 1e-7 relative, 1e-6 absolute
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

The vectors are a second transcription of the same standard, not a published example. Against an independent source the core was also checked on 2026-10-09: with the Coast Guard's almanac for week 392, the azimuth and elevation of 34 satellite sightings from Denver, Sydney, and Greenwich were compared with Skyfield's SGP4 propagation of CelesTrak's GPS elements for the same day (tools/vectors/check_dop_skyfield.py). 33 agreed within 0.1° in elevation, most within 0.03°. PRN 13 differed by 2.7°, which neither file explains; a satellite reassigned between PRNs or repositioning between the two sources' epochs would do it, and it was not investigated further.

## Differential tests

- `tools/vectors/gen_dop.py`: 26 vectors from a numpy transcription of IS-GPS-200 with a different Kepler solver, rotation form, and matrix inverse, at nine sites, masks from 0° to 20°, skylines, an old almanac, unhealthy satellites, and too few satellites, four of them from the constellation written as a SEM almanac
- `tools/vectors/check_dop_skyfield.py`: azimuth and elevation against Skyfield's SGP4 on CelesTrak elements (manual; needs the network)

## Invariants

- `core/crates/gp-survey/tests/dop.rs` `a_coast_guard_almanac_parses`: two records of the Coast Guard's own file parse to the printed values, a missing line names its satellite, and a repeated PRN is refused
- `core/crates/gp-survey/tests/dop.rs` `a_sem_almanac_reads_as_the_same_orbits`: the Coast Guard's SEM and YUMA files for the same week give the same orbits to YUMA's printed precision, and the format is told apart by its labels
- `core/crates/gp-survey/tests/dop.rs` `the_same_instant_in_any_offset_gives_the_same_sky`: UTC and an offset for the same instant give identical results
- `core/crates/gp-survey/tests/dop.rs` `record_order_and_week_form_do_not_matter`: reversing the records, or writing the full week for the ten-bit one, changes nothing
- `core/crates/gp-survey/tests/dop.rs` `a_higher_mask_never_helps`: raising the mask never adds a satellite or lowers PDOP
- `core/crates/gp-survey/tests/dop.rs` `a_skyline_under_the_mask_changes_nothing`: a skyline under the mask has no effect, and a wall leaves out exactly the satellites behind it
- `core/crates/gp-survey/tests/dop.rs` `unhealthy_satellites_are_left_out`: marking one satellite unhealthy removes it
- `core/crates/gp-survey/tests/dop.rs` `an_old_almanac_is_flagged`: a start 17 days after the reference raises ALMANAC_OLD, and the example does not
