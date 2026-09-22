//! NOTAM and TFR geometry (add-practitioner-essentials, instrument-procedures
//! "NOTAM and TFR geometry"): the areas NOTAM text describes, as polygons with
//! area and bounds: a geodesic circle around packed coordinates
//! (`393400N1224330W`) or a fix-radial-distance (`ABC012098.7`, with the
//! navaid's position and variation from the user), or a list of points; the
//! altitude limits ride along as properties of the GeoJSON.

use geographiclib_rs::{DirectGeodesic, Geodesic, PolygonArea, Winding};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point;

use crate::unit;

const NOTAM_ORDER: Reference = Reference {
    title: "Notices to Air Missions (NOTAM), FAA Order JO 7930.2",
    issuer: "Federal Aviation Administration",
    year: 2025,
    edition: "JO 7930.2U, effective August 10, 2025",
    locator: "TFR and airspace NOTAM formats: latitude and longitude as degrees, minutes, and seconds with N/S and E/W, fix-radial-distance references, radii in nautical miles, and altitude limits",
    url: "https://www.faa.gov/air_traffic/publications/atpubs/notam_html/",
};

const RFC7946: Reference = Reference {
    title: "The GeoJSON Format (RFC 7946)",
    issuer: "Internet Engineering Task Force",
    year: 2016,
    edition: "RFC 7946",
    locator: "Section 3.1.6 (Polygon, counterclockwise exterior rings) and 3.2 (Feature properties)",
    url: "https://www.rfc-editor.org/rfc/rfc7946",
};

const POINT_ROW: &[Field] = &[Field::new(
    "point",
    "Point",
    "Packed coordinates, like 393400N1224330W",
    Kind::Text { max_len: 24 },
)
.required()];

const RING_ROW: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new("part", "Part", "0", Kind::Number { min: 0.0, max: 0.0 })
        .precision(Precision::Decimals(0)),
];

/// Degrees from packed NOTAM coordinates: `393400N1224330W` (seconds),
/// `3934N12243W` (minutes), with an optional decimal on the last part.
pub fn packed(s: &str) -> Option<(f64, f64)> {
    let u = s.trim().to_ascii_uppercase();
    let ns = u.find(['N', 'S'])?;
    let (lat_s, rest) = u.split_at(ns);
    let hemi_lat = if rest.starts_with('S') { -1.0 } else { 1.0 };
    let rest = &rest[1..];
    let ew = rest.find(['E', 'W'])?;
    if ew + 1 != rest.len() {
        return None;
    }
    let (lon_s, h) = rest.split_at(ew);
    let hemi_lon = if h == "W" { -1.0 } else { 1.0 };
    let part = |t: &str, deg_digits: usize| -> Option<f64> {
        let (int, frac) = t.split_once('.').unwrap_or((t, ""));
        if !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let d: f64 = int.get(..deg_digits)?.parse().ok()?;
        let tail = &int[deg_digits..];
        let frac_v: f64 = if frac.is_empty() {
            0.0
        } else {
            format!("0.{frac}").parse().ok()?
        };
        let v = match tail.len() {
            2 => d + (tail.parse::<f64>().ok()? + frac_v) / 60.0,
            4 => {
                let m: f64 = tail[..2].parse().ok()?;
                let sec: f64 = tail[2..].parse::<f64>().ok()? + frac_v;
                if m >= 60.0 || sec >= 60.0 {
                    return None;
                }
                d + m / 60.0 + sec / 3600.0
            }
            _ => return None,
        };
        Some(v)
    };
    let lat = part(lat_s, 2)? * hemi_lat;
    let lon = part(lon_s, 3)? * hemi_lon;
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

/// A fix-radial-distance: `ABC012098.7` → (radial, distance in NM).
pub fn frd(s: &str) -> Option<(f64, f64)> {
    let u = s.trim().to_ascii_uppercase();
    let ident_len = u.bytes().take_while(u8::is_ascii_alphabetic).count();
    if !(2..=5).contains(&ident_len) {
        return None;
    }
    let rest = &u[ident_len..];
    if rest.len() < 6 || !rest[..6].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let radial: f64 = rest[..3].parse().ok()?;
    let dist: f64 = rest[3..].parse().ok()?;
    (radial < 360.0).then_some((radial, dist))
}

/// An altitude limit as NOTAMs write it: SFC, a number of feet (MSL unless
/// AGL is said), or a flight level; normalized text.
fn limit(s: &str, field: &str) -> Result<String, ToolError> {
    let u = s.trim().to_ascii_uppercase().replace(',', "");
    if matches!(u.as_str(), "SFC" | "SURFACE" | "GND") {
        return Ok("SFC".into());
    }
    if let Some(n) = u.strip_prefix("FL")
        && let Ok(v) = n.trim().parse::<u32>()
        && v <= 600
    {
        return Ok(format!("FL{v:03}"));
    }
    let digits: String = u.chars().take_while(char::is_ascii_digit).collect();
    if let Ok(v) = digits.parse::<u32>() {
        let tail = u[digits.len()..].trim().trim_start_matches("FT").trim();
        let reference = match tail {
            "" | "MSL" => "MSL",
            "AGL" => "AGL",
            _ => {
                return Err(ToolError::invalid(
                    field,
                    format!(
                        "\"{s}\" is not an altitude limit like SFC, 3000 FT MSL, 500 AGL, or FL180."
                    ),
                ));
            }
        };
        if v <= 60_000 {
            return Ok(format!("{v} ft {reference}"));
        }
    }
    Err(ToolError::invalid(
        field,
        format!("\"{s}\" is not an altitude limit like SFC, 3000 FT MSL, 500 AGL, or FL180."),
    ))
}

pub static TFR_AREA: ToolDef = ToolDef {
    id: "aviation.airspace.tfr-area",
    title: "TFR and NOTAM area",
    summary: "Draws the area a TFR or NOTAM describes, a circle around packed coordinates or a fix-radial-distance, or a list of points, with its area, bounds, and altitude limits, ready to download as GeoJSON.",
    aliases: &[
        "TFR map",
        "TFR circle",
        "NOTAM area",
        "temporary flight restriction area",
    ],
    keywords: &[
        "TFR",
        "NOTAM",
        "airspace",
        "radius",
        "circle",
        "polygon",
        "GeoJSON",
        "flight restriction",
    ],
    inputs: &[
        Field::new(
            "center",
            "Center",
            "Packed like 393400N1224330W, or a fix-radial-distance like ABC012098.7",
            Kind::Text { max_len: 24 },
        )
        .core(),
        Field::new(
            "radius",
            "Radius",
            "Like 3 NM",
            Kind::Quantity {
                q: QT::Distance,
                unit: "NM",
            },
        )
        .core(),
        Field::new(
            "points",
            "Or the corner points",
            "One per line, packed like 393400N1224330W",
            Kind::List {
                items: POINT_ROW,
                min: 3,
                max: 200,
            },
        )
        .core(),
        Field::new("floor", "Floor", "Like SFC", Kind::Text { max_len: 24 }).core(),
        Field::new(
            "ceiling",
            "Ceiling",
            "Like 3000 FT MSL or FL180",
            Kind::Text { max_len: 24 },
        )
        .core(),
        Field::new(
            "navaid_lat",
            "Navaid latitude",
            "For a fix-radial-distance, like 37.72",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "navaid_lon",
            "Navaid longitude",
            "For a fix-radial-distance, like -122.22",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-180,180)"),
        Field::new(
            "navaid_variation",
            "Navaid variation",
            "For a fix-radial-distance, east positive, like 14",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "area",
            "Area",
            "Of the polygon on the ellipsoid",
            Kind::Quantity {
                q: QT::Area,
                unit: "NM2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new("floor", "Floor", "Normalized", Kind::Text { max_len: 24 }).optional(),
        Field::new(
            "ceiling",
            "Ceiling",
            "Normalized",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        point::lat_field("center_lat", "Center latitude")
            .precision(Precision::Decimals(7))
            .optional(),
        point::lon_field("center_lon", "Center longitude")
            .precision(Precision::Decimals(7))
            .optional(),
        point::lat_field("south", "South edge").precision(Precision::Decimals(5)),
        point::lat_field("north", "North edge").precision(Precision::Decimals(5)),
        point::lon_field("west", "West edge").precision(Precision::Decimals(5)),
        point::lon_field("east", "East edge").precision(Precision::Decimals(5)),
        Field::new(
            "filename",
            "File name",
            "Suggested",
            Kind::Text { max_len: 60 },
        ),
        Field::new(
            "media_type",
            "Media type",
            "For the download",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "file",
            "GeoJSON",
            "The area as an RFC 7946 feature with its limits",
            Kind::Text { max_len: 1_000_000 },
        ),
        Field::new(
            "rings",
            "Outline",
            "The polygon's points, for drawing",
            Kind::List {
                items: RING_ROW,
                min: 0,
                max: 1_000,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "A circle is 72 points at 5° steps of the geodesic direct problem from the center (Karney 2013); a fix-radial-distance center is the navaid's position carried along radial + variation for the distance. Area by Karney's geodesic polygon area; the GeoJSON ring runs counterclockwise and closes on its first point (RFC 7946)",
    accuracy: "Points exact on WGS 84; the 72-point circle's chords sag under 0.1% of the radius. Check the NOTAM itself: this draws what you enter",
    references: &[NOTAM_ORDER, RFC7946],
    examples: &[Example {
        id: "primary",
        title: "A 3 NM TFR, surface to 3,000 ft MSL",
        input: r#"{"center":"393400N1224330W","radius":"3 NM","floor":"SFC","ceiling":"3000 FT MSL"}"#,
        source: "add-practitioner-essentials TFR scenario: a 3 NM geodesic circle, exportable as GeoJSON with its altitude limits",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("rings", "rings")],
    }],
    related: &[
        Related {
            id: "navigation.route.range-rings",
            reason: "alternative",
        },
        Related {
            id: "aviation.ifr.radial-fix",
            reason: "parent",
        },
    ],
    sentence: "The area covers {area}, from {south} to {north} and {west} to {east}.",
    limits: &[("batchRows", 100)],
    run: run_tfr,
    ..ToolDef::BLANK
};

fn run_tfr(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = Geodesic::wgs84();
    let dg = unit(QT::Angle, "deg");
    let mut center: Option<(f64, f64)> = None;
    let ring: Vec<(f64, f64)> = match (ctx.text("center")?, ctx.is_set("points")) {
        (Some(c), false) => {
            let r = ctx
                .quantity("radius")?
                .ok_or_else(|| {
                    ToolError::invalid("/radius", "Give the radius of the circle, like 3 NM.")
                })?
                .base();
            if r.is_nan() || !(0.0..=500.0 * 1852.0).contains(&r) || r == 0.0 {
                return Err(ToolError::invalid(
                    "/radius",
                    "Give a radius above 0 and up to 500 NM.",
                ));
            }
            let c0 = if let Some(p) = packed(&c) {
                p
            } else if let Some((radial, nm)) = frd(&c) {
                let (lat, lon) = match (ctx.quantity("navaid_lat")?, ctx.quantity("navaid_lon")?) {
                    (Some(a), Some(b)) => (a.to(dg), b.to(dg)),
                    _ => {
                        return Err(ToolError::invalid(
                            "/navaid_lat",
                            "A fix-radial-distance needs the navaid's latitude and longitude, and its variation.",
                        ));
                    }
                };
                let var = ctx
                    .quantity("navaid_variation")?
                    .ok_or_else(|| {
                        ToolError::invalid(
                            "/navaid_variation",
                            "Give the navaid's published variation, east positive.",
                        )
                    })?
                    .to(dg);
                g.direct(lat, lon, (radial + var).rem_euclid(360.0), nm * 1852.0)
            } else {
                return Err(ToolError::invalid(
                    "/center",
                    format!(
                        "\"{c}\" is neither packed coordinates like 393400N1224330W nor a fix-radial-distance like ABC012098.7."
                    ),
                ));
            };
            center = Some(c0);
            // Counterclockwise: azimuths decreasing from north.
            (0..72)
                .map(|k| g.direct(c0.0, c0.1, 360.0 - 5.0 * f64::from(k), r))
                .collect()
        }
        (None, true) => {
            let rows = ctx.rows("points")?;
            let mut pts = Vec::with_capacity(rows.len());
            for (i, r) in rows.iter().enumerate() {
                let t = ctx.row_text("points", i, r, "point")?.expect("required");
                pts.push(packed(&t).ok_or_else(|| {
                    ToolError::invalid(
                        &format!("/points/{i}/point"),
                        format!("\"{t}\" is not packed coordinates like 393400N1224330W."),
                    )
                })?);
            }
            if pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
            if pts.len() < 3 {
                return Err(ToolError::invalid(
                    "/points",
                    "An area needs at least 3 distinct points.",
                ));
            }
            gp_geo::point::refuse_repeated_corner(&pts, "/points")?;
            pts
        }
        (Some(_), true) => {
            return Err(ToolError::invalid(
                "/center",
                "Give a center and radius, or corner points, not both.",
            ));
        }
        (None, false) => {
            return Err(ToolError::invalid(
                "/center",
                "Give a center and radius, or the corner points.",
            ));
        }
    };
    let lons: Vec<f64> = ring.iter().map(|p| p.1).collect();
    let (west, east) = lons
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    if east - west > 180.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This area crosses the antimeridian, which this tool does not draw.",
        )
        .at("/center"));
    }
    let (south, north) = ring
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(p.0), b.max(p.0))
        });
    let mut pa = PolygonArea::new(&g, Winding::CounterClockwise);
    for &(la, lo) in &ring {
        pa.add_point(la, lo);
    }
    let (_, area, _) = pa.compute(true);
    let area = area.abs();
    let floor = ctx
        .text("floor")?
        .map(|s| limit(&s, "/floor"))
        .transpose()?;
    let ceiling = ctx
        .text("ceiling")?
        .map(|s| limit(&s, "/ceiling"))
        .transpose()?;
    // A point-list outline goes counterclockwise in the file, whatever order it was entered in.
    let signed = {
        let n = ring.len();
        (0..n)
            .map(|i| ring[i].1 * ring[(i + 1) % n].0 - ring[(i + 1) % n].1 * ring[i].0)
            .sum::<f64>()
    };
    let mut file_ring: Vec<(f64, f64)> = ring.clone();
    if signed < 0.0 {
        file_ring.reverse();
    }
    let mut coords: Vec<Json> = file_ring
        .iter()
        .map(|&(la, lo)| Json::Arr(vec![Json::Num(lo), Json::Num(la)]))
        .collect();
    coords.push(coords[0].clone());
    let mut props = Vec::new();
    if let Some(f) = &floor {
        props.push(("floor", Json::str(f)));
    }
    if let Some(c) = &ceiling {
        props.push(("ceiling", Json::str(c)));
    }
    let doc = Json::obj([
        ("type", Json::str("Feature")),
        (
            "geometry",
            Json::obj([
                ("type", Json::str("Polygon")),
                ("coordinates", Json::Arr(vec![Json::Arr(coords)])),
            ]),
        ),
        ("properties", Json::obj(props)),
    ]);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Outline",
            "72 geodesic points for a circle, or the entered corners",
            format!("{} points", ring.len()),
            format!("{} points", ring.len()),
        );
        ctx.step(
            "Area",
            "Karney geodesic polygon area",
            format!("{} points on WGS 84", ring.len()),
            display::quantity(area / (1852.0 * 1852.0), "NM2", Precision::Decimals(2), fmt),
        );
    }
    let d = |v: f64| Q { value: v, unit: dg };
    let mut out = vec![(
        "area",
        ctx.out(
            "area",
            Q {
                value: area,
                unit: gp_base::units::base_unit(QT::Area),
            },
        ),
    )];
    if let Some(f) = floor {
        out.push(("floor", Json::str(f)));
    }
    if let Some(c) = ceiling {
        out.push(("ceiling", Json::str(c)));
    }
    if let Some((la, lo)) = center {
        out.push(("center_lat", ctx.out("center_lat", d(la))));
        out.push(("center_lon", ctx.out("center_lon", d(lo))));
    }
    out.push(("south", ctx.out("south", d(south))));
    out.push(("north", ctx.out("north", d(north))));
    out.push(("west", ctx.out("west", d(west))));
    out.push(("east", ctx.out("east", d(east))));
    out.push(("filename", Json::str("tfr-area.geojson")));
    out.push(("media_type", Json::str("application/geo+json")));
    out.push(("file", Json::str(doc.to_string()? + "\n")));
    out.push((
        "rings",
        Json::Arr(
            ring.iter()
                .map(|&(la, lo)| {
                    Json::obj([
                        ("lat", d(la).to_json()),
                        ("lon", d(lo).to_json()),
                        ("part", Json::Num(0.0)),
                    ])
                })
                .collect(),
        ),
    ));
    Ok(Json::obj(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_and_frd_forms() {
        let (la, lo) = packed("393400N1224330W").unwrap();
        assert!((la - (39.0 + 34.0 / 60.0)).abs() < 1e-12);
        assert!((lo + (122.0 + 43.0 / 60.0 + 30.0 / 3600.0)).abs() < 1e-12);
        let (la, lo) = packed("3934S12243E").unwrap();
        assert!(
            (la + (39.0 + 34.0 / 60.0)).abs() < 1e-12 && (lo - (122.0 + 43.0 / 60.0)).abs() < 1e-12
        );
        assert!(packed("396000N1224330W").is_none());
        assert!(packed("39N122W").is_none());
        assert_eq!(frd("ABC012098.7"), Some((12.0, 98.7)));
        assert_eq!(frd("OAK270015"), Some((270.0, 15.0)));
        assert!(frd("ABC400010").is_none());
    }

    #[test]
    fn altitude_limits() {
        assert_eq!(limit("sfc", "/f").unwrap(), "SFC");
        assert_eq!(limit("3,000 FT MSL", "/f").unwrap(), "3000 ft MSL");
        assert_eq!(limit("500 AGL", "/f").unwrap(), "500 ft AGL");
        assert_eq!(limit("FL180", "/f").unwrap(), "FL180");
        assert!(limit("high", "/f").is_err());
    }
}
