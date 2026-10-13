<!-- Derivation note (trust/correctness-program layer A). -->
# Survey deflection angle (`survey.cogo.deflection-angle`)

## Method

The tool treats the backsight as the direction from the occupied point back to the preceding point. Reversing that direction gives the forward extension of the incoming line. The foresight is the direction from the occupied point to the following point. Their clockwise and counterclockwise differences are the possible right and left deflections.

The default selects the smaller angle. The user can instead force the right or left angle when a field procedure or plan specifies the side. A right deflection is positive and a left deflection is negative. A zero angle is labeled straight. An exact 180° angle is labeled reverse because left and right are equivalent.

## Equations

Let B be the backsight north azimuth and F be the foresight north azimuth.

- forward tangent T = wrap(B + 180°)
- right deflection R = wrap(F − T)
- left deflection L = wrap(T − F)
- shortest deflection = R when R ≤ 180°; otherwise L

Here `wrap` reduces an angle to [0°, 360°). The reported magnitude is nonnegative. The signed result is +R for a right turn and −L for a left turn.

## Symbols and units

B, F, and T are north azimuths in degrees clockwise from north. R and L are turn magnitudes in degrees. Input directions may be quadrant bearings, decimal or DMS north azimuths, or gons. Both input directions share one stated reference: grid, geodetic, or assumed.

## Domain

Both directions are required and must use the same north reference. A quadrant bearing must contain an angle from 0° through 90°. A north azimuth must be from 0° through 360°, and a gon value from 0 through 400. The right and left conventions may return an angle greater than 180°; the shortest convention cannot.

## Approximations

The angle differences are exact apart from floating-point representation. Decimal outputs display to eight places, and DMS displays to 0.01 arcsecond. The calculation applies no grid convergence, magnetic declination, or basis-of-bearing rotation.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite implementation check
- sourceEdition: 2026-10-13
- sourceLocator: deflection-angle catalog operation
- independent: no
- inputs: backsight S 30°00′00″ W, foresight N 80°00′00″ E, shortest convention, grid reference
- calculation: the backsight azimuth is 210°, so its forward extension is 30°; the foresight azimuth is 80°; 80° − 30° = 50° clockwise
- outputs: 50°00′00″ right, signed deflection +50°, forward tangent azimuth 30°
- tolerance: exact before display rounding
- verifiedBy: golden vector v001 and the `deflection_angle_conventions` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` independently parses the two directions, reverses the backsight, and evaluates the clockwise and counterclockwise differences.
- `core/vectors/survey.cogo.deflection-angle.jsonl` covers cardinal directions, every quadrant, DMS, gons, wraparound at north, straight and reverse lines, forced long turns, and invalid directions.

## Invariants

- Reversing the backsight twice recovers its original direction.
- Right and left magnitudes sum to 360° unless both are zero.
- A straight continuation has zero deflection even when the azimuths cross north.
- Swapping between right and left changes the sign and replaces a nonzero magnitude θ with 360° − θ.
- Changing only the reference label does not change the numeric result.
