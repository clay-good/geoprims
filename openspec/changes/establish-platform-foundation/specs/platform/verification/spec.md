## Purpose

Proves that every geoprims tool is correct against authoritative references, stays correct across releases and hosts, and discloses its accuracy honestly, so that surveyors, pilots, and engineers can rely on the numbers.

## ADDED Requirements

### Requirement: Golden vectors from authoritative sources
Every tool SHALL ship at least 5 golden test vectors, and every stable tool SHALL ship at least 20, drawn from authoritative sources where they exist (for example: GeographicLib `GeodTest.dat` for geodesics, NGS NCAT/VDatum outputs for datum and State Plane, NOAA/NCEI WMM test values for magnetics, NGA MGRS test points, ICAO Doc 7488 atmosphere tables, H3 and S2 reference-implementation outputs, published textbook examples for surveying). Each vector SHALL record its source, the source's version or date, and its tolerance per output field with unit.

#### Scenario: Vector provenance required
- **WHEN** a vector lacks a `source` or `tolerance`
- **THEN** the verification build fails naming the tool and vector index

#### Scenario: WMM official test values
- **WHEN** the magnetic tool suite runs
- **THEN** every official WMM2025 test value published by NCEI matches within the tolerance stated in the WMM2025 test-value file

### Requirement: Edge-case vectors are mandatory per domain
Each domain SHALL maintain an edge-case vector set covering at least: poles (±90°), the antimeridian (±180°), the equator, coincident points, antipodal and nearly antipodal points, zone and grid boundaries (UTM zone edges, the Norway and Svalbard exceptions, MGRS band X, H3 pentagons, S2 face edges), zero and negative magnitudes where physically meaningful, and the extremes of each tool's declared domain.

#### Scenario: Pentagon coverage
- **WHEN** the indexing suite runs
- **THEN** it includes, for every H3 resolution 0–15, at least one vector at a pentagon cell and its neighbors

### Requirement: Cross-implementation differential testing
For every tool family where an independent reference implementation exists (e.g. GeographicLib C++, PROJ, the H3 C library, the S2 C++ library, NOAA WMM C code), CI SHALL run a differential test on at least 10,000 randomized and edge-biased inputs and SHALL fail if any output differs beyond the tool's declared tolerance.

#### Scenario: Geodesic differential test
- **WHEN** 10,000 random point pairs are run through `navigation.geodesic.inverse` and GeographicLib C++ `GeodSolve -i`
- **THEN** all distances agree within 1e-9 m relative plus 15 nm absolute and all azimuths within 1e-9 degrees, else CI fails with the failing pairs

### Requirement: Property-based invariants
Every invertible tool pair SHALL have round-trip property tests (for example forward then inverse projection returns the original point within declared tolerance). Tool families SHALL test documented invariants (e.g. triangle inequality for geodesic distances, polygon area sign reversal under reversed winding, H3 parent contains child center, unit conversion A→B→A identity).

#### Scenario: UTM round trip
- **WHEN** 100,000 random points within the UTM domain are converted forward and back
- **THEN** every point returns within 1 nm (1e-9 m) of its origin

### Requirement: Cross-host determinism suite
The full vector suite SHALL run in each release candidate on current stable Chromium, Firefox, and WebKit (via headless automation) and on Node.js active LTS, and SHALL compare serialized outputs byte-for-byte across hosts.

#### Scenario: Host divergence
- **WHEN** WebKit produces a result differing in the last bit from Node.js for any vector
- **THEN** CI fails and reports the tool, vector, and both values

### Requirement: Published accuracy statements
Each tool page and manifest SHALL state accuracy in plain language with the conditions under which it holds, and the site SHALL publish a verification report per release listing, per tool, the vector count, the sources, the maximum observed error versus reference, and the declared tolerance.

#### Scenario: Verification report published
- **WHEN** a release is deployed
- **THEN** `/verification/<version>` lists every tool with its vector count, sources, maximum observed error, and tolerance

### Requirement: Reference device profile for performance
Performance budgets SHALL be measured on a documented reference profile: a mid-tier Android device class (or equivalent CPU throttle of 4× on a desktop CI runner) and Node.js LTS on a standard CI runner. Benchmarks SHALL report p50 and p95 over at least 1,000 invocations after warm-up.

#### Scenario: Benchmark output
- **WHEN** the benchmark job completes
- **THEN** it emits a table of p50/p95 per tool and a diff against the previous release

### Requirement: Regression history
Test vectors SHALL be immutable once published: a vector may be superseded (with a recorded reason, such as an upstream model correction) but not silently edited. Changing a tool's numeric behavior beyond tolerance SHALL require a tool version bump and a changelog entry.

#### Scenario: Silent vector edit blocked
- **WHEN** a pull request modifies an existing vector's expected value without a supersession record
- **THEN** the verification lint fails
