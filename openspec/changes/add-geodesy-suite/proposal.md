## Why

Every other geoprims domain stands on geodesy. A drone flight plan, a traverse, and an H3 polyfill all start with "where exactly is this point, in which reference frame, at what height?" Practitioners get this wrong constantly:

- lat/lon swapped, DMS misparsed
- NAD83 treated as WGS84 (a 1–2 m error today)
- orthometric heights used where ellipsoid heights belong
- declination taken from an expired model

Today they fix it by hopping between NGS web forms, epsg.io (ads and trackers), and desktop GIS. geoprims makes the fundamentals correct, explicit about frames and epochs, and instant.

Depends on: `establish-platform-foundation`.

## What Changes

Adds the `geodesy` domain: about 78 operations and 162 endpoints (inventory in `design.md`).

- **Coordinate parsing and formatting:** DD, DMS, DDM, signed and hemisphere forms, packed forms, and auto-detection with ambiguity reporting.
- **Reference frames and ellipsoids:** geodetic ↔ ECEF ↔ ENU/NED/AER, ellipsoid parameters, radii of curvature, auxiliary latitudes, meridian arc length.
- **Datums and transformations:** WGS 84 realizations, ITRF, NAD83(2011), NAD27 via NADCON5, time-dependent 14-parameter Helmert with explicit epochs, and plate-motion velocity. NATRF2022 is supported and labeled beta.
- **Projections:** UTM, UPS, SPCS83 and SPCS2022 (beta), Transverse Mercator, Lambert Conformal Conic, Albers, Hotine Oblique Mercator, Web Mercator, and azimuthal equidistant, plus grid convergence and point scale factor.
- **Grid references:** MGRS, USNG, Maidenhead, GARS, and GEOREF, all with precision control and polar handling.
- **Heights and geoid:** EGM96, EGM2008, and GEOID18 undulation; ellipsoidal ↔ orthometric conversion; height-reference conversion (HAE, MSL, AGL).
- **Geomagnetism:** WMM2025, WMMHR2025, and IGRF-14 field elements; true ↔ magnetic conversion; grid variation; blackout and caution zones; uncertainty.

## Capabilities

### New Capabilities

- `geodesy/coordinate-parsing`: Parsing, detecting, validating, and formatting coordinate and angle notations.
- `geodesy/reference-frames`: Ellipsoids, auxiliary latitudes, and geodetic/ECEF/local-tangent-plane conversions.
- `geodesy/datums-and-transformations`: Datum and reference-frame transformations with epochs.
- `geodesy/projections`: Map projections, State Plane, UTM/UPS, convergence, and scale factor.
- `geodesy/grid-references`: MGRS, USNG, Maidenhead, GARS, and GEOREF.
- `geodesy/heights-and-geoid`: Geoid models and height-system conversions.
- `geodesy/geomagnetism`: Magnetic field models and true/magnetic conversions.

### Modified Capabilities

None.

## Non-goals

- A general "any EPSG code to any EPSG code" engine covering all ~7,000 EPSG CRSs. The curated registry covers the CRSs practitioners in scope use. A full PROJ lab is a possible later addition (foundation design D4).
- Vertical datum transformations beyond geoid-based ones (e.g. tidal datums as in VDatum) in v1.
- Crustal-anomaly magnetic models beyond WMMHR (EMM is excluded).
- Legal survey determinations. Outputs are computational.

## Impact

- `core/gp-geodesy` crate, which also serves as a dependency of the navigation, survey, drone, and indexing crates for parsing and frames.
- Assets: `wmm2025`, `wmmhr2025`, `igrf14`, `egm96-15`, `egm2008-*`, `geoid18`, `nadcon5-*`, `crs-registry`, `spcs2022-beta`.
- Differential references: GeographicLib C++, PROJ 9.x, NGS HTDP and NCAT sample outputs, NOAA WMM test values, NGA/GEOTRANS MGRS cases.
