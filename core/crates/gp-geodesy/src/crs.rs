//! CRS search (geodesy/projections, "CRS registry lookup and transform"): the
//! coordinate systems the converters here produce, found by EPSG code, by
//! words in the name, or by a point inside the area of use, each with its
//! status and the tool that converts to it. EPSG systems come from the index
//! generated from EPSG v13.102 (gp_geo::crs); SPCS2022 zones from the NGS beta
//! asset, read only when a name or a point could match one.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::crs::{self, Crs, CrsKind};
use gp_geo::point;

use crate::spcs2022;

const EPSG: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "Coordinate reference system names, units, and areas of use",
    url: "https://epsg.org/search/by-name",
};
const NGS_SPCS2022: Reference = Reference {
    title: "State Plane Coordinate System of 2022 (SPCS2022) beta zone definitions and example coordinates",
    issuer: "National Geodetic Survey, NOAA",
    year: 2026,
    edition: "SPCS2022 beta zone definitions",
    locator: "Zone names, codes, and bounds, updated June 1, 2026",
    url: "https://beta.ngs.noaa.gov/SPCS/zone-information.html",
};

/// More matches than this are cut, with a note to narrow the search.
const MAX_ROWS: usize = 200;

const fn text(name: &'static str, title: &'static str, help: &'static str, n: usize) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: n })
}

const ROW: &[Field] = &[
    text(
        "code",
        "Code",
        "EPSG code, or the NGS code of an SPCS2022 zone",
        24,
    ),
    text("name", "Name", "As the registry gives it", 120),
    text(
        "kind",
        "Kind",
        "Geographic, UTM, State Plane, or Web Mercator",
        40,
    ),
    text("unit", "Unit", "Of its coordinates", 24),
    text(
        "status",
        "Status",
        "current, legacy, or beta (not yet adopted)",
        12,
    ),
    text(
        "convert_with",
        "Convert with",
        "The tool here that produces these coordinates",
        60,
    ),
];

pub static SEARCH: ToolDef = ToolDef {
    id: "geodesy.crs.search",
    title: "Coordinate system finder",
    summary: "Finds coordinate reference systems by EPSG code, by name, or by a point inside their area of use: UTM, State Plane 1983 and 2022, and geographic, each with its unit, its status, and the tool that converts to it.",
    aliases: &[
        "EPSG lookup",
        "find EPSG code",
        "what coordinate system is this",
        "CRS search",
        "projection finder",
    ],
    keywords: &[
        "EPSG",
        "CRS",
        "coordinate system",
        "projection",
        "UTM",
        "state plane",
        "SPCS",
        "NAD83",
        "WGS 84",
        "zone",
        "datum",
    ],
    inputs: &[
        text(
            "query",
            "EPSG code or name",
            "Like 2232, EPSG:26913, or Colorado Central",
            80,
        )
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "Or a point inside the area of use, like 39.74",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "Decimal degrees, like -104.99",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-180,180)"),
    ],
    outputs: &[
        Field::new(
            "matches",
            "Coordinate systems",
            "Most local first",
            Kind::List {
                items: ROW,
                min: 0,
                max: MAX_ROWS,
            },
        ),
        Field::new(
            "count",
            "Found",
            "How many matched, before any cut",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        text("note", "Note", "How the match was made", 240),
    ],
    errors: &[gp_base::ErrorCode::InvalidInput],
    assets: &[spcs2022::ASSET_ID],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Code or name match against the CRS index from EPSG v13.102 and the SPCS2022 beta zones, or the area-of-use box of each",
    accuracy: "Areas of use are EPSG's and NGS's rectangular boxes, so near a zone line more than one zone covers a point",
    when_to_use: "Use this when you have coordinates and a code or a name but are not sure what they are, or when you need the right system for a place: give an EPSG code from a file’s metadata, a name like Colorado Central, or a point, and it lists the UTM zones, State Plane zones in meters and feet, and geographic systems that fit, with each one’s unit, whether it is current, legacy, or still beta, and which tool here converts to it.",
    limitations: "The index holds only what the converters here produce: geographic WGS 84, NAD83, NAD83(2011), and NAD27, Web Mercator, every UTM zone on WGS 84, NAD83, and NAD83(2011), and every SPCS83 zone on NAD83 and NAD83(2011), plus the SPCS2022 beta zones. Other EPSG systems, county and local grids among them, are not listed. A point search uses each system’s rectangular area of use, not the county or state line, so confirm a State Plane zone by county. Finding a system does not transform between datums; that is the datum tools.",
    references: &[EPSG, NGS_SPCS2022],
    examples: &[Example {
        id: "primary",
        title: "What covers downtown Denver",
        input: r#"{"lat":39.74,"lon":-104.99}"#,
        source: "add-geodesy-suite location scenario: UTM 13N on NAD83 and WGS 84, SPCS83 Colorado Central, and the SPCS2022 zones covering the point",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.zone-lookup",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.zone",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.transform",
            reason: "next",
        },
    ],
    sentence: "Found {count} coordinate {plural count \"system\" \"systems\"}.",
    limits: &[("batchRows", 100)],
    run: run_search,
    ..ToolDef::BLANK
};

/// Sort order: the most local systems first.
fn rank(kind: &str) -> u8 {
    match kind {
        "State Plane 1983" => 0,
        "State Plane 2022" => 1,
        "UTM" => 2,
        "Web Mercator" => 3,
        _ => 4,
    }
}

fn epsg_row(c: &Crs) -> (u8, String, Json) {
    let (kind, convert) = match c.kind {
        CrsKind::Geographic => (
            "Geographic",
            match c.code {
                // NAD27 and the original NAD83 move by the NADCON5 grids;
                // NAD83(2011) by the frame transformations.
                4267 | 4269 => "geodesy.datum.nadcon5",
                6318 => "geodesy.datum.transform",
                _ => "",
            },
        ),
        CrsKind::WebMercator => ("Web Mercator", "geodesy.projection.web-mercator-forward"),
        CrsKind::Utm { .. } => ("UTM", "geodesy.utm.forward"),
        CrsKind::Spcs83 { .. } => ("State Plane 1983", "geodesy.spcs.spcs83-forward"),
    };
    let unit = match c.unit {
        "deg" => "degrees",
        "m" => "meters",
        "ftUS" => "US survey feet",
        _ => "international feet",
    };
    let status = if c.code == 4267 { "legacy" } else { "current" };
    (
        rank(kind),
        format!("{:09}", c.code),
        Json::obj([
            ("code", Json::str(format!("EPSG:{}", c.code))),
            ("name", Json::str(c.name)),
            ("kind", Json::str(kind)),
            ("unit", Json::str(unit)),
            ("status", Json::str(status)),
            ("convert_with", Json::str(convert)),
        ]),
    )
}

fn run_search(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let query = ctx.text("query")?.unwrap_or_default();
    let query = query.trim().to_owned();
    let at = if ctx.is_set("lat") || ctx.is_set("lon") {
        Some(point::read(ctx, "lat", "lon")?)
    } else {
        None
    };
    if query.is_empty() && at.is_none() {
        return Err(
            ToolError::invalid("/query", "Give an EPSG code, a name, or a point.")
                .hint("Like 2232, Colorado Central, or a latitude and longitude"),
        );
    }
    let code = crs::epsg_code(&query);
    let words = crs::words(&query);
    let covers = |c: &Crs| at.is_none_or(|(lat, lon)| c.covers(lat, lon));
    // A system whose whole name was typed comes first, so "NAD83" finds the
    // datum itself before the hundreds of systems built on it.
    let exact = |name: &str| !query.is_empty() && name.eq_ignore_ascii_case(&query);
    let mut rows: Vec<(u8, String, Json)> = crs::REGISTRY
        .iter()
        .filter(|c| match code {
            Some(n) => c.code == n,
            None => crs::matches_words(&words, &c.search_text()),
        })
        .filter(|c| covers(c))
        .map(|c| {
            let mut row = epsg_row(c);
            if exact(c.name) {
                row.0 = 0;
                row.1.insert(0, ' ');
            }
            row
        })
        .collect();

    // SPCS2022: only when a name or a point could match a zone, so a plain
    // EPSG lookup does not fetch the zone file.
    let ngs_code = query.strip_prefix("NGS:SPCS2022:").unwrap_or(&query);
    let ngs = ngs_code.len() == 6 && ngs_code.bytes().all(|b| b.is_ascii_digit());
    if code.is_none() || ngs {
        let data = spcs2022::load(ctx)?;
        for z in &data.zones {
            let named = if ngs {
                z.code == ngs_code
            } else {
                crs::matches_words(
                    &words,
                    &format!("{} state plane spcs spcs2022 2022 {}", z.name, z.code),
                )
            };
            let inside = at.is_none_or(|(lat, lon)| spcs2022::contains(z, lat, lon));
            if named && inside {
                rows.push((
                    rank("State Plane 2022"),
                    z.code.clone(),
                    Json::obj([
                        ("code", Json::str(format!("NGS:SPCS2022:{}", z.code))),
                        ("name", Json::str(z.name.clone())),
                        ("kind", Json::str("State Plane 2022")),
                        ("unit", Json::str("meters")),
                        ("status", Json::str(z.status.clone())),
                        ("convert_with", Json::str("geodesy.spcs.spcs2022-forward")),
                    ]),
                ));
            }
        }
    }
    rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let count = rows.len();
    let how = match (code.is_some() || ngs, !words.is_empty(), at.is_some()) {
        (true, _, false) => "The system with that code.",
        (true, _, true) => "The system with that code, if its area of use covers the point.",
        (false, true, false) => "Systems whose name has every word you gave.",
        (false, true, true) => {
            "Systems whose name has every word you gave and whose area of use covers the point."
        }
        _ => {
            "Systems whose area of use covers the point, the most local first. Confirm a State Plane zone by county."
        }
    };
    let note = if count > MAX_ROWS {
        format!("{how} Showing the first {MAX_ROWS}; add words to narrow it.")
    } else if count == 0 {
        format!(
            "{how} Nothing matched; the index holds geographic, UTM, State Plane, and Web Mercator systems only."
        )
    } else {
        how.to_owned()
    };
    rows.truncate(MAX_ROWS);
    Ok(Json::obj(vec![
        (
            "matches",
            Json::Arr(rows.into_iter().map(|(_, _, j)| j).collect()),
        ),
        ("count", Json::Num(count as f64)),
        ("note", Json::str(note)),
    ]))
}

// ---------------------------------------------------------------- transform

const IOGP_G7_2: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "IOGP",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Concatenated operations: a conversion, a transformation, and a conversion applied in turn",
    url: "https://www.iogp.org/wp-content/uploads/2019/09/373-07-02.pdf",
};

const STEP: &[Field] = &[
    text("step", "Step", "What is done", 40),
    text("method", "Method", "How", 120),
    Field::new(
        "accuracy",
        "Accuracy",
        "Its stated uncertainty (1σ); 0 for an exact formula",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
];

const fn len(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}

pub static TRANSFORM: ToolDef = ToolDef {
    id: "geodesy.crs.transform",
    title: "Coordinate system to coordinate system",
    summary: "Converts coordinates from one EPSG coordinate system to another, such as State Plane to UTM, through every step between: the inverse projection, the datum and frame changes at an epoch, and the forward projection, each with its accuracy.",
    aliases: &[
        "convert between EPSG codes",
        "state plane to UTM",
        "CRS to CRS",
        "reproject coordinates",
        "cs2cs",
    ],
    keywords: &[
        "EPSG",
        "CRS",
        "transform",
        "reproject",
        "state plane",
        "UTM",
        "NAD83",
        "WGS 84",
        "datum",
        "epoch",
    ],
    inputs: &[
        text(
            "from",
            "From",
            "EPSG code of the coordinates you have, like 6427",
            24,
        )
        .required()
        .core(),
        text("to", "To", "EPSG code you want them in, like 32613", 24)
            .required()
            .core(),
        Field::new(
            "easting",
            "Easting",
            "For a projected system, in its unit unless you say, like 3,140,000",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
        Field::new(
            "northing",
            "Northing",
            "For a projected system, like 1,700,000",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "For a geographic system, like 39.74",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "For a geographic system, like -104.99",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-180,180)"),
        text(
            "epoch",
            "Epoch",
            "When the coordinates are for, needed when the frames differ: a date or decimal year, like 2026.0",
            40,
        ),
        len(
            "height",
            "Ellipsoid height",
            "Optional, like 1600 m; it barely moves a horizontal answer",
        ),
    ],
    outputs: &[
        text("coordinates", "Coordinates", "In the target system", 120),
        len("easting", "Easting", "In the target system's unit")
            .precision(Precision::Decimals(3))
            .optional(),
        len("northing", "Northing", "In the target system's unit")
            .precision(Precision::Decimals(3))
            .optional(),
        Field::new(
            "lat",
            "Latitude",
            "In a geographic target",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .optional(),
        Field::new(
            "lon",
            "Longitude",
            "In a geographic target",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .optional(),
        len(
            "accuracy",
            "Accuracy",
            "Root sum of squares of the steps' stated uncertainties (1σ)",
        )
        .precision(Precision::Decimals(3)),
        text("target", "Target system", "Its EPSG name", 120),
        Field::new(
            "steps",
            "Steps",
            "In order",
            Kind::List {
                items: STEP,
                min: 1,
                max: 20,
            },
        ),
    ],
    errors: &[
        gp_base::ErrorCode::InvalidInput,
        gp_base::ErrorCode::OutOfDomain,
    ],
    warnings: &[
        "REALIZATION_ASSUMED",
        "OUTSIDE_ZONE_EXTENT",
        "LEGACY_UNIT",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Inverse projection on the source system's ellipsoid, the frame path of the datum transformation tool at the epoch, and the forward projection, applied in turn",
    accuracy: "The projections are exact formulas; the frame steps carry their published uncertainties, summed as a root sum of squares. The coordinates' own accuracy is not included.",
    when_to_use: "Use this when coordinates come in one system and are needed in another and the two may sit on different datums: State Plane on NAD 83 (2011) to UTM on WGS 84 for a drone or GIS layer, UTM to geographic, or feet to meters between the same zone’s twins. Give both EPSG codes, the coordinates, and the epoch they are for, and it lists every step it took and how sure each is.",
    limitations: "It works between the systems the coordinate system finder lists on WGS 84 and NAD 83 (2011). NAD 27 and the original NAD 83 move by the NADCON5 grids, which this does not chain yet, so convert them with the NADCON5 tool first; the SPCS2022 beta zones sit on frames not in the chain. WGS 84 is taken as its current realization, G2296. Heights are ellipsoid heights and are not converted to or from orthometric heights. A point far outside the target zone is computed with a warning, and one the zone cannot hold is refused.",
    references: &[IOGP_G7_2, EPSG],
    examples: &[Example {
        id: "primary",
        title: "Colorado Central on NAD 83 (2011) to UTM 13N on WGS 84",
        input: r#"{"from":"6427","to":"32613","easting":"953000 m","northing":"515000 m","epoch":"2026.0","height":"1600 m"}"#,
        source: "add-geodesy-suite composite-path scenario: inverse projection, the frame steps at the epoch, and the forward projection, each with its accuracy",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.crs.search",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.transform",
            reason: "alternative",
        },
        Related {
            id: "geodesy.spcs.spcs83-inverse",
            reason: "alternative",
        },
    ],
    sentence: "In {target} that is {coordinates}, to about {accuracy} from the datum steps.",
    limits: &[("batchRows", 10_000)],
    run: run_transform,
    ..ToolDef::BLANK
};

/// The datum frame a system's coordinates are in, by its EPSG name.
fn frame_of(c: &Crs) -> Option<&'static str> {
    if c.name.starts_with("WGS 84") {
        Some("WGS84(G2296)")
    } else if c.name.starts_with("NAD83(2011)") {
        Some("NAD83(2011)")
    } else {
        None
    }
}

fn ellipsoid_of(frame: &str) -> gp_geo::ellipsoid::Ellipsoid {
    let id = if frame.starts_with("WGS84") {
        "wgs84"
    } else {
        "grs80"
    };
    *gp_geo::ellipsoid::CATALOG
        .iter()
        .find(|e| e.id == id)
        .expect("catalog ellipsoid")
}

fn unit_of(c: &Crs) -> &'static gp_base::units::Unit {
    gp_base::units::by_symbol(QT::Length, if c.unit == "deg" { "m" } else { c.unit })
        .expect("registered unit")
}

fn describe(c: &Crs) -> String {
    match c.kind {
        CrsKind::Geographic => format!("{} geographic coordinates", c.name),
        CrsKind::WebMercator => "Popular Visualisation Pseudo Mercator (EPSG 1024)".to_owned(),
        CrsKind::Utm { zone, north } => format!(
            "UTM zone {zone}{}, transverse Mercator (Krüger series)",
            if north { "N" } else { "S" }
        ),
        CrsKind::Spcs83 { fips, .. } => {
            let z = gp_geo::spcs::find(fips).expect("zone in the table");
            format!(
                "SPCS83 {} ({}), {}",
                z.short_name(),
                fips,
                match z.proj {
                    gp_geo::spcs::Proj::Tm { .. } => "transverse Mercator",
                    gp_geo::spcs::Proj::Lcc { .. } | gp_geo::spcs::Proj::Lcc1 { .. } =>
                        "Lambert conformal conic",
                    _ => "oblique Mercator",
                }
            )
        }
    }
}

fn system(ctx: &mut Ctx, name: &str) -> Result<&'static Crs, ToolError> {
    let at = format!("/{name}");
    let raw = ctx.text(name)?.expect("required");
    let code = crs::epsg_code(&raw).ok_or_else(|| {
        ToolError::invalid(&at, format!("\"{raw}\" is not an EPSG code."))
            .hint("Like 6427 or EPSG:32613; the coordinate system finder looks codes up by name")
    })?;
    let c = crs::by_code(code).ok_or_else(|| {
        ToolError::new(
            gp_base::ErrorCode::OutOfDomain,
            format!("EPSG:{code} is not one of the systems this tool converts."),
        )
        .at(&at)
        .hint("It works with the UTM, State Plane, Web Mercator, and geographic systems the coordinate system finder lists")
    })?;
    if frame_of(c).is_none() {
        return Err(ToolError::new(
            gp_base::ErrorCode::OutOfDomain,
            format!(
                "{} moves by the NADCON5 grids, which this tool does not chain yet.",
                c.name
            ),
        )
        .at(&at)
        .hint("Convert it to NAD 83 (2011) with the NADCON5 tool first"));
    }
    Ok(c)
}

/// A coordinate in the system's own unit: a bare number is taken in that unit.
fn coordinate(
    ctx: &mut Ctx,
    name: &str,
    unit: &'static gp_base::units::Unit,
) -> Result<f64, ToolError> {
    let bare = match ctx.raw(name) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().replace(',', "").parse::<f64>().ok(),
        _ => None,
    };
    match bare {
        Some(v) if v.is_finite() => Ok(gp_base::units::to_base(v, unit)),
        _ => Ok(ctx.req_quantity(name)?.base()),
    }
}

fn run_transform(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let src = system(ctx, "from")?;
    let dst = system(ctx, "to")?;
    let (f0, f1) = (
        frame_of(src).expect("checked"),
        frame_of(dst).expect("checked"),
    );
    if f0.starts_with("WGS84") || f1.starts_with("WGS84") {
        ctx.warnings.push(gp_base::error::Warning::new(
            "REALIZATION_ASSUMED",
            "EPSG's WGS 84 names several realizations; it was taken as the current one, WGS 84 (G2296).",
        ));
    }
    let h = ctx.quantity("height")?.map_or(0.0, |q| q.base());
    // 1. To latitude and longitude on the source system's ellipsoid.
    let (lat, lon) = match src.kind {
        CrsKind::Geographic => point::read(ctx, "lat", "lon")?,
        kind => {
            let u = unit_of(src);
            let (e, n) = (
                coordinate(ctx, "easting", u)?,
                coordinate(ctx, "northing", u)?,
            );
            let ell = ellipsoid_of(f0);
            let (la, lo) = match kind {
                CrsKind::Utm { zone, north } => {
                    gp_geo::utmups::utm_inverse(ell.a, ell.f, zone, north, e, n)
                }
                CrsKind::WebMercator => gp_geo::proj::WebMercator::inverse(e, n),
                CrsKind::Spcs83 { fips, fe, fn_ } => {
                    // This system's own false origin, not the metric zone's.
                    let z = gp_geo::spcs::find(fips).expect("zone");
                    let (zfe, zfn) = z.false_origin();
                    z.inverse(e - fe + zfe, n - fn_ + zfn)
                }
                CrsKind::Geographic => unreachable!(),
            };
            if !(la.is_finite() && lo.is_finite() && la.abs() <= 90.0) {
                return Err(ToolError::new(
                    gp_base::ErrorCode::OutOfDomain,
                    format!("Those coordinates are not on the {} grid.", src.name),
                )
                .at("/easting"));
            }
            (la, lo)
        }
    };
    let mut steps = Vec::new();
    let mut row = |step: &str, method: String, sigma: f64| {
        steps.push(Json::obj([
            ("step", Json::str(step)),
            ("method", Json::str(method)),
            (
                "accuracy",
                Q {
                    value: sigma,
                    unit: gp_base::units::by_symbol(QT::Length, "m").expect("m"),
                }
                .to_json(),
            ),
        ]));
    };
    if !matches!(src.kind, CrsKind::Geographic) {
        row("Inverse projection", describe(src), 0.0);
    }
    // 2. The frame path, at the epoch, through ECEF.
    let path = crate::datum::frame_path(f0, f1).expect("both frames are in the chain");
    let (mut lat2, mut lon2) = (lat, lon);
    let mut var = 0.0;
    if !path.is_empty() {
        let t = crate::datum::epoch(ctx, "epoch")?.ok_or_else(|| {
            ToolError::invalid(
                "/epoch",
                "These systems are on different frames, so the epoch is needed.",
            )
            .hint("A date or decimal year, like 2026-01-01 or 2026.0")
        })?;
        if !(1980.0..=2100.0).contains(&t) {
            return Err(ToolError::new(
                gp_base::ErrorCode::OutOfDomain,
                "The epoch must be between 1980 and 2100.",
            )
            .at("/epoch"));
        }
        let (e0, e1) = (ellipsoid_of(f0), ellipsoid_of(f1));
        let p = gp_geo::frames::to_ecef(&e0, lat.to_radians(), lon.to_radians(), h);
        let mut v = [p.0, p.1, p.2];
        for &(a, b, step, sigma) in &path {
            v = crate::datum::apply_step(a, b, step, v, t);
            var += sigma * sigma;
            row(
                "Frame transformation",
                format!("{} → {} at {t}: {}", a, b, crate::datum::step_method(step)),
                sigma,
            );
        }
        let (phi, lam, _) =
            gp_geo::frames::from_ecef(&e1, v[0], v[1], v[2]).expect("not the center");
        (lat2, lon2) = (phi.to_degrees(), gp_base::angle::wrap_lon(lam.to_degrees()));
    }
    // 3. Onto the target grid.
    if !dst.covers(lat2, lon2) {
        ctx.warnings.push(
            gp_base::error::Warning::new(
                "OUTSIDE_ZONE_EXTENT",
                format!(
                    "The point is outside the area of use of {}; distortion grows quickly outside it.",
                    dst.name
                ),
            )
            .at("/to"),
        );
    }
    let u = unit_of(dst);
    let sigma = var.sqrt();
    let mut out = Vec::new();
    let coordinates;
    match dst.kind {
        CrsKind::Geographic => {
            coordinates = format!("{lat2:.9}°, {lon2:.9}°");
            out.push(("coordinates", Json::str(coordinates)));
            out.push(("lat", ctx.out("lat", deg(lat2))));
            out.push(("lon", ctx.out("lon", deg(lon2))));
        }
        kind => {
            let (e, n) = match kind {
                CrsKind::Utm { zone, north } => {
                    let ell = ellipsoid_of(f1);
                    let tm = gp_geo::tm::Tm::new(ell.a, ell.f, gp_geo::utmups::UTM_K0);
                    let dl =
                        gp_base::angle::wrap_lon(lon2 - gp_geo::utmups::central_meridian(zone));
                    let (x, y, _, _) = tm.forward(lat2, dl);
                    (
                        x + gp_geo::utmups::UTM_FE,
                        y + if north {
                            0.0
                        } else {
                            gp_geo::utmups::UTM_FN_SOUTH
                        },
                    )
                }
                CrsKind::WebMercator => {
                    let g = gp_geo::proj::WebMercator::forward(lat2, lon2);
                    (g.e, g.n)
                }
                CrsKind::Spcs83 { fips, fe, fn_ } => {
                    let z = gp_geo::spcs::find(fips).expect("zone");
                    let (zfe, zfn) = z.false_origin();
                    let g = z.forward(lat2, lon2);
                    (g.e - zfe + fe, g.n - zfn + fn_)
                }
                CrsKind::Geographic => unreachable!(),
            };
            if !(e.is_finite() && n.is_finite()) {
                return Err(ToolError::new(
                    gp_base::ErrorCode::OutOfDomain,
                    format!(
                        "This point is too far outside {} to place on its grid.",
                        dst.name
                    ),
                )
                .at("/to"));
            }
            let m = gp_base::units::by_symbol(QT::Length, "m").expect("m");
            let (ej, nj) = (
                ctx.emit("easting", Q { value: e, unit: m }, u),
                ctx.emit("northing", Q { value: n, unit: m }, u),
            );
            let fmt = ctx.options.format;
            let show = |v: f64| {
                gp_base::display::quantity(
                    gp_base::units::convert(v, m, u),
                    u.symbol,
                    Precision::Decimals(3),
                    fmt,
                )
            };
            coordinates = format!("{} E, {} N", show(e), show(n));
            out.push(("coordinates", Json::str(coordinates)));
            out.push(("easting", ej));
            out.push(("northing", nj));
            row("Forward projection", describe(dst), 0.0);
        }
    }
    ctx.accuracy = Some(if path.is_empty() {
        "Exact: the two systems share a frame, so only projection formulas are applied.".to_owned()
    } else {
        format!(
            "About {:.1} cm (1σ), the root sum of squares of the frame steps' stated uncertainties; the projections are exact and the coordinates' own accuracy is not included.",
            sigma * 100.0
        )
    });
    out.push((
        "accuracy",
        ctx.out(
            "accuracy",
            Q {
                value: sigma,
                unit: gp_base::units::by_symbol(QT::Length, "m").expect("m"),
            },
        ),
    ));
    out.push(("target", Json::str(dst.name)));
    out.push(("steps", Json::Arr(steps)));
    Ok(Json::obj(out))
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: gp_base::units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}
