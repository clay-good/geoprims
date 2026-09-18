## Purpose

Defines, once, every error and warning code a geoprims tool can return, how severe each is, and whether a condition is an error (no result) or a warning (a result with a caveat), so tools behave consistently and agents can act on codes reliably.

## ADDED Requirements

### Requirement: Error codes
The closed error enumeration SHALL be:

| Code | Meaning |
|---|---|
| `INVALID_INPUT` | Malformed or impossible input |
| `OUT_OF_DOMAIN` | Valid input outside the method's or model's domain or validity |
| `UNIT_MISMATCH` | Wrong quantity type or ambiguous unit |
| `DID_NOT_CONVERGE` | Iteration limit reached |
| `DEGENERATE_GEOMETRY` | Undefined or degenerate geometry (e.g. Earth's center, self-intersecting ring, H3 pentagon path failure) |
| `NO_SOLUTION` | Well-formed problem with no answer (e.g. crosswind above airspeed, overhead DME) |
| `ASSET_UNAVAILABLE` | Required data not present |
| `ASSET_INTEGRITY` | Data failed its digest check |
| `LIMIT_EXCEEDED` | Size, time, or memory limit |
| `UNSUPPORTED` | Unknown or unavailable tool |
| `INTERNAL` | Defect |

A condition SHALL be an error when no meaningful number can be returned, and a warning when a number is returned with a caveat.

#### Scenario: DST gap classification
- **WHEN** a nonexistent local time (DST gap) is converted
- **THEN** the result is `INVALID_INPUT`, because the input time does not exist

### Requirement: Warning severities
Every warning SHALL have one severity:
- **`caution`:** an operational or safety-relevant limit is exceeded. Shown first, with an icon, and never collapsible.
- **`accuracy`:** the result is less accurate or rests on an assumption.
- **`info`:** a normalization or convention note.

The seed registry SHALL contain at least:

| Severity | Codes |
|---|---|
| **caution** | `COMPASS_BLACKOUT_ZONE`, `COMPASS_CAUTION_ZONE`, `LOAD_LIMIT_EXCEEDED`, `OUTSIDE_CG_ENVELOPE`, `BELOW_TERRAIN`, `CANNOT_RETURN_INTO_WIND`, `ABOVE_MAX_HOLDING_SPEED`, `WAYPOINT_OUTSIDE_GEOFENCE`, `C_RATE_EXCEEDED`, `OBSERVATION_OLD`, `TRIGGER_TOO_FAST`, `OVERLAP_BELOW_TARGET`, `FLY_OVER_RECOMMENDED`, `INSUFFICIENT_CHECKPOINTS`, `ADJUSTMENT_TEST_FAILED`, `RESECTION_UNSTABLE` |
| **accuracy** | `ACCURACY_DEGRADED`, `ALGORITHM_FALLBACK`, `MODEL_EXTRAPOLATED`, `NON_OFFICIAL_DATUM`, `LOW_ACCURACY_TRANSFORM`, `DEFORMATION_ZONE`, `FRAME_MISMATCH`, `REALIZATION_ASSUMED`, `ORTHOMETRIC_AS_ELLIPSOIDAL`, `CALIBRATION_ASSUMED`, `ISA_TEMPERATURE_ASSUMED`, `HEURISTIC_DERATING`, `PLANAR_ON_GEOGRAPHIC`, `PENTAGON_DISTORTION`, `UT1_APPROXIMATED`, `ALMANAC_OLD`, `LEAP_SECOND_TABLE_EXPIRED`, `NO_REDUNDANCY`, `SCALE_SUSPECT`, `SUSPECT_SCALING`, `SUSPECT_VALUE`, `EQUIVALENT_FOCAL_LENGTH`, `STATION_VARIATION_DIFFERS`, `RUNWAY_HEADING_APPROXIMATE`, `PRISMOIDAL_MIDDLE_AREA_AVERAGED`, `CURVE_CALL_INCOMPLETE`, `NON_METRIC_CALL`, `GRID_MISMATCH`, `EXPERIMENTAL_TOOL` |
| **info** | `INPUT_NORMALIZED`, `UNIT_ASSUMED`, `AMBIGUOUS_INPUT`, `LEGACY_UNIT`, `NONSTANDARD_ZONE`, `OUTSIDE_ZONE_EXTENT`, `BAND_ADJUSTED`, `WEB_MERCATOR_CLAMPED`, `CROSSES_ANTIMERIDIAN`, `AZIMUTH_UNDEFINED`, `AZIMUTH_NOT_UNIQUE`, `LONGITUDE_UNDEFINED`, `DECLINATION_POLE_CONVENTION`, `RHUMB_REACHES_POLE`, `FOOT_OUTSIDE_SEGMENT`, `CENTROID_OUTSIDE`, `BUFFER_COLLAPSED`, `BELOW_HORIZON`, `DIVERGING`, `PERFECT_CLOSURE` |

Each registry entry SHALL carry a plain-language message template, the tools that may emit it, and, where useful, a hint to a related tool.

#### Scenario: Caution first
- **WHEN** a result carries `OUTSIDE_CG_ENVELOPE` and `INPUT_NORMALIZED`
- **THEN** the answer card shows the caution first with its icon, and the info note last

#### Scenario: Registry completeness
- **WHEN** the build scans core code and manifests for emitted codes
- **THEN** every code is in the registry with a severity, or the build fails
