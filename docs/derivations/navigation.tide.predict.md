<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Tide prediction from harmonic constituents (`navigation.tide.predict`)

## Method

The tide at a station is the sum of cosine waves, one per harmonic constituent, each with its own speed. NOAA publishes each constituent's amplitude H and phase κ for a station; the reader pastes them (the station page's table or `harcon.json`) with the station's datum sheet. For each calendar year the window touches, the tool computes each constituent's equilibrium argument V₀ at 1 January 00:00 UTC and its node factor f and correction u at 2 July 00:00 UTC, the middle of the year, as NOAA does. Mean longitudes of the Moon, Sun, lunar perigee, lunar node, and solar perigee come from the Meeus polynomials that pytides2 uses; the node quantities I, ξ, ν, ν′, 2ν″, and P come from Schureman's Special Publication 98. Heights are measured from mean sea level and moved to the chosen datum by the sheet's MSL − datum. Highs and lows are where the height's rate of change is zero: the tool steps the rate on a 6-minute grid, and bisects each sign change to under a second.

## Equations

- h(t) = (MSL − datum) + Σ f·H·cos(a·t + V₀ + u − κ), with t in hours from 1 January 00:00 UTC and a the pasted speed in degrees per hour.
- V = Σ cᵢ·xᵢ over x = (T + h − s, s, h, p, N, p₁, 90°), with each constituent's integer coefficients; compound constituents sum their members' V and u with their multipliers and multiply their f values.
- Node factors and u follow Schureman's equations as pytides2 transcribes them (SP 98 equations 65 to 78, 195 to 235), with these as NOAA's predictions require: M1 is Schureman's composite of A16 and A23 (paragraphs 121 and 122, equations 190 to 196), argument T − s + h + p − 90° − ν − Q_a with tan Q_a = sin 2P / (3 cos I / cos² ½I + cos 2P) and f = f(O1)·(1/Q_a) by equation 195, plus a measured +20.5°; 2MK3 = 2 M2 − K1; RHO and 2Q1 are O1-type constituents.
- Local phases convert to Greenwich: κ = κ′ − a × (UTC offset in hours).

## Symbols and units

H and heights in feet or meters as pasted (results in feet by default; any length unit is accepted); κ and V in degrees; a in degrees per hour; t in hours. Times are UTC unless a fixed UTC offset is given.

## Domain

The constituents must be among NOAA's 37 standard ones. Fewer than all 37 are refused unless the reader acknowledges it, and then PARTIAL_CONSTITUENTS lists the missing ones. Windows longer than 31 days, and more than 10,000 heights, are LIMIT_EXCEEDED. Interval from 1 minute to 1 day. A datum the sheet lacks, or a sheet without MSL, is INVALID_INPUT.

## Approximations

The tide is astronomical only: wind, pressure, and river flow are not modeled. NOAA predicts some stations, Anchorage among them, with more than 37 constituents, and subordinate stations with time and height corrections; neither is matched. The +20.5° on M1 is measured, not derived: with Schureman's composite, NOAA's predictions need it at every station and year tried (Seattle 2019 to 2029; San Francisco, Boston, and Honolulu in 2021 and 2026), to within ±0.04°, and its source is not known. Meeus's polynomials stand in for Schureman's Table 1 mean longitudes; the difference is far below NOAA's rounding. Optionally f, u, and V are taken at each instant instead of the year's middle, as XTide does.

## Worked example

- sourcePublisher: NOAA Center for Operational Oceanographic Products and Services
- sourceTitle: Tide predictions for Boston, MA (8443970), from the predictions API
- sourceEdition: 2026 predictions, fetched 2026-10-20
- sourceLocator: 21 October 2026, GMT, MLLW, highs and lows: low 1.515 ft at 05:46, high 8.551 ft at 12:06, low 1.928 ft at 18:05
- independent: yes
- inputs: Boston's 37 harmonic constituents and datum sheet (harcon.json and datums.json in feet), start 2026-10-21, 1 day
- outputs: highs and lows within 2 minutes and 0.01 ft of NOAA's
- tolerance: 0.02 ft and 2 minutes (the spec's)
- verifiedBy: `core/crates/gp-navigation/tests/tide.rs` `predictions_match_noaa_within_0_02_ft_and_2_minutes`
- verifiedOn: 2026-10-20

## Differential tests

- `core/crates/gp-navigation/tests/tide.rs` `predictions_match_noaa_within_0_02_ft_and_2_minutes`: October 2026 at Boston, Seattle, San Francisco, Key West, and Honolulu, from fixtures of NOAA's own constituents, datums, and predictions (`tests/data/tides/`): all 3,720 hourly heights within 0.0085 ft of NOAA's, and all 600 highs and lows matched one for one within 2 minutes and 0.0085 ft.
- `tools/vectors/gen_tide.py predict`: 20 vectors expecting NOAA's published hourly heights (four a day, four days at each station) within 0.02 ft, and two refusals; 22 vectors in `core/vectors/navigation.tide.predict.jsonl`.
- `core/crates/gp-navigation/tests/tide.rs` `node_factors_match_pytides_1990_to_2040`: f and V₀ + u of M2, S2, N2, K1, and O1 for every year from 1990 to 2040 within 1e-6 and 0.001° of pytides2 0.0.5 (`tests/data/tides/pytides_fu.json`, generated from its source).
- During development, a Python reimplementation on pytides2 was checked against full years of NOAA's hourly predictions (Seattle 2019 to 2029), which isolated the M1, 2MK3, RHO, and 2Q1 conventions.

## Invariants

- `core/crates/gp-navigation/tests/tide.rs` `tide_refusals_and_options`: a 90-day window is LIMIT_EXCEEDED naming 31 days; a paste of 20 constituents names the 17 missing and runs only with the acknowledgment, carrying PARTIAL_CONSTITUENTS; without a datum sheet the heights are above MSL and say so; the instant option stays within 0.1 ft of the mid-year one; a table with local phases and its UTC offset gives the same tide as the Greenwich paste.
