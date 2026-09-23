<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Aerial mapping window (`time.sun.mapping-window`)

## Method

Photogrammetry wants the sun high. Low sun means long shadows, which hide detail and confuse matching, so vendor guidance is generally to fly with the sun above about 30°. This tool answers when that holds on a given day at a given place.

The sun's elevation over a day is a single smooth arc with one maximum, so finding when it crosses a threshold is a bracketed root-find, not a search. The day's highest point is the solar transit; the lowest is half a day away. If the peak is below the threshold there is no window at all; if the trough is above it, the window is the whole day. Otherwise one crossing lies between the trough and the peak on each side, and bisection finds both.

The threshold is compared against the **geometric** elevation — where the sun actually is — because a shadow is cast by geometry and not by what the atmosphere does to the light on its way to the eye. The elevation *reported* is the apparent one, which is what an observer or a camera sees, and the two differ by refraction: about 0.02° at 30° up.

The day's path is the same sun sampled every 20 minutes of local clock time, so the window can be read against the arc it sits on.

## Equations

- Geometric elevation e(t) from the NREL SPA, without refraction.
- Window = the two t either side of transit where e(t) = threshold, by bisection.
- Reported peak and path elevations = apparent elevation, geometric plus refraction.
- Duration = window end − window start.

## Symbols and units

`lat`, `lon`, `date` and `offset` fix the local day; `threshold` is the minimum elevation, default 30°. Out come `window` and its `window_start` and `window_end` in local time and Zulu, the `duration`, the day's `max_elevation`, and the `path`.

## Domain

Any place and date. Thresholds from −18° (astronomical twilight) to 89°. At high latitudes the answer is often "at no time that day" in winter or "all day" for a low threshold in summer, and both are reported as such rather than as an empty window.

## Approximations

Times are good to about a minute, which is the accuracy the crossing solver and the SPA together support and is far finer than the decision needs. Elevations are the SPA's, to about 0.0003°, with the standard sea-level refraction model applied for the apparent value; refraction varies with pressure and temperature by a few hundredths of a degree, which moves a crossing time by seconds.

## Worked example

- sourcePublisher: National Renewable Energy Laboratory; pvlib community
- sourceTitle: Solar Position Algorithm for Solar Radiation Applications (NREL/TP-560-34302); pvlib-python
- sourceEdition: Revised January 2008; pvlib 0.13.0
- sourceLocator: SPA sections 3.1 to 3.15; pvlib `solarposition.spa_python` at sea level, 101,325 Pa, 12 °C
- independent: yes
- inputs: 21 place-and-date cases, including Denver at both solstices and the equinox, London in June, Sydney in January, Singapore at equinox with a 70° threshold that the sun barely clears, and thresholds of 20°, 45° and 60°
- outputs: the window start and end to the minute, and the day's highest sun
- tolerance: the times exact to the minute, 0.001° on the peak
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The reference finds the crossings the same way the tool does — bisection on the geometric elevation — but on pvlib's SPA rather than the core's, and its peak is found by a ternary search rather than taken at transit. Building it turned up a real defect: the peak and the path were coming from the NOAA low-precision series while the times were solved on the SPA, so the tool reported a maximum up to ten arcseconds **above** the highest elevation `time.sun.position` gives for the same instant. Both now run on the same sun. The fix is recorded in the repository history.

## Differential tests

- `tools/vectors/gen_sun_pvlib.py`: 16 of the 21 vectors, crossings by bisection on pvlib's SPA
- `core/crates/gp-time/tests/spa_parity.rs`: the sun itself against pvlib at 250 points over 1990–2060
- `core/vectors/time.sun.mapping-window.jsonl`: 21 vectors from the equator to 51° north and 34° south

## Invariants

- `core/crates/gp-time/tests/time.rs` `mapping_window_invariants`: the reported peak agrees with `time.sun.position` at the same place and day to within an arcsecond, which is the check that caught the two tools running on different suns; the window is symmetric about the peak to within a minute or two, as a smooth arc crossing a level must be; raising the threshold shortens the window and never lengthens it; a threshold above the day's peak gives no window at all rather than an empty one; the peak is at least the threshold whenever a window exists; and the apparent peak exceeds the geometric one, since refraction lifts the sun and never lowers it
