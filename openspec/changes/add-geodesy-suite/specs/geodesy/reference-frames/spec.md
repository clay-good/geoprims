## Purpose

Provides the ellipsoid geometry and frame conversions (geodetic, Earth-centered Earth-fixed, and local tangent planes) that every 3D spatial and flight calculation depends on.

## ADDED Requirements

### Requirement: Ellipsoid catalog and parameters
The domain SHALL provide a catalog of ellipsoids at least including WGS 84, GRS 80, Clarke 1866, Airy 1830, International 1924, Bessel 1841, Krassovsky 1940, and a sphere of user-chosen radius, and a tool returning derived parameters: semi-minor axis, flattening, inverse flattening, first and second eccentricity squared, third flattening, mean radius R1, authalic radius, and volumetric radius. Custom ellipsoids SHALL be accepted as (a, 1/f) or (a, b).

#### Scenario: WGS 84 parameters
- **WHEN** WGS 84 parameters are requested
- **THEN** a = 6,378,137 m exactly, 1/f = 298.257223563 exactly, and b = 6,356,752.314245… m

### Requirement: Radii of curvature and arc lengths
Tools SHALL compute, at a given latitude: meridional radius M, prime-vertical radius N, Gaussian mean radius √(MN), radius in a given azimuth (Euler), length of one degree of latitude and longitude, and meridian arc length from the equator (and between two latitudes) using a series accurate to 1 mm or better.

#### Scenario: Degree lengths at 45°
- **WHEN** the length of one degree at 45° N on WGS 84 is requested
- **THEN** one degree of latitude (the meridian arc from 44.5° to 45.5°) ≈ 111,131.78 m and one degree of longitude ≈ 78,846.84 m (±0.01 m)

### Requirement: Auxiliary latitudes
Tools SHALL convert geodetic latitude to and from geocentric, parametric (reduced), rectifying, authalic, conformal, and isometric latitude, with round-trip error below 1e-12 degrees.

#### Scenario: Geocentric latitude
- **WHEN** geodetic latitude 45° is converted to geocentric on WGS 84
- **THEN** the result is ≈ 44.8076° and converting back returns 45° within 1e-12°

### Requirement: Geodetic ↔ ECEF
Tools SHALL convert geodetic (lat, lon, ellipsoidal height) to ECEF (X, Y, Z) and back. The inverse SHALL use a closed-form or non-iterative method accurate to 1e-9 m in position for heights from -10 km to +100,000 km, including points at the poles and at the Earth's center.

#### Scenario: Forward conversion
- **WHEN** (40.446111°, -79.982222°, 300 m) on WGS 84 is converted to ECEF
- **THEN** X ≈ 845,580.010 m, Y ≈ -4,786,836.717 m, Z ≈ 4,116,002.385 m (±1 mm)

#### Scenario: Pole
- **WHEN** ECEF (0, 0, 6,356,752.314245 m) is converted to geodetic
- **THEN** latitude is 90°, height is 0 m (±1e-9 m), and longitude is returned as 0 with warning `LONGITUDE_UNDEFINED`

### Requirement: Local tangent plane frames
Tools SHALL convert between ECEF or geodetic coordinates and local East-North-Up (ENU), North-East-Down (NED), and Azimuth-Elevation-Range (AER) frames relative to a stated origin, in both directions, and SHALL return the rotation matrix on request.

#### Scenario: AER of an overhead target
- **WHEN** a target 1,000 m directly above the origin is converted to AER
- **THEN** elevation is 90°, range is 1,000 m, and azimuth is returned as 0 with warning `AZIMUTH_UNDEFINED`

#### Scenario: ENU round trip
- **WHEN** 10,000 random points within 1,000 km of random origins are converted ENU → geodetic → ENU
- **THEN** every point returns within 1e-8 m

### Requirement: Frame visualization
Frame tools SHALL visualize the ellipsoid, the origin, and the ENU/NED axes on the 3D globe, and AER results as a sky-plot vector diagram.

#### Scenario: Sky plot
- **WHEN** an AER result is shown
- **THEN** the vector diagram shows a polar sky plot with the target's azimuth and elevation
