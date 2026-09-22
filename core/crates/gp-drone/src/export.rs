//! Mission export (add-drone-suite, drone/mission-patterns, "Export"): a
//! waypoint list written as KML (altitude mode matching the height
//! reference), GeoJSON (a property set per waypoint), or CSV, each with a
//! header naming the tool and version and a not-for-navigation notice.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::unit;

const KML: Reference = Reference {
    title: "OGC KML 2.3",
    issuer: "Open Geospatial Consortium",
    year: 2015,
    edition: "OGC 12-007r2",
    locator: "Section 9.20 (kml:altitudeMode: clampToGround, relativeToGround, absolute, which is relative to sea level)",
    url: "https://docs.ogc.org/is/12-007r2/12-007r2.html",
};
const GEOJSON: Reference = Reference {
    title: "The GeoJSON Format",
    issuer: "IETF RFC 7946",
    year: 2016,
    edition: "RFC 7946",
    locator: "Sections 3.1 (geometry: longitude, latitude, altitude) and 6.1 (foreign members)",
    url: "https://www.rfc-editor.org/rfc/rfc7946",
};

const CSV: Reference = Reference {
    title: "Common Format and MIME Type for Comma-Separated Values (CSV) Files",
    issuer: "IETF RFC 4180",
    year: 2005,
    edition: "RFC 4180",
    locator: "Section 2 (quoting fields that hold commas, quotes, or line breaks)",
    url: "https://www.rfc-editor.org/rfc/rfc4180",
};

const NOTICE: &str = "Planning aid only, not for navigation. Check every waypoint before flight.";

const WAYPOINT: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.4406",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -80.002",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "height",
        "Height",
        "In the chosen reference, like 80 m",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .required(),
    Field::new(
        "heading",
        "Heading",
        "Degrees true, like 90",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    ),
    Field::new(
        "gimbal_pitch",
        "Gimbal pitch",
        "Degrees, down negative, like -90",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    ),
    Field::new(
        "action",
        "Action",
        "Like photo, or hover 5 s",
        Kind::Text { max_len: 40 },
    ),
];

pub static EXPORT: ToolDef = ToolDef {
    id: "drone.mission.export",
    title: "Export a mission (KML, GeoJSON, CSV)",
    summary: "Waypoints written as a KML file with the right altitude mode, GeoJSON with each waypoint's details, or CSV, labeled with the height reference and a not-for-navigation notice.",
    aliases: &["export waypoints", "mission to KML", "waypoints to GeoJSON", "waypoint CSV", "KML altitude mode"],
    keywords: &["export", "KML", "GeoJSON", "CSV", "waypoints", "altitude mode", "relativeToGround", "mission file"],
    inputs: &[
        Field::new("waypoints", "Waypoints", "In flight order, one per line: lat, lon, height, like 40.4406, -80.002, 80", Kind::List { items: WAYPOINT, min: 1, max: 10_000 }).required().core(),
        Field::new("height_reference", "Heights are", "agl (above the ground below), takeoff (above the takeoff point), msl (above sea level), hae (above the ellipsoid), or terrain (above sea level, following a surface model such as GLO-30)", Kind::Choice(&["agl", "takeoff", "msl", "hae", "terrain"])).required().core(),
        Field::new("format", "Format", "kml (the default), geojson, or csv", Kind::Choice(&["kml", "geojson", "csv"])).core(),
        Field::new("takeoff_elevation", "Takeoff elevation", "Above sea level, like 312 m; needed to write takeoff heights to KML", Kind::Quantity { q: QT::Length, unit: "m" }),
        Field::new("geoid_height", "Geoid height", "N at the site, like -33.9 m (from the geoid tool); needed to write HAE to KML", Kind::Quantity { q: QT::Length, unit: "m" }),
        Field::new("name", "Mission name", "Like North field, shown in the file", Kind::Text { max_len: 80 }),
        Field::new("surface_model_acknowledged", "Surface-model notice", "yes: terrain-following heights come from a surface model that includes trees and buildings inconsistently", Kind::Choice(&["yes"])),
        Field::new("clearance_margin", "Clearance margin", "Added to every terrain-following waypoint, like 15 m (the default)", Kind::Quantity { q: QT::Length, unit: "m" }),
    ],
    outputs: &[
        Field::new("waypoint_count", "Waypoints", "Written to the file", Kind::Number { min: 0.0, max: 1e6 }).precision(Precision::Decimals(0)),
        Field::new("filename", "File name", "Suggested", Kind::Text { max_len: 120 }),
        Field::new("heights", "Heights", "What the file's heights are measured from", Kind::Text { max_len: 80 }),
        Field::new("altitude_mode", "Heights in the file", "KML altitude mode, or the reference as labeled", Kind::Text { max_len: 60 }),
        Field::new("media_type", "Media type", "For the download", Kind::Text { max_len: 60 }),
        Field::new("file", "File", "The file's text", Kind::Text { max_len: 50_000_000 }),
        Field::new("clearance_margin", "Clearance margin added", "To every terrain-following waypoint", Kind::Quantity { q: QT::Length, unit: "m" }).precision(Precision::Decimals(1)).optional(),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "KML altitude mode follows the height reference: AGL is relativeToGround; MSL is absolute; takeoff heights become absolute by adding the takeoff's MSL elevation; HAE becomes absolute by subtracting the geoid height N (OGC KML 2.3 §9.20; KML's absolute is above sea level). GeoJSON carries [longitude, latitude, height] with the reference in each waypoint's properties (RFC 7946); CSV has one row per waypoint",
    accuracy: "Coordinates to 7 decimal places (about 1 cm) and heights to 0.01 m; no conversion beyond the stated offsets",
    references: &[KML, GEOJSON, CSV],
    examples: &[Example {
        id: "primary",
        title: "Three waypoints at 80 m AGL, as KML",
        input: r#"{"waypoints":[{"lat":40.4406,"lon":-80.002,"height":"80 m","heading":90,"gimbal_pitch":-90,"action":"photo"},{"lat":40.4406,"lon":-80.0005,"height":"80 m","heading":90,"gimbal_pitch":-90,"action":"photo"},{"lat":40.4412,"lon":-80.0005,"height":"80 m","heading":270,"gimbal_pitch":-90,"action":"photo"}],"height_reference":"agl","format":"kml","name":"North field"}"#,
        source: "add-drone-suite KML altitude-mode scenario (AGL writes relativeToGround)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "drone.mission.survey-grid", reason: "parent" },
        Related { id: "drone.mission.geofence", reason: "alternative" },
    ],
    sentence: "The file has {waypoint_count} {plural waypoint_count \"waypoint\" \"waypoints\"}, with heights {heights}.",
    limits: &[("batchRows", 20)],
    run: run_export,
    ..ToolDef::BLANK
};

struct Wp {
    lat: f64,
    lon: f64,
    h: f64,
    heading: Option<f64>,
    pitch: Option<f64>,
    action: String,
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn csv_cell(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Rounded for the file: 7 decimals of a degree, 2 of a meter.
fn r(x: f64, d: i32) -> f64 {
    let k = [1.0, 10.0, 100.0, 1e3, 1e4, 1e5, 1e6, 1e7][d as usize];
    let v = (x * k).round() / k;
    if v == 0.0 { 0.0 } else { v }
}

fn run_export(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (d, m) = (unit(QT::Angle, "deg"), unit(QT::Length, "m"));
    let rows = ctx.rows("waypoints")?;
    let mut wps = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("waypoints", i, row, "lat")?
            .expect("required")
            .to(d);
        let lon = ctx
            .row_quantity("waypoints", i, row, "lon")?
            .expect("required")
            .to(d);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::invalid(
                &format!("/waypoints/{i}/lat"),
                "Latitude must be between -90° and 90°.",
            ));
        }
        let h = ctx
            .row_quantity("waypoints", i, row, "height")?
            .expect("required")
            .to(m);
        let heading = ctx
            .row_quantity("waypoints", i, row, "heading")?
            .map(|q| q.to(d).rem_euclid(360.0));
        let pitch = ctx
            .row_quantity("waypoints", i, row, "gimbal_pitch")?
            .map(|q| q.to(d));
        let action: String = row
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .chars()
            .take(40)
            .collect();
        wps.push(Wp {
            lat,
            lon: (lon + 540.0).rem_euclid(360.0) - 180.0,
            h,
            heading,
            pitch,
            action,
        });
    }
    let reference = ctx.choice("height_reference")?.expect("required");
    // Terrain following: blocked until the surface-model notice is acknowledged,
    // and a clearance margin (15 m unless set) rides on every waypoint.
    let margin = if reference == "terrain" {
        if ctx.choice("surface_model_acknowledged")? != Some("yes") {
            return Err(ToolError::invalid(
                "/surface_model_acknowledged",
                "Export is blocked until you acknowledge that terrain-following heights come from a surface model (like GLO-30) that includes trees and buildings inconsistently: set the surface-model notice to yes, and check the clearance margin (15 m unless you set one).",
            ));
        }
        let mg = ctx.quantity("clearance_margin")?.map_or(15.0, |q| q.to(m));
        if mg.is_nan() || !(0.0..=500.0).contains(&mg) {
            return Err(ToolError::invalid(
                "/clearance_margin",
                "Give a clearance margin from 0 to 500 m.",
            ));
        }
        for w in &mut wps {
            w.h += mg;
        }
        Some(mg)
    } else {
        None
    };
    let format = ctx.choice("format")?.unwrap_or("kml");
    let name = ctx.text("name")?.unwrap_or_else(|| "Mission".to_owned());
    let label = match reference {
        "agl" => "AGL",
        "takeoff" => "above takeoff",
        "msl" => "MSL",
        "terrain" => "terrain-following MSL",
        _ => "HAE",
    };
    let header = format!(
        "geoprims drone.mission.export, core {}. Heights {label}. {NOTICE}",
        env!("CARGO_PKG_VERSION")
    );
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let stem = if stem.is_empty() {
        "mission".to_owned()
    } else {
        stem
    };
    let plain = match reference {
        "agl" => "above the ground",
        "takeoff" => "above the takeoff point",
        "msl" => "above sea level",
        "terrain" => "above sea level, following a surface model, plus the clearance margin",
        _ => "above the ellipsoid",
    };
    let heights = match (format, reference) {
        ("kml", "takeoff") => "above sea level, with the takeoff elevation added".to_owned(),
        ("kml", "hae") => "above sea level, with the geoid height taken off".to_owned(),
        _ => plain.to_owned(),
    };
    let (file, mode, media, ext) = match format {
        "kml" => {
            // KML's absolute is above sea level, so other references convert first.
            let (mode, offset) = match reference {
                "agl" => ("relativeToGround", 0.0),
                "msl" | "terrain" => ("absolute", 0.0),
                "takeoff" => {
                    let Some(e) = ctx.quantity("takeoff_elevation")?.map(|q| q.to(m)) else {
                        return Err(ToolError::invalid(
                            "/takeoff_elevation",
                            "KML has no above-takeoff mode: give the takeoff's elevation above sea level, like 312 m, and the heights are written as absolute.",
                        ));
                    };
                    ("absolute", e)
                }
                _ => {
                    let Some(n) = ctx.quantity("geoid_height")?.map(|q| q.to(m)) else {
                        return Err(ToolError::invalid(
                            "/geoid_height",
                            "KML heights are above sea level, not the ellipsoid: give the geoid height N at the site, like -33.9 m (the geoid tool finds it).",
                        ));
                    };
                    ("absolute", -n)
                }
            };
            let coord = |w: &Wp| format!("{},{},{}", r(w.lon, 7), r(w.lat, 7), r(w.h + offset, 2));
            let mut k = String::new();
            k.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            k.push_str(&format!("<!-- {} -->\n", xml(&header)));
            k.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n");
            k.push_str(&format!("<name>{}</name>\n", xml(&name)));
            k.push_str(&format!("<description>{}</description>\n", xml(NOTICE)));
            k.push_str(&format!(
                "<Placemark>\n<name>Flight path</name>\n<LineString>\n<altitudeMode>{mode}</altitudeMode>\n<coordinates>{}</coordinates>\n</LineString>\n</Placemark>\n",
                wps.iter().map(coord).collect::<Vec<_>>().join(" ")
            ));
            for (i, w) in wps.iter().enumerate() {
                let mut notes = Vec::new();
                if let Some(h) = w.heading {
                    notes.push(format!("heading {}°", r(h, 1)));
                }
                if let Some(p) = w.pitch {
                    notes.push(format!("gimbal {}°", r(p, 1)));
                }
                if !w.action.is_empty() {
                    notes.push(w.action.clone());
                }
                k.push_str(&format!(
                    "<Placemark>\n<name>{}</name>\n<description>{}</description>\n<Point>\n<altitudeMode>{mode}</altitudeMode>\n<coordinates>{}</coordinates>\n</Point>\n</Placemark>\n",
                    i + 1,
                    xml(&notes.join(", ")),
                    coord(w)
                ));
            }
            k.push_str("</Document>\n</kml>\n");
            (
                k,
                mode.to_owned(),
                "application/vnd.google-earth.kml+xml",
                "kml",
            )
        }
        "geojson" => {
            let pt = |w: &Wp| {
                Json::Arr(vec![
                    Json::Num(r(w.lon, 7)),
                    Json::Num(r(w.lat, 7)),
                    Json::Num(r(w.h, 2)),
                ])
            };
            let mut features = vec![Json::obj([
                ("type", Json::str("Feature")),
                (
                    "geometry",
                    Json::obj([
                        ("type", Json::str("LineString")),
                        ("coordinates", Json::Arr(wps.iter().map(pt).collect())),
                    ]),
                ),
                (
                    "properties",
                    Json::obj([
                        ("name", Json::str("Flight path")),
                        ("height_reference", Json::str(label)),
                    ]),
                ),
            ])];
            for (i, w) in wps.iter().enumerate() {
                let mut props = vec![
                    ("index", Json::Num((i + 1) as f64)),
                    ("height", Json::Num(r(w.h, 2))),
                    ("height_reference", Json::str(label)),
                ];
                if let Some(h) = w.heading {
                    props.push(("heading", Json::Num(r(h, 1))));
                }
                if let Some(p) = w.pitch {
                    props.push(("gimbal_pitch", Json::Num(r(p, 1))));
                }
                if !w.action.is_empty() {
                    props.push(("action", Json::str(w.action.clone())));
                }
                features.push(Json::obj([
                    ("type", Json::str("Feature")),
                    (
                        "geometry",
                        Json::obj([("type", Json::str("Point")), ("coordinates", pt(w))]),
                    ),
                    ("properties", Json::obj(props)),
                ]));
            }
            let doc = Json::obj([
                ("type", Json::str("FeatureCollection")),
                ("name", Json::str(name.clone())),
                // RFC 7946 foreign members carry what JSON cannot say in a comment.
                (
                    "geoprims",
                    Json::obj([
                        ("tool", Json::str("drone.mission.export")),
                        ("core", Json::str(env!("CARGO_PKG_VERSION"))),
                        ("height_reference", Json::str(label)),
                        ("notice", Json::str(NOTICE)),
                    ]),
                ),
                ("features", Json::Arr(features)),
            ]);
            (
                doc.to_string()? + "\n",
                format!("{label} in each waypoint's properties"),
                "application/geo+json",
                "geojson",
            )
        }
        _ => {
            let mut c = format!("# {header}\n# {}\n", name.replace('\n', " "));
            c.push_str("index,lat,lon,height_m,reference,heading_deg,gimbal_pitch_deg,action\n");
            for (i, w) in wps.iter().enumerate() {
                c.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    i + 1,
                    r(w.lat, 7),
                    r(w.lon, 7),
                    r(w.h, 2),
                    csv_cell(label),
                    w.heading.map_or(String::new(), |h| r(h, 1).to_string()),
                    w.pitch.map_or(String::new(), |p| r(p, 1).to_string()),
                    csv_cell(&w.action)
                ));
            }
            (
                c,
                format!("{label} in the reference column"),
                "text/csv",
                "csv",
            )
        }
    };
    let mut out = vec![
        ("waypoint_count", Json::Num(wps.len() as f64)),
        ("filename", Json::str(format!("{stem}.{ext}"))),
        ("heights", Json::str(heights)),
        ("altitude_mode", Json::str(mode)),
        ("media_type", Json::str(media)),
        ("file", Json::str(file)),
    ];
    if let Some(mg) = margin {
        out.push((
            "clearance_margin",
            ctx.out("clearance_margin", Q { value: mg, unit: m }),
        ));
    }
    Ok(Json::obj(out))
}
