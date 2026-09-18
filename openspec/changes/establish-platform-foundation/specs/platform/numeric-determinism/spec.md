## Purpose

Makes every geoprims result reproducible across browsers, operating systems, and runtimes by fixing floating-point, transcendental-function, angle-normalization, rounding, and serialization rules.

## ADDED Requirements

### Requirement: IEEE-754 binary64 throughout
All computation SHALL use IEEE-754 binary64 arithmetic in the WebAssembly core. Tools SHALL NOT use host JavaScript `Math` functions for any value that reaches an output. Transcendental functions (sin, cos, tan, atan2, asin, acos, exp, log, pow, sqrt of non-exact cases, hypot, cbrt) SHALL come from a single libm implementation compiled into the core.

#### Scenario: Host Math not used
- **WHEN** the core is audited by the determinism test that replaces the host's `Math` object with a poisoned stub
- **THEN** all golden vectors still pass

### Requirement: No fused multiply-add variability
The core SHALL NOT rely on compiler-chosen fused multiply-add contraction. Where a fused operation is required for accuracy, it SHALL be an explicit software-correct operation producing the same bits on every host.

#### Scenario: FMA-sensitive vector
- **WHEN** the determinism suite runs vectors specifically constructed to differ under FMA contraction
- **THEN** results are bit-identical across all supported hosts

### Requirement: Canonical NaN and signed-zero handling
NaN SHALL never appear in a result (see tool-contract error model). Negative zero SHALL be normalized to positive zero in all outputs. Infinite values SHALL be returned only for fields whose schema explicitly allows them (none by default).

#### Scenario: Negative zero normalized
- **WHEN** a longitude difference computes to -0.0
- **THEN** the serialized output is `0`

### Requirement: Angle normalization rules
Longitudes SHALL be returned in `[-180, 180)` degrees. Azimuths and headings SHALL be returned in `[0, 360)` degrees. Latitudes SHALL be validated to `[-90, 90]` and rejected otherwise (never wrapped). Longitudes supplied outside `[-180, 180)` SHALL be accepted and normalized, and the result SHALL include a warning `INPUT_NORMALIZED` naming the field and the normalized value. Angle normalization SHALL use exact remainder operations, not repeated subtraction.

#### Scenario: Longitude 180 normalized
- **WHEN** a user enters longitude 180
- **THEN** the tool uses -180 and returns warning `INPUT_NORMALIZED` for `/lon`

#### Scenario: Latitude out of range rejected
- **WHEN** a user enters latitude 90.0000001
- **THEN** the tool returns `INVALID_INPUT` for `/lat`

#### Scenario: Large longitude normalized exactly
- **WHEN** a user enters longitude 540.25
- **THEN** the normalized longitude is exactly -179.75

### Requirement: Output serialization is shortest round-trip
Numeric outputs SHALL be serialized by the core using the ECMAScript Number-to-String algorithm (identical to `JSON.stringify` and RFC 8785 number formatting): the shortest decimal string that round-trips to the same binary64 value, with no trailing `.0` on integers. Result objects SHALL be serialized with a fixed key order defined by the output schema. Display rounding for humans SHALL be a presentation concern applied after serialization and SHALL never alter the machine-readable value returned to agents or exported to files.

#### Scenario: Display vs machine value
- **WHEN** a distance of 12345.678901234567 m is computed
- **THEN** the web UI may display `12,345.679 m` while the copied JSON, MCP result, and CSV export contain `12345.678901234567`

### Requirement: Declared significant precision
Every numeric output SHALL declare a `x-display-precision` rule (decimal places or significant figures, possibly unit-dependent) chosen so that displayed digits do not imply more accuracy than the tool's stated accuracy.

#### Scenario: Honest display precision
- **WHEN** WMM2025 declination is computed with a stated global uncertainty on the order of tenths of a degree
- **THEN** the default display shows at most 2 decimal places of degrees, while the machine value retains full precision

### Requirement: Iterative algorithms have explicit convergence contracts
Every iterative algorithm SHALL declare its convergence tolerance and maximum iteration count. When the maximum is reached without convergence the tool SHALL return `DID_NOT_CONVERGE` (or fall back to a documented alternate algorithm and emit a warning naming it), never an unconverged value presented as a result.

#### Scenario: Vincenty near-antipodal
- **WHEN** the Vincenty inverse tool is asked for the distance between (0°, 0°) and (0.5°, 179.7°)
- **THEN** the tool either returns `DID_NOT_CONVERGE` with a hint to `navigation.geodesic.inverse`, or returns the Karney result with warning `ALGORITHM_FALLBACK`, per the tool's declared behavior
