## Purpose

Reduces a timed solar or Polaris observation to the astronomic azimuth of a survey line by the hour-angle method, with every correction shown and the difference from geodetic azimuth stated.

## ADDED Requirements

### Requirement: Hour-angle reduction
Given the observer's geodetic position, the UT1 instant of each pointing (from a stated time source and offset, with DUT1 optional and `UT1_APPROXIMATED` without it), the horizontal angle from the reference mark to the body, and for the sun the limb observed, the tool SHALL compute the body's azimuth Z = atan2(−sin t, cos φ tan δ − sin φ cos t) from north (t = LHA from the computed GHA), apply the semidiameter correction SD / cos h for a limb, and return the azimuth of the line from each pointing, their mean, and their spread (arcseconds). It SHALL warn `ASTRONOMIC_AZIMUTH` that the result is astronomic and differs from geodetic azimuth by the Laplace correction, which the reader may enter.

#### Scenario: Polaris
- **WHEN** Bowditch §1504's Polaris observation (2016-02-23 04:21:15 UT at 29°31.0'N 074°30.0'W) is reduced
- **THEN** Polaris's azimuth is 359.2° (±0.1°), and with a horizontal angle of 45°00'00" from the mark clockwise to Polaris the line's azimuth is 314.2° (±0.1°)

#### Scenario: Sun limb correction
- **WHEN** the sun's trailing limb is observed at altitude 30° with semidiameter 16.1'
- **THEN** the correction applied is 16.1' / cos 30° = 18.59' (±0.01'), shown as SD / cos h, and its sign follows the limb

#### Scenario: Laplace correction entered
- **WHEN** the reader enters a Laplace correction of −2.4"
- **THEN** the geodetic azimuth is reported beside the astronomic one and the warning is replaced by the stated correction

### Requirement: Observation set statistics
With two or more pointings, the tool SHALL report each pointing's azimuth, the mean, the standard deviation, and SHALL flag a pointing more than 3σ from the mean with `SUSPECT_VALUE`.

#### Scenario: Blunder
- **WHEN** one of six pointings is 2' from the others
- **THEN** it carries `SUSPECT_VALUE` and the mean without it is also shown

### Requirement: Professional-use notice
Results SHALL display "Astronomic azimuth from your observation times and angles. Control surveys follow your jurisdiction's standards and a licensed surveyor's judgment."

#### Scenario: Notice present
- **WHEN** any azimuth result is shown
- **THEN** the notice is visible with the result
