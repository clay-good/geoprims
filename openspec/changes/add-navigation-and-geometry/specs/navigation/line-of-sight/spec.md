## Purpose

Answers "how far can I see, or be seen, or talk by radio" over a curved, refracting Earth without terrain. Constants and assumptions are explicit, because published rules of thumb disagree.

## ADDED Requirements

### Requirement: Horizon distance with explicit model
Given observer height above the surface, the tool SHALL compute horizon distance for three models and state the model used: geometric (no refraction, d = √(2Rh + h²)), optical with refraction coefficient k (default 0.13, effective radius R/(1 − k)), and radio (4/3 Earth, k = 0.25). R SHALL default to 6,371,000 m and be adjustable, with an option to use the ellipsoid's radius of curvature at the observer's latitude and azimuth. It SHALL return both the straight-line (slant) distance and the surface (arc) distance.

#### Scenario: 100 m observer
- **WHEN** horizon distance is requested for h = 100 m with R = 6,371,000 m
- **THEN** the geometric distance ≈ 35.70 km, the optical (k = 0.13) ≈ 38.27 km, and the radio (4/3) ≈ 41.22 km

#### Scenario: Rule-of-thumb comparison
- **WHEN** the result is displayed
- **THEN** it also shows the common approximations (3.57√h km, 1.17√h NM visual, 1.23√h NM radio) with their error for this height

### Requirement: Mutual visibility of two elevated points
Given two heights, the tool SHALL compute the maximum distance at which they are mutually visible (sum of the horizon distances) and, given an actual separation, whether they are visible, the clearance at the midpoint, and the hidden height of the far target.

#### Scenario: Hidden height
- **WHEN** an observer at 2 m looks at a target 30 km away with k = 0.13
- **THEN** the observer's horizon is ≈ 5.41 km and the hidden height of the target (the part below the horizon) is ≈ 41.3 m

### Requirement: Dip of the horizon and geographic range
A tool SHALL compute the dip of the visible horizon for an observer height (with refraction), and the geographic range of a light or object of given height (nautical use).

#### Scenario: Dip at 10 m
- **WHEN** dip is requested for h = 10 m with standard refraction
- **THEN** the result is in arcminutes, and the approximation 1.76′√h is also shown with its difference

### Requirement: Fresnel zone and radio clearance
A tool SHALL compute the first Fresnel zone radius at any point along a link (from frequency and distances) and the 60% clearance requirement, combined with Earth bulge at that point for a given k.

#### Scenario: 5.8 GHz drone link
- **WHEN** a 5.8 GHz link of 10 km is evaluated at the midpoint
- **THEN** the first Fresnel radius and Earth bulge (k = 4/3) are reported and summed as required clearance

### Requirement: Terrain is out of scope here and says so
Line-of-sight results SHALL state that terrain and obstacles are not considered and link to the terrain line-of-sight tool.

#### Scenario: Terrain notice
- **WHEN** any line-of-sight result is displayed
- **THEN** it includes the notice and a link to `raster.terrain.line-of-sight`
