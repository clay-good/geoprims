<!-- Derivation note (trust/correctness-program layer A). -->
# Two-face zenith mean and index error (`survey.reduction.two-face-mean`)

## Method

A zenith angle read with the telescope direct (face left) and again reversed (face right) to the same target carries the vertical index error with the same sign in both readings. Face right reads 360° minus the zenith, so averaging the face-left reading with 360° minus the face-right reading cancels the index error, and half the excess of their sum over 360° is the index error itself. The slope reduction, `survey.reduction.slope`, uses the same function when it is given both faces.

## Equations

- Mean zenith Z = (FL + 360° − FR) / 2
- Index error i = (FL + FR − 360°) / 2, positive when the circle reads high
- Vertical angle = 90° − Z

## Symbols and units

FL face-left zenith reading, FR face-right zenith reading, both in degrees or degrees, minutes, and seconds. The mean is reported in DMS to 0.1 arcsecond and in decimal degrees, the index error in arcseconds, and the vertical angle in decimal degrees, positive above the horizon.

## Domain

The face-left reading is from 0° to 180° and the face-right reading from 180° to 360°. Readings entered in the wrong face are refused rather than swapped.

## Approximations

None in the arithmetic. The mean is free of index error only if the instrument was level and both readings were to the same point; pointing error and refraction remain.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/instrument-reductions, scenario "Two-face observation"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: face left 85°00'10", face right 274°59'40"
- outputs: mean zenith 85°00'15", index error −5" (FL + FR = 359°59'50")
- tolerance: exact to 0.1 arcsecond
- verifiedBy: golden vector v001 and the `two_face_mean_matches_the_slope_reduction` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `two_face_mean`: 19 random pairs built in whole arcseconds from a zenith between 35° and 145°, an index error up to 90 arcseconds, and a pointing difference up to 4 arcseconds, reduced by integer arithmetic.
- `core/vectors/survey.reduction.two-face-mean.jsonl`: those vectors, the spec scenario, and three refusal cases.
- The integration suite requires the same mean and index error as `survey.reduction.slope` for the same readings.

## Invariants

- Mean + index error = face-left reading.
- Adding the same error to both faces changes the index error by that amount and leaves the mean unchanged.
- FL + FR = 360° gives a zero index error and a mean equal to the face-left reading.
