## 1. Geodesics

- [ ] 1.1 Wrap `geographiclib-rs` inverse and direct with the tool contract (including m12, M12, S12); verify the JFK-LHR, nearly-antipodal, antipodal, coincident, and direct scenarios and the GeodTest short set (690 KB) in CI
- [ ] 1.2 Implement Vincenty direct and inverse with iteration limits and Karney delta; verify the non-convergence and comparison scenarios
- [ ] 1.3 Implement spherical methods with radius handling and ellipsoidal delta; verify the haversine-error scenario
- [ ] 1.4 Port GeographicLib Rhumb (direct, inverse, pole handling); verify the rhumb scenarios and a `RhumbSolve` differential test
- [ ] 1.5 Implement waypoints, densification, midpoint, and intermediate points with GPX/GeoJSON output; verify the equal-intervals scenario
- [ ] 1.6 Port GeographicLib Intersect and implement the vertex tool; verify the non-intersecting scenario and an `IntersectTool` differential test
- [ ] 1.7 Support custom ellipsoids with GeodesicExact above |f| > 0.02; verify the Mars and high-flattening scenarios
- [ ] 1.8 Implement the comparison overlay (geodesic, rhumb, great circle); verify the visual fixture

## 2. Route geometry

- [ ] 2.1 Implement ellipsoidal cross-track/along-track with foot-point iteration and the spherical mode; verify both cross-track scenarios and 1 mm agreement with a brute-force reference
- [ ] 2.2 Implement closest point on multi-leg routes; verify the leg-identification scenario
- [ ] 2.3 Implement course intersection and intercept; verify the unreachable-target scenario
- [ ] 2.4 Implement fly-by turn anticipation and standard-rate turns; verify the 90° and standard-rate scenarios
- [ ] 2.5 Implement geodesic range rings with pole and antimeridian handling; verify the pole-enclosing scenario with a GeoJSON validator
- [ ] 2.6 Implement multi-leg route totals with magnetic courses, times, and ETAs; verify the magnetic-course scenario
- [ ] 2.7 Implement time-speed-distance and 2D CPA; verify the solve-for-time and CPA scenarios
- [ ] 2.8 Implement route visualization (legs, turn arcs, offsets, rings); verify the turn-arc fixture

## 3. Line of sight and 3D

- [ ] 3.1 Implement horizon models, mutual visibility, hidden height, dip, and geographic range; verify the 100 m, hidden-height, and dip scenarios
- [ ] 3.2 Implement Fresnel zone and Earth-bulge clearance; verify the 5.8 GHz scenario against the textbook formula
- [ ] 3.3 Implement 3D distance with height-reference reconciliation and look angles; verify the drone and below-horizon scenarios
- [ ] 3.4 Implement vector algebra with convention selection and 3D CPA; verify the navigational-convention and vertical-separation scenarios
- [ ] 3.5 Implement vector-diagram rendering (head-to-tail, rotatable ENU); verify the visual fixture

## 4. Computational geometry

- [ ] 4.1 Implement geodesic and planar area/perimeter with orientation and pole handling; verify the Colorado, polar-cap, and shoelace-warning scenarios and a `Planimeter` differential test
- [ ] 4.2 Implement centroids and representative point; verify the C-shape scenario
- [ ] 4.3 Implement hulls, antimeridian-aware bbox, MBR, and minimum enclosing circle; verify the antimeridian scenario
- [ ] 4.4 Implement geodesic buffers with join/cap styles and measured validation; verify the geofence and collapse scenarios
- [ ] 4.5 Implement RDP and Visvalingam with topology preservation; verify the narrow-inlet scenario
- [ ] 4.6 Implement robust predicates and point-in-polygon (winding and even-odd); verify the boundary and winding scenarios
- [ ] 4.7 Implement validation and make-valid; verify the bow-tie scenario
- [ ] 4.8 Implement boolean operations (planar and geodesic-edge); verify the geofence-overlap scenario and the GEOS differential suite on JTS test geometries
- [ ] 4.9 Implement densify, Delaunay, Voronoi (planar and spherical), and shape distances; verify the Fréchet scenario
- [ ] 4.10 Enforce input limits; verify the oversized-input scenario

## 5. Catalog and docs

- [ ] 5.1 Register all navigation and geometry operations and generated endpoints with practitioner aliases ("as the crow flies", "great circle", "XTE", "loxodrome"); verify catalog counts (46/58 and 38/44)
- [ ] 5.2 Write docs for each tool, including a "haversine vs ellipsoid" explainer page; verify the docs build
- [ ] 5.3 Promote tools meeting the stable bar; verify the verification report
