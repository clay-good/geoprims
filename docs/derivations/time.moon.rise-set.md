<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Moonrise, moonset, and transit (`time.moon.rise-set`)

## Method

Moonrise is harder than sunrise for two reasons: the moon is close enough that the observer's own position on the Earth moves it by nearly a degree, and it moves fast enough that its place changes while the horizon crossing is being found.

USNO settles the first by defining the event geocentrically. From its *Rise, Set, and Twilight Definitions*, moonrise and moonset occur when the geometric zenith distance of the moon's **centre**, measured from the centre of the Earth, is

    90.5666 degrees + the moon's apparent angular radius - its horizontal parallax

which is 90°34′ of refraction-corrected horizon plus the semidiameter less the parallax. Under normal conditions at sea level the upper limb is then tangent to a level horizon. This tool uses exactly that criterion, so its times are the times in USNO's own tables.

The second is settled by sampling. The moon's apparent place comes from the same `gp-astro` series as the celestial almanac, which is far too costly to run at every step of a solver, so the tool samples the apparent right ascension, declination, and distance every two hours across the local day and interpolates between samples. The moon moves about half a degree an hour; cubic interpolation over a two-hour span is good to a thousandth of an arcsecond, which is nothing beside the minute the times are wanted to.

1. The local day is turned into a window of Universal Time using the zone's offset at local noon.
2. The window is sampled every two hours, with a sample either side so the interpolation never extrapolates.
3. The rise-and-set criterion is evaluated every ten minutes across the window and every sign change is bisected. Every crossing is kept, not just the first: the moon rises about 50 minutes later each day, so a local date can hold two of one kind or none at all.
4. Upper transit is found the same way, from the zeros of the local hour angle where it is increasing.
5. When a date has no rise or no set, the window is widened by a day and a half either side to find the ones on each side of it, and both are named in the result.

Greenwich apparent sidereal time is the cheap rotation polynomials of IERS 2010 plus an equation of the equinoxes held fixed across the window. That term changes by under a thousandth of an arcsecond in a day, so holding it fixed costs nothing and saves a full nutation series at every step of the solver.

## Equations

- The event: h = HP − SD − 34′, where h is the geocentric altitude of the moon's centre, HP = asin(a / d) the horizontal parallax, SD = asin(0.2725076 sin HP) the semidiameter, and a the Earth's equatorial radius. With HP about 57′ and SD about 15.5′ this puts the centre about 7.5′ **above** the geocentric horizon at the event.
- The altitude solved for: sin h = sin L sin d + cos L cos d cos H, with H = GAST + λ − α.
- Upper transit: H = 0, taken from the zeros of H wrapped into ±180° where it increases.
- GAST = GMST(UT1) + the equation of the equinoxes at the middle of the window.

## Symbols and units

L is the latitude, λ the longitude, α and d the moon's apparent right ascension and declination, H the local hour angle, HP its horizontal parallax, SD its semidiameter, and h the geocentric altitude of its centre. Times are reported in local time and Zulu, each dated; altitudes and azimuths in degrees. Trigonometry runs in radians.

## Domain

Any place; a local date; a zone as a fixed offset or a tzdb name. Inside the polar circles the moon can stay above or below the horizon for the whole day, which carries `POLAR_MOON` and the state `always-up` or `always-down` rather than a made-up time. A date whose rise or set the daily retardation skips carries `POLAR_MOON` too, with the state `no-moonrise` or `no-moonset` and the times either side.

## Approximations

- **The sampled ephemeris.** Two-hourly samples with cubic interpolation, which is a thousandth of an arcsecond over the span. The error this contributes to a time is far below a second.
- **The equation of the equinoxes held fixed** across the window, which is under a thousandth of an arcsecond of sidereal time.
- **The zone offset at local noon** sets the window. On the two days a year a zone changes its clocks, the window can be an hour out at one end; each event is still converted to local time with the offset in force at that instant.
- **A level horizon at sea level.** No height of eye, no terrain, and the 34′ is the standard horizon refraction, which the weather moves by minutes of arc. This is the same assumption USNO's tables make.
- **The geocentric semidiameter** stands in for USNO's "apparent angular radius". The two differ by the augmentation, which is about 0.3′ at the zenith and nil at the horizon, so at a rise or set they agree.

## Worked example

- sourcePublisher: US Naval Observatory, Astronomical Applications Department
- sourceTitle: USNO Astronomical Applications API, rstt/oneday
- sourceEdition: API 4.0.1
- sourceLocator: Moonrise, moonset, and upper transit for 2026-06-21 at 39.7392, −104.9903 with tz=0
- independent: yes
- inputs: latitude 39.7392, longitude −104.9903, local date 2026-06-21, zone America/Denver
- outputs: USNO gives set 06:35Z, rise 18:59Z, and an upper transit at 00:20Z on that UTC day; the tool returns the same set and rise to the minute. Its transit is the one inside the Denver local day, 2026-06-22 01:03Z, because the local day begins at 06:00Z.
- tolerance: 1 minute, which is the precision USNO prints
- verifiedBy: `core/crates/gp-time/tests/moon.rs` `rise_set_matches_usno`, and golden vectors v001 to v026
- verifiedOn: 2026-10-28

## Differential tests

- `core/crates/gp-time/tests/moon.rs` `rise_set_matches_usno` runs the tool against USNO's rstt/oneday at 500 places and UTC dates from 1995 to 2045, with the latitudes beyond 60° deliberately over-sampled. All 1,361 rise, set, and transit times agree within one minute, the 29 days the moon never rises or sets are reported as such, and the 32 days the retardation leaves without a rise or a set are caught as `no-moonrise` or `no-moonset`.
  - One event in the sample falls within a minute of midnight, where USNO dates it to the neighbouring day; the test accepts either dating for an event that close to the boundary.
  - 15 of about 450 rows are a minute apart from USNO. They cluster at high latitudes, where the moon crosses the horizon at a shallow angle and a second of arc is many seconds of time. A vector string carries no tolerance, so those rows are checked by this test rather than written as vectors; `tools/vectors/gen_moon.py` lists them.
- `tools/vectors/gen_moon.py time.moon.rise-set` builds the 26 vectors in `core/vectors/time.moon.rise-set.jsonl`: USNO's own rise and set strings for the rows where the printed minute agrees exactly, its continuously-above and continuously-below days, and its skipped-event days.

## Invariants

- `core/crates/gp-time/tests/moon.rs` `a_skipped_moonset_says_when_the_neighbours_are` checks that a date USNO gives no moonset names the sets either side of it and gives the retardation as the reason, over the 11 such days in the sample.
- `core/crates/gp-time/tests/moon.rs` `rise_set_matches_usno` also checks that a day USNO calls continuously above or below the horizon is never answered with a time.
