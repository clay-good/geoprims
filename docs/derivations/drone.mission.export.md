<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Mission export (`drone.mission.export`)

## Method

The tool writes a list of waypoints as a file another program can open: KML for map viewers, GeoJSON for GIS work, or CSV for a spreadsheet. It does no geometry. Its one piece of arithmetic is the height: KML has an altitude mode, so the tool picks the mode that matches the height reference and, where KML needs heights above sea level, converts them with the offset the user gives. Every file names the tool, its version, the height reference, and a not-for-navigation notice.

## Equations

- KML, above ground (AGL): altitudeMode relativeToGround, height as entered.
- KML, above sea level (MSL): altitudeMode absolute, height as entered.
- KML, above takeoff: altitudeMode absolute, height + takeoff elevation.
- KML, above the ellipsoid (HAE): altitudeMode absolute, height − geoid height N.
- KML, terrain following: altitudeMode absolute, with the clearance margin (15 m unless set) on every waypoint, and only after the surface-model notice is acknowledged.
- GeoJSON: each position is [longitude, latitude, height], the height as entered, with its reference in the feature's properties.
- CSV: one row per waypoint: latitude, longitude, height, reference, heading, gimbal pitch, action.

## Symbols and units

Latitude and longitude in decimal degrees on WGS 84, written to 7 decimal places. Heights in meters, to 0.01 m. N is the geoid height (the geoid above the ellipsoid, negative where it lies below).

## Domain

At least one waypoint. A takeoff reference needs the takeoff elevation and an ellipsoid reference needs the geoid height when the format is KML; without them the request is INVALID_INPUT at that field. Terrain following is refused until the surface-model notice is acknowledged.

## Approximations

None in the coordinates beyond the rounding above. One geoid height and one takeoff elevation are applied to every waypoint, which is exact for the takeoff and good to the geoid's slope over the mission for the ellipsoid. Heading, gimbal pitch, and actions are written as notes or properties that a flight app may ignore. No vendor mission format is written.

## Worked example

- sourcePublisher: Open Source Geospatial Foundation (GDAL)
- sourceTitle: GDAL 3.13.3, ogrinfo, reading the tool's KML and GeoJSON files with its KML and GeoJSON drivers
- sourceEdition: GDAL 3.13.3 (2026-08-13)
- sourceLocator: tools/diff/export.test.mjs: 16 seeded missions of 2 to 7 waypoints in both hemispheres and both sides of the antimeridian, each written as KML and as GeoJSON in turn for the four height references
- independent: yes
- inputs: for example, waypoints at 40.4406, −80.002 at 80 m and 40.5, −80.1 at 95 m, above takeoff with a takeoff elevation of 300 m, as KML
- outputs: GDAL reads the points back as (−80.002, 40.4406, 380) and (−80.1, 40.5, 395): longitude first, in order, at the sea-level heights the rule gives; the same mission as GeoJSON reads back at 80 and 95
- tolerance: 1e-9° in position and 1e-6 m in height
- verifiedBy: automated differential test (below), which runs wherever GDAL is installed
- verifiedOn: 2026-10-10

GDAL is a separate implementation of both file formats, so a file it reads back correctly is one other software can open. The media type each file carries is the one its specification registers (vectors v019 through v021).

## Differential tests

- `tools/diff/export.test.mjs`: GDAL's ogrinfo reads every exported KML and GeoJSON file and must return the same waypoints in order, longitude before latitude, at the height the reference's rule gives (32 files); skipped where GDAL is not installed
- `tools/vectors/gen_drone.py`: the altitude mode, file name, and waypoint count for every height reference and format, the refused requests, the terrain-following margin, and each format's media type
- `core/vectors/drone.mission.export.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `kml_altitude_mode_follows_the_height_reference`: the KML is well formed and carries the altitude mode the reference calls for on every placemark
- `core/crates/gp-drone/tests/mission.rs` `geojson_and_csv_carry_every_waypoint`: the GeoJSON parses with longitude, latitude, height order and per-waypoint properties, and the CSV has one quoted row per waypoint
