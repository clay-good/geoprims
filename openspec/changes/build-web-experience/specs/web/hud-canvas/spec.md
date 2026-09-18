## Purpose

Renders every geoprims calculation on a live canvas (2D map, 3D globe, or engineering vector diagram) so users see the geometry of their inputs and results and can catch mistakes at a glance.

## ADDED Requirements

### Requirement: Canvas modes
The canvas SHALL support three modes: **2D map** (projected view, default Web Mercator with a selectable equirectangular or polar azimuthal view), **3D globe** (orthographic globe with optional ellipsoid exaggeration and altitude extrusion), and **vector diagram** (unit-free engineering diagrams such as wind triangles, airspeed gauges, traverse sketches, and earthwork cross-sections). Each tool's manifest `visualization` descriptor SHALL select the default mode; the user SHALL be able to switch among modes the tool supports.

#### Scenario: Wind triangle defaults to vector mode
- **WHEN** a user opens the wind-triangle tool
- **THEN** the canvas shows a vector diagram of the true-airspeed vector, wind vector, and ground vector with labeled magnitudes and angles

#### Scenario: Geodesic defaults to globe
- **WHEN** a user opens the geodesic inverse tool with two points
- **THEN** the canvas shows the globe with both points, the geodesic path, and the rhumb line as a dashed comparison

### Requirement: Real-time update
The canvas SHALL re-render within one animation frame after a new result is available, and SHALL render at least 60 frames per second during pan, zoom, and rotate for scenes of up to 100,000 vertices on the reference device.

#### Scenario: Frame budget
- **WHEN** the rendering benchmark animates a scene of 100,000 vertices while rotating the globe
- **THEN** the p95 frame time is at most 16.7 ms on the reference profile

### Requirement: Geometrically faithful rendering
Geodesic lines SHALL be rendered by densifying along the true ellipsoidal geodesic (not straight segments in screen space), with densification such that the rendered path deviates from the true path by less than 1 screen pixel at the current zoom. Rhumb lines, small circles (range rings, geofences), and polygons SHALL likewise be densified in their true geometry. Lines and polygons crossing the antimeridian or enclosing a pole SHALL render correctly in every mode.

#### Scenario: Antimeridian polygon
- **WHEN** a polygon spans from longitude 170° E to 170° W
- **THEN** the 2D map draws it as one contiguous shape across the antimeridian, not a band spanning the globe

#### Scenario: Polar cap
- **WHEN** a geofence circle of radius 500 km is centered on the North Pole
- **THEN** the globe shows a closed circular cap and the 2D polar view shows a circle

### Requirement: Layer kinds
The renderer SHALL implement each layer kind declared in the tool contract: `point`, `line-geodesic`, `line-rhumb`, `polyline`, `polygon`, `bbox`, `circle-geodesic`, `cell-set` (H3, S2, geohash, quadkey, tile, Maidenhead, MGRS squares), `grid-overlay` (UTM zones, MGRS grid, graticule, H3 or S2 at a chosen level), `vector-diagram`, `profile-chart` (elevation or atmosphere profiles), `raster` (DEM, viewshed, index maps), `gauge`, and `extrusion-3d` (altitude or airspace-like volumes).

#### Scenario: H3 k-ring
- **WHEN** a user runs H3 k-ring with k = 2 on a resolution 7 cell
- **THEN** the canvas draws 19 hexagon outlines with the origin highlighted and ring distance encoded by line intensity

### Requirement: Interactive input from the canvas
Users SHALL be able to set point inputs by clicking or tapping the canvas and to drag existing input points, with the form updating live. Every canvas-driven input SHALL have an equivalent keyboard/form path (WCAG 2.5.7 dragging alternative).

#### Scenario: Drag a waypoint
- **WHEN** a user drags waypoint B on the map
- **THEN** B's coordinate fields update continuously, the result recomputes, and the route redraws

### Requirement: Measurement readouts in the HUD
The canvas SHALL show a HUD overlay with cursor coordinates (in the user's chosen format), scale bar, north arrow (true north; magnetic north indicator when the tool involves magnetic values), and the current view's projection name.

#### Scenario: Cursor readout format
- **WHEN** the coordinate display setting is MGRS
- **THEN** the cursor readout shows MGRS for the location under the pointer

### Requirement: Basemap is optional, keyless, and offline-capable
The canvas SHALL render without any basemap by default using bundled Natural Earth coastlines and a graticule. An optional vector basemap (self-hosted tiles) MAY be enabled in settings and SHALL show required attribution when on. No basemap SHALL require an API key or third-party request.

#### Scenario: Offline canvas
- **WHEN** the device is offline and no offline basemap pack is installed
- **THEN** the canvas still renders coastlines, graticule, and all tool layers

### Requirement: GPU capability fallback
The renderer SHALL use WebGPU when available and fall back to WebGL2 otherwise, with identical layer content. If neither is available, the canvas SHALL fall back to a 2D-canvas renderer supporting at least points, lines, polygons, and vector diagrams, and SHALL indicate reduced capability.

#### Scenario: No WebGPU
- **WHEN** the browser lacks WebGPU (for example Firefox on Linux)
- **THEN** the canvas renders through WebGL2 with the same layers and the user sees no error

### Requirement: Retro-HUD visual effects are optional and safe
Phosphor glow, persistence trails, scanlines, and slight vignetting MAY be applied as post-processing. These effects SHALL be disabled when `prefers-reduced-motion` is set or the user turns them off, SHALL never reduce the contrast of text or data marks below the visual-theme minimums, and SHALL never flash more than 3 times per second.

#### Scenario: Reduced motion
- **WHEN** the operating system requests reduced motion
- **THEN** persistence trails and animated scanlines are off, and view transitions are instant

### Requirement: Canvas export
Users SHALL be able to export the current canvas as PNG (with attribution and a result caption), as SVG for the 2D and vector modes, and as GeoJSON of all geographic layers.

#### Scenario: Export with attribution
- **WHEN** a user exports a PNG of a view that includes DEM-derived layers
- **THEN** the image includes the DEM attribution text in a footer strip

### Requirement: Accessible description of the canvas
The canvas SHALL expose a text alternative summarizing what is drawn (for example "Geodesic from A to B, 5,570 km, initial course 51°, crosses 60° N") via an accessible description updated with each result.

#### Scenario: Screen reader summary
- **WHEN** a screen-reader user computes a route
- **THEN** the canvas's accessible description states the route endpoints, length, and initial course
