## Purpose

Renders every geoprims calculation on a live canvas (2D map, 3D globe, or engineering vector diagram) so users see the geometry of their inputs and results and can catch mistakes at a glance. The canvas is the product's centerpiece: clean, cartographic, and in motion where motion explains the math.

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

### Requirement: Measurement readouts
The canvas SHALL show a quiet readout overlay with cursor coordinates (in the user's chosen format), scale bar, north arrow (true north; magnetic north indicator when the tool involves magnetic values), and the current view's projection name.

#### Scenario: Cursor readout format
- **WHEN** the coordinate display setting is MGRS
- **THEN** the cursor readout shows MGRS for the location under the pointer

### Requirement: Natural Earth is the only basemap
The canvas SHALL draw its base layer entirely from Natural Earth vector data shipped with the site: `ne-110m` (bundled) for world and regional views and `ne-50m` (loaded on demand as one static file) when zoomed in, with land, coastlines, country borders, lakes, and a graticule. There SHALL be no tiled basemap, no tile server, and no map service of any kind: the base layer is static files deployed with the website (`ne-110m` is the same file the MCP server bundles). No basemap SHALL require an API key or a third-party request. Beyond the zoom where Natural Earth detail runs out, the canvas SHALL keep drawing tool layers precisely over the generalized base and SHALL say so in the readout ("Base map generalized at this zoom").

#### Scenario: Offline canvas
- **WHEN** the device is offline after the site has been installed
- **THEN** the canvas still renders land, coastlines, graticule, and all tool layers

#### Scenario: No tile requests
- **WHEN** a user pans and zooms the 2D map and spins the globe
- **THEN** the network log shows no basemap tile requests

### Requirement: Cartographic style
The canvas SHALL follow `web/visual-theme`: in `paper` mode, water is the page surface, land is a slightly deeper neutral fill, coastlines and borders are hairlines, and the graticule is faint; `ink` mode inverts the same relationships. Tool layers SHALL be the only strongly colored marks: the signal accent for the primary result, neutrals for inputs and comparisons, and a light casing (halo) around lines and labels so they read over any fill. The globe SHALL have soft limb shading that gives it depth without obscuring data. Labels SHALL use the product sans and never overlap one another.

#### Scenario: Result stands out
- **WHEN** the geodesic inverse tool draws its path over land and sea
- **THEN** the path is the only accent-colored mark and keeps at least 3:1 contrast against every fill it crosses

### Requirement: Animated scenes and playback
Tools whose results unfold over time or distance SHALL declare a timeline in their manifest `visualization` descriptor, and the canvas SHALL animate them: for example closest point of approach (both tracks moving to the CPA moment), fly-by and holding turns (the aircraft symbol flying the path), sun position and twilight (the terminator sweeping across the globe), drone survey patterns (the aircraft flying lines while footprints fill in), and route legs. Animated scenes SHALL provide play and pause, a scrubber, speed choices, and loop; the playhead SHALL be part of the view state saved in the permalink. Every value shown at the playhead SHALL come from the core, not from interpolation in the renderer. Scenes SHALL open paused on the result's key moment (for example the CPA), so the answer is visible without playing.

When a result first appears or changes, the canvas SHALL ease the camera to frame it (at most 800 ms) and MAY draw new paths in along their length. Animation SHALL never carry information that is not also in the result panel and the canvas's accessible description, and SHALL meet WCAG 2.2.2 (pausable; nothing auto-plays for more than 5 seconds).

#### Scenario: CPA playback
- **WHEN** a user presses play on the closest-point-of-approach tool
- **THEN** both tracks advance together, the separation readout updates from core values, and the scene pauses at the end with the CPA marked

#### Scenario: Scrub from a permalink
- **WHEN** a user opens a permalink saved with the sun-position scene scrubbed to 18:40 UTC
- **THEN** the globe shows the terminator at 18:40 UTC, paused

#### Scenario: Reduced motion
- **WHEN** reduced motion is requested
- **THEN** camera moves and path draw-ins are instant, nothing plays automatically, and the scrubber still works

### Requirement: GPU capability fallback
The renderer SHALL use WebGPU when available and fall back to WebGL2 otherwise, with identical layer content. If neither is available, the canvas SHALL fall back to a 2D-canvas renderer supporting at least points, lines, polygons, and vector diagrams, and SHALL indicate reduced capability.

#### Scenario: No WebGPU
- **WHEN** the browser lacks WebGPU (for example Firefox on Linux)
- **THEN** the canvas renders through WebGL2 with the same layers and the user sees no error

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
