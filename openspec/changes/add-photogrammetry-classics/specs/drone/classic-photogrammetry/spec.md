## Purpose

Computes the single-photo and stereo-pair relations of classic photogrammetry (scale, relief displacement, parallax heights, base-height ratio) with the textbook examples reproduced and the geometry drawn.

## ADDED Requirements

### Requirement: Photo scale
The tool SHALL compute scale S = f / (H − h) as a ratio 1:n (and as GSD when a pixel pitch is given, by the same function as the GSD tool), the flying height for a scale, and ground distance from a photo distance (D = d (H − h) / f). Heights SHALL carry their reference (above the point, or above datum with the point's elevation).

#### Scenario: Flying height for a GSD
- **WHEN** a 100 mm lens with a 10 µm pixel must give 1 ft GSD
- **THEN** the flying height is 10,000 ft above ground (±1 ft), as the Penn State lesson shows

#### Scenario: Round trip
- **WHEN** a photo distance is converted to ground and back
- **THEN** the original is recovered within 1e-9 of the unit

### Requirement: Relief displacement
The tool SHALL solve d = r h / H for any one unknown (displacement d, radial distance r, object height h, flying height H above the object's base), with r measured from the principal point and the direction note (outward for points above datum).

#### Scenario: Tower height
- **WHEN** H = 535 m above datum, base elevation 259 m, d = 54.1 mm, r = 121.7 mm
- **THEN** the tower height is 122.7 m (±0.1 m), as the textbook example

### Requirement: Height from parallax
The tool SHALL compute height from absolute parallax (h = H − f B / p) and from a parallax difference (h_A = h_C + Δp (H − h_C) / p_a), with parallax in the same unit as the focal length, and SHALL offer ground X and Y from x B / p and y B / p.

#### Scenario: Tree height
- **WHEN** Δp = 1.3 mm, H = 915 m, p_a = 88.2 mm, and the control point is at ground
- **THEN** the tree height is 13.3 m (±0.1 m), as the textbook example

#### Scenario: Zero parallax difference
- **WHEN** Δp = 0
- **THEN** the height equals the control point's

### Requirement: Stereo base and vertical precision
The tool SHALL compute the airbase B = (1 − endlap) × H × (sensor length along track / f), the base-height ratio B/H, and the vertical precision implied by a parallax measurement precision, σ_h = (H / B)(H / f) σ_p, labeled "derived from the parallax equation" and "geometric precision of one measurement, not product accuracy," with the ASPRS checkpoint rule cited.

#### Scenario: Airbase at 60% endlap
- **WHEN** the Penn State plan (10,000 ft AGL, 60% endlap, the lesson's format) is evaluated
- **THEN** the airbase is 2,800 ft (±1 ft)

#### Scenario: More overlap, shorter base
- **WHEN** endlap rises from 60% to 80%
- **THEN** B/H halves and σ_h doubles, and the sentence says so

### Requirement: Drawn geometry
Each tool SHALL draw its geometry from the result: the camera and ground with the scale triangle; the object, its base and top rays, and the displacement on the photo; the two camera stations and the parallax rays; the two stations with the base and overlap.

#### Scenario: Diagram from the result
- **WHEN** the relief-displacement example runs
- **THEN** the diagram shows the tower, the two rays, and the 54.1 mm displacement labeled, from the core's values
