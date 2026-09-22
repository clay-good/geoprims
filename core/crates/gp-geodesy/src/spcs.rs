//! State plane tools (geodesy/projections spec, "State Plane Coordinate
//! Systems"): SPCS83 forward and inverse for all 124 zones, and zone lookup.
//! The math and the generated zone table live in gp-geo.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::point;
use gp_geo::spcs::{self, Zone};

const NGS_5: Reference = Reference {
    title: "State Plane Coordinate System of 1983, NOAA Manual NOS NGS 5",
    issuer: "Stem, J. E., National Geodetic Survey",
    year: 1990,
    edition: "NOAA Manual NOS NGS 5 (reprinted with corrections)",
    locator: "Zone constants (Appendix) and the Lambert and transverse Mercator mapping equations",
    url: "https://geodesy.noaa.gov/library/pdfs/NOAA_Manual_NOS_NGS_0005.pdf",
};
const EPSG_G7_2: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Sections 3.2.1.1 (Lambert Conic Conformal 2SP, EPSG 9802) and 3.2.4 (Hotine Oblique Mercator variant A, EPSG 9812)",
    url: "https://www.iogp.org/bookstore/product/coordinate-conversions-and-transformation-including-formulas/",
};
const KARNEY_TM: Reference = Reference {
    title: "Transverse Mercator with an accuracy of a few nanometers",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2011,
    edition: "Vol. 85, No. 8",
    locator: "pp. 475-485 (6th-order Krüger series)",
    url: "https://doi.org/10.1007/s00190-011-0445-3",
};

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");

const ZONE: Field = Field::new(
    "zone",
    "Zone",
    "NGS code like 3702, or a name like Pennsylvania South",
    Kind::Text { max_len: 60 },
);
const UNIT: Field = Field::new(
    "unit",
    "Unit",
    "legal (the zone's feet unit where one is defined, else meters; default), m, ft, or ftUS",
    Kind::Choice(&["legal", "m", "ft", "ftUS"]),
);

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const fn text(name: &'static str, title: &'static str, help: &'static str, n: usize) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: n })
}

fn len_unit(sym: &str) -> &'static Unit {
    units::by_symbol(QT::Length, sym).expect("registered unit")
}

/// The output unit the caller chose for a zone.
fn chosen_unit(ctx: &Ctx, z: &Zone) -> Result<&'static Unit, ToolError> {
    Ok(match ctx.choice("unit")?.unwrap_or("legal") {
        "legal" => len_unit(z.feet.unwrap_or("m")),
        u => len_unit(u),
    })
}

fn zone_by_key(key: &str) -> Result<&'static Zone, ToolError> {
    spcs::find(key).ok_or_else(|| {
        ToolError::invalid("/zone", format!("{key} is not an SPCS83 zone."))
            .hint("Use the NGS code (like 3702) or the name (like Pennsylvania South). The zone lookup tool lists zones by state or point.")
    })
}

fn zone_list(zs: &[&Zone]) -> String {
    zs.iter()
        .map(|z| format!("{} ({})", z.short_name(), z.fips))
        .collect::<Vec<_>>()
        .join(", ")
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

fn warn_outside(ctx: &mut Ctx, z: &Zone, lat: f64, lon: f64) {
    if !z.contains(lat, lon) {
        let others = spcs::candidates(lat, lon);
        let hint = if others.is_empty() {
            "No SPCS83 zone covers this point.".to_owned()
        } else {
            format!("Zones covering it: {}.", zone_list(&others))
        };
        ctx.warnings.push(
            Warning::new(
                "OUTSIDE_ZONE_EXTENT",
                format!("The point is outside the {} zone's area of use; distortion grows quickly outside it. {hint}", z.short_name()),
            )
            .at("/zone"),
        );
    }
}

const SPCS_OUTPUTS: &[Field] = &[
    qty("easting", "Easting", "In the chosen unit", QT::Length, "m")
        .precision(Precision::Decimals(3)),
    qty(
        "northing",
        "Northing",
        "In the chosen unit",
        QT::Length,
        "m",
    )
    .precision(Precision::Decimals(3)),
    text("zone_name", "Zone", "SPCS83 zone name", 60),
    text("zone_code", "NGS zone code", "Like 3702", 4),
    Field::new(
        "epsg",
        "EPSG code",
        "The meter version of the zone",
        Kind::Number {
            min: 1.0,
            max: 99999.0,
        },
    )
    .precision(Precision::Decimals(0)),
    qty(
        "convergence",
        "Grid convergence",
        "Bearing of grid north clockwise from true north",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(6)),
    Field::new(
        "scale_factor",
        "Point scale factor",
        "Grid distance / ellipsoid distance",
        Kind::Number { min: 0.0, max: 2.0 },
    )
    .precision(Precision::Decimals(8)),
];

// ---------------------------------------------------------------- forward

pub static FORWARD: ToolDef = ToolDef {
    id: "geodesy.spcs.spcs83-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to state plane (SPCS83)",
    summary: "Converts NAD83 latitude and longitude to State Plane Coordinate System of 1983 easting and northing in any of the 124 zones, in meters, international feet, or US survey feet, with convergence and scale factor.",
    aliases: &[
        "state plane converter",
        "lat long to state plane",
        "SPCS83 converter",
        "state plane coordinates calculator",
    ],
    keywords: &[
        "state plane",
        "SPCS",
        "SPCS83",
        "NAD83",
        "easting",
        "northing",
        "US survey feet",
        "Lambert",
        "transverse Mercator",
        "FIPS zone",
    ],
    inputs: &[LAT, LON, ZONE.core(), UNIT.core()],
    outputs: SPCS_OUTPUTS,
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "OUTSIDE_ZONE_EXTENT",
        "LEGACY_UNIT",
        "INPUT_NORMALIZED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "SPCS83 zone definitions from the EPSG dataset on GRS 80: transverse Mercator (Krüger series), Lambert Conic Conformal (2SP), and Hotine Oblique Mercator (variant A) for Alaska zone 1",
    accuracy: "Matches PROJ within 0.05 µm on 2,480 points across all 124 zones; inverse round trips within 1e-12°. The input must already be NAD83; this tool does not change datums.",
    when_to_use: "Use this for survey and engineering work in the United States, where plans, plats, and agency data are on State Plane coordinates: it converts NAD 83 latitude and longitude to easting and northing in any of the 124 zones, in meters, international feet, or US survey feet, with the convergence and scale factor the work needs.",
    limitations: "The input must already be NAD 83: passing WGS 84 or ITRF coordinates puts the result off by up to a meter or two, because this projects rather than transforms datums. Feet come in two lengths and the legal one differs by state, which the result states. A point outside its zone's extent still converts, with a warning, but the distortion grows.",
    references: &[NGS_5, EPSG_G7_2, KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh in Pennsylvania South (US survey feet)",
        input: r#"{"lat":40.446111,"lon":-79.982222,"zone":"3702"}"#,
        source: "add-geodesy-suite scenario: 1,347,294.025 ftUS E, 413,222.374 ftUS N (±0.003 ftUS)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("easting", "easting")],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.spcs83-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.zone-lookup",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: "In {zone_name} ({zone_code}) the point is at easting {easting}, northing {northing}.{warn OUTSIDE_ZONE_EXTENT} It is outside this zone's area.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let z = match ctx.text("zone")? {
        Some(k) => zone_by_key(&k)?,
        None => match spcs::candidates(lat, lon).as_slice() {
            [one] => one,
            [] => {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "No SPCS83 zone covers this point.",
                )
                .at("/lat"));
            }
            many => {
                return Err(ToolError::invalid(
                    "/zone",
                    format!(
                        "Several zones cover this point: {}. Choose one (by county).",
                        zone_list(many)
                    ),
                ));
            }
        },
    };
    warn_outside(ctx, z, lat, lon);
    let g = z.forward(lat, lon);
    let u = chosen_unit(ctx, z)?;
    let m = len_unit("m");
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Zone",
            "the state plane zone, each with its own projection and origin",
            format!("{}°, {}°", n(lat, 6), n(lon, 6)),
            format!("{} ({})", z.short_name(), z.fips),
        );
        ctx.step(
            "Scale factor",
            "how much the zone's projection stretches distance here",
            format!("at {}°, {}°", n(lat, 6), n(lon, 6)),
            n(g.k, 8),
        );
        // In the unit the card shows, which is the zone's own by default.
        let east = Q {
            value: g.e,
            unit: m,
        }
        .to(u);
        ctx.step(
            "Easting",
            "x from that projection, plus the zone's false easting",
            format!("{}° east in {}", n(lon, 6), z.short_name()),
            // The same formatter the card uses, so the unit reads the same way.
            gp_base::display::quantity(east, u.symbol, Precision::Decimals(3), fmt),
        );
    }
    Ok(Json::obj(vec![
        (
            "easting",
            ctx.emit(
                "easting",
                Q {
                    value: g.e,
                    unit: m,
                },
                u,
            ),
        ),
        (
            "northing",
            ctx.emit(
                "northing",
                Q {
                    value: g.n,
                    unit: m,
                },
                u,
            ),
        ),
        ("zone_name", Json::str(z.short_name())),
        ("zone_code", Json::str(z.fips)),
        ("epsg", Json::Num(z.epsg as f64)),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
    ]))
}

// ---------------------------------------------------------------- inverse

pub static INVERSE: ToolDef = ToolDef {
    id: "geodesy.spcs.spcs83-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "State plane (SPCS83) to latitude and longitude",
    summary: "Converts SPCS83 easting and northing in any of the 124 zones back to NAD83 latitude and longitude, with convergence and scale factor.",
    aliases: &[
        "state plane to lat long",
        "SPCS83 inverse",
        "state plane to geographic",
    ],
    keywords: &[
        "state plane",
        "SPCS83",
        "NAD83",
        "easting",
        "northing",
        "latitude",
        "longitude",
    ],
    inputs: &[
        ZONE.required().core(),
        qty(
            "easting",
            "Easting",
            "Like 1347294.025 ftUS; a bare number is in the unit field's unit",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qty(
            "northing",
            "Northing",
            "Like 413222.374 ftUS",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        UNIT.core(),
    ],
    outputs: &[
        qty("lat", "Latitude", "NAD83, north positive", QT::Angle, "deg")
            .precision(Precision::Decimals(9))
            .angle_range("[-90,90]"),
        qty("lon", "Longitude", "NAD83, east positive", QT::Angle, "deg")
            .precision(Precision::Decimals(9))
            .angle_range("[-180,180)"),
        text("zone_name", "Zone", "SPCS83 zone name", 60),
        text("zone_code", "NGS zone code", "Like 3702", 4),
        qty(
            "convergence",
            "Grid convergence",
            "Bearing of grid north clockwise from true north",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "scale_factor",
            "Point scale factor",
            "Grid distance / ellipsoid distance",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(8)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "OUTSIDE_ZONE_EXTENT",
        "UNIT_ASSUMED",
        "LEGACY_UNIT",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Inverse of the SPCS83 zone projection on GRS 80",
    accuracy: "Round trips within 1e-12° of latitude and longitude.",
    when_to_use: "Use this when plan coordinates have to become geographic ones: State Plane easting and northing in any of the 124 zones back to NAD 83 latitude and longitude, with the convergence and scale factor at that point. It is what turns a plan, a plat, or an agency dataset back into coordinates you can map, compare with GPS, or hand to a tool that works in latitude and longitude.",
    limitations: "The zone and the unit have to match the ones the coordinates were computed in; the same numbers in the wrong zone or in feet rather than meters land far away without looking wrong. The result is NAD 83, not WGS 84, and the difference reaches a meter or more. The convergence it reports is the angle between grid north and true north at that point, which is what a bearing taken from the plan has to be corrected by before it is a true bearing.",
    references: &[NGS_5, EPSG_G7_2, KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "Pennsylvania South coordinates in US survey feet",
        input: r#"{"zone":"3702","easting":"1347294.025 ftUS","northing":"413222.374 ftUS"}"#,
        source: "Inverse of the add-geodesy-suite Pennsylvania South scenario: about 40.446111°, -79.982222°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The point is at {lat}, {lon} (NAD83).",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let key = ctx.text("zone")?.unwrap_or_default();
    let z = zone_by_key(&key)?;
    let u = chosen_unit(ctx, z)?;
    let e = ctx
        .quantity_or("easting", Some(u))?
        .ok_or_else(|| ToolError::invalid("/easting", "Easting is required."))?;
    let n = ctx
        .quantity_or("northing", Some(u))?
        .ok_or_else(|| ToolError::invalid("/northing", "Northing is required."))?;
    if u.symbol == "ftUS" {
        ctx.warnings.push(Warning::new(
            "LEGACY_UNIT",
            "The US survey foot was deprecated by NIST and NOAA on January 1, 2023. Use it only for legacy data.",
        ));
    }
    let (lat, lon) = z.inverse(e.base(), n.base());
    if !lat.is_finite() || !lon.is_finite() || lat.abs() > 90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "These coordinates are far outside the zone.",
        )
        .at("/easting"));
    }
    let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    warn_outside(ctx, z, lat, lon);
    let g = z.forward(lat, lon);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let nn = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Zone",
            "the zone whose projection and origin these coordinates belong to",
            format!("{} m E, {} m N", nn(e.base(), 3), nn(n.base(), 3)),
            format!("{} ({})", z.short_name(), z.fips),
        );
        ctx.step(
            "Scale factor",
            "how much that projection stretched distance where this lands",
            format!("at {} m E, {} m N", nn(e.base(), 3), nn(n.base(), 3)),
            nn(g.k, 8),
        );
        ctx.step(
            "Latitude",
            "φ from the zone's projection, run backwards",
            format!("{} m N in {}", nn(n.base(), 3), z.short_name()),
            format!("{}°", nn(lat, 7)),
        );
    }
    Ok(Json::obj(vec![
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("zone_name", Json::str(z.short_name())),
        ("zone_code", Json::str(z.fips)),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
    ]))
}

// ---------------------------------------------------------------- lookup

const ZONE_ROW: &[Field] = &[
    text("zone_code", "NGS code", "Like 3702", 4),
    text("zone_name", "Zone", "Name", 60),
    Field::new(
        "epsg",
        "EPSG",
        "Meter version",
        Kind::Number {
            min: 1.0,
            max: 99999.0,
        },
    )
    .precision(Precision::Decimals(0)),
    text(
        "projection",
        "Projection",
        "Lambert, transverse Mercator, or oblique Mercator",
        40,
    ),
    text(
        "feet",
        "Feet unit",
        "US survey feet, international feet, or none defined",
        40,
    ),
];

pub static LOOKUP: ToolDef = ToolDef {
    id: "geodesy.spcs.zone-lookup",
    title: "State plane zone lookup",
    summary: "Finds SPCS83 zones by state or zone name, or the zones whose area of use covers a point, with each zone's NGS code, EPSG code, projection, and feet unit.",
    aliases: &[
        "what state plane zone am I in",
        "state plane zone finder",
        "SPCS zone lookup",
    ],
    keywords: &["state plane", "zone", "FIPS", "SPCS83", "county", "EPSG"],
    inputs: &[
        text("query", "State or zone", "Like Colorado or Texas South", 60).core(),
        Field::new(
            "lat",
            "Latitude",
            "Or a point: decimal degrees, like 39.74",
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
            "zones",
            "Zones",
            "Matching zones",
            Kind::List {
                items: ZONE_ROW,
                min: 0,
                max: 124,
            },
        ),
        Field::new(
            "count",
            "Count",
            "Number of zones",
            Kind::Number {
                min: 0.0,
                max: 124.0,
            },
        )
        .precision(Precision::Decimals(0)),
        text("note", "Note", "How the match was made", 200),
    ],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "Name match against the SPCS83 zone table, or the EPSG area-of-use box of each zone",
    accuracy: "Point lookup uses each zone's bounding box, so near a zone line more than one zone can match: confirm with the county list for the zone.",
    references: &[NGS_5],
    examples: &[Example {
        id: "primary",
        title: "Zones in Colorado",
        input: r#"{"query":"Colorado"}"#,
        source: "NOAA Manual NOS NGS 5: Colorado North (0501), Central (0502), and South (0503)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "geodesy.spcs.spcs83-forward",
        reason: "next",
    }],
    sentence: "Found {count} {plural count \"zone\" \"zones\"}.",
    limits: &[("batchRows", 1_000)],
    run: run_lookup,
    ..ToolDef::BLANK
};

fn run_lookup(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (zones, note): (Vec<&Zone>, &str) = if ctx.is_set("lat") || ctx.is_set("lon") {
        let (lat, lon) = point::read(ctx, "lat", "lon")?;
        (
            spcs::candidates(lat, lon),
            "Zones whose area-of-use box covers the point. Confirm the zone by county.",
        )
    } else {
        let q = ctx.text("query")?.unwrap_or_default().to_ascii_lowercase();
        if q.trim().is_empty() {
            return Err(
                ToolError::invalid("/query", "Give a state or zone name, or a point.")
                    .hint("Example: Colorado"),
            );
        }
        let words: Vec<&str> = q.split_whitespace().collect();
        (
            spcs::ZONES
                .iter()
                .filter(|z| {
                    let n = z.name.to_ascii_lowercase();
                    z.fips == q.trim()
                        || words
                            .iter()
                            .all(|w| n.split(|c: char| !c.is_alphanumeric()).any(|t| t == *w))
                })
                .collect(),
            "Zones whose name matches every word.",
        )
    };
    let rows = zones
        .iter()
        .map(|z| {
            Json::obj([
                ("zone_code", Json::str(z.fips)),
                ("zone_name", Json::str(z.short_name())),
                ("epsg", Json::Num(z.epsg as f64)),
                (
                    "projection",
                    Json::str(match z.proj {
                        spcs::Proj::Tm { .. } => "transverse Mercator",
                        spcs::Proj::Lcc { .. } => "Lambert conformal conic",
                        spcs::Proj::OmercA { .. } => "oblique Mercator",
                    }),
                ),
                (
                    "feet",
                    Json::str(match z.feet {
                        Some("ftUS") => "US survey feet",
                        Some(_) => "international feet",
                        None => "none defined (meters)",
                    }),
                ),
            ])
        })
        .collect();
    Ok(Json::obj(vec![
        ("zones", Json::Arr(rows)),
        ("count", Json::Num(zones.len() as f64)),
        ("note", Json::str(note)),
    ]))
}

// ---------------------------------------------------------------- arc to chord

const NGS_5_T_T: Reference = Reference {
    locator: "The (t − T) arc-to-chord correction for Lambert and transverse Mercator zones",
    ..NGS_5
};

pub static ARC_TO_CHORD: ToolDef = ToolDef {
    id: "geodesy.projection.arc-to-chord",
    title: "Arc-to-chord correction (t − T)",
    summary: "The small angle between a line's straight grid bearing and its curved projected geodesic, at each end, in UTM or a State Plane zone, with the grid and ellipsoid distances.",
    aliases: &[
        "arc to chord correction",
        "t minus T",
        "second term correction",
        "grid bearing correction",
    ],
    keywords: &[
        "arc-to-chord",
        "t-T",
        "grid bearing",
        "convergence",
        "state plane",
        "UTM",
        "Lambert",
        "transverse Mercator",
    ],
    inputs: &[
        point::lat_field("lat1", "From latitude"),
        point::lon_field("lon1", "From longitude"),
        point::lat_field("lat2", "To latitude"),
        point::lon_field("lon2", "To longitude"),
        Field::new(
            "grid",
            "Grid",
            "utm (default, the From point's zone) or spcs",
            Kind::Choice(&["utm", "spcs"]),
        )
        .core(),
        Field::new(
            "zone",
            "Zone",
            "SPCS83 code or name, like 3702 or Pennsylvania South; or a UTM zone number to force one",
            Kind::Text { max_len: 60 },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "t_minus_t_from",
            "t − T at the From end",
            "Grid bearing of the chord minus the projected geodesic's",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(3))
        .angle_range("unbounded"),
        Field::new(
            "t_minus_t_to",
            "t − T at the To end",
            "The same, looking back from the To end",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(3))
        .angle_range("unbounded"),
        Field::new(
            "grid_bearing",
            "Grid bearing t",
            "Of the straight line between the grid coordinates",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7))
        .angle_range("[0,360)"),
        Field::new(
            "projected_bearing",
            "Projected geodesic bearing T",
            "Geodesic azimuth minus convergence",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7))
        .angle_range("[0,360)"),
        Field::new(
            "grid_distance",
            "Grid distance",
            "Between the grid coordinates",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "ellipsoid_distance",
            "Ellipsoid distance",
            "Along the geodesic",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "grid_used",
            "Grid",
            "The zone the line is projected in",
            Kind::Text { max_len: 60 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "t = atan2(ΔE, ΔN) of the projected end points; T = geodesic azimuth − grid convergence at that end (Karney geodesic on WGS 84 for UTM, GRS 80 for SPCS83); t − T at each end. Exact for the projection, with no series approximation",
    accuracy: "Exact to about 1e-6″; the classic formulas agree to about 0.01″ on lines of a few kilometers",
    references: &[NGS_5_T_T, KARNEY_TM, EPSG_G7_2],
    examples: &[Example {
        id: "primary",
        title: "A 10 km line in Pennsylvania South",
        input: r#"{"lat1":40.44,"lon1":-79.99,"lat2":40.52,"lon2":-79.91,"grid":"spcs","zone":"3702"}"#,
        source: "NGS Manual 5 (t − T) definition, evaluated exactly",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
    ],
    sentence: "The chord is {t_minus_t_from} off the projected geodesic at the From end.",
    limits: &[("batchRows", 10_000)],
    run: run_arc_to_chord,
    ..ToolDef::BLANK
};

fn run_arc_to_chord(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let (la2, lo2) = point::read(ctx, "lat2", "lon2")?;
    let zone_text = ctx.text("zone")?;
    let spcs_grid = ctx.choice("grid")? == Some("spcs");
    // (E, N, convergence) at each end, the ellipsoid, and the zone's name.
    let (p1, p2, g, used) = if spcs_grid {
        let key = zone_text.ok_or_else(|| {
            ToolError::invalid(
                "/zone",
                "Give the SPCS83 zone, like 3702 or Pennsylvania South.",
            )
        })?;
        let z = zone_by_key(&key)?;
        let (a, b) = (z.forward(la1, lo1), z.forward(la2, lo2));
        (
            (a.e, a.n, a.convergence),
            (b.e, b.n, b.convergence),
            Geodesic::new(spcs::GRS80_A, spcs::GRS80_F),
            format!("SPCS83 {} ({})", z.short_name(), z.fips),
        )
    } else {
        if !gp_geo::utmups::in_utm_domain(la1) || !gp_geo::utmups::in_utm_domain(la2) {
            return Err(
                ToolError::new(ErrorCode::OutOfDomain, "UTM covers 80° S to 84° N.").at("/lat1"),
            );
        }
        let zone = match zone_text {
            Some(t) => {
                let z: u8 = t
                    .trim()
                    .parse()
                    .ok()
                    .filter(|z| (1..=60).contains(z))
                    .ok_or_else(|| {
                        ToolError::invalid("/zone", "A UTM zone is a number from 1 to 60.")
                    })?;
                z
            }
            None => gp_geo::utmups::standard_zone(la1, lo1),
        };
        let (a0, f0) = (6_378_137.0, 1.0 / 298.257_223_563);
        let a = gp_geo::utmups::utm_forward(a0, f0, la1, lo1, zone);
        let b = gp_geo::utmups::utm_forward(a0, f0, la2, lo2, zone);
        (
            (a.easting, a.northing, a.convergence),
            (b.easting, b.northing, b.convergence),
            Geodesic::wgs84(),
            format!("UTM zone {zone}"),
        )
    };
    let (s12, az1, az2, _): (f64, f64, f64, f64) = g.inverse(la1, lo1, la2, lo2);
    if s12 == 0.0 {
        return Err(ToolError::invalid("/lat2", "The two points are the same."));
    }
    let wrap = |x: f64| {
        let r = (x + 180.0).rem_euclid(360.0) - 180.0;
        if r == -180.0 { 180.0 } else { r }
    };
    let t1 = libm::atan2(p2.0 - p1.0, p2.1 - p1.1).to_degrees();
    let t2 = libm::atan2(p1.0 - p2.0, p1.1 - p2.1).to_degrees();
    let big_t1 = az1 - p1.2;
    let big_t2 = az2 + 180.0 - p2.2;
    let (d1, d2) = (wrap(t1 - big_t1) * 3600.0, wrap(t2 - big_t2) * 3600.0);
    let grid_d = libm::hypot(p2.0 - p1.0, p2.1 - p1.1);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| gp_base::display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Grid bearing t",
            "atan2(ΔE, ΔN) between the projected ends",
            format!("ΔE {} m, ΔN {} m", n(p2.0 - p1.0, 3), n(p2.1 - p1.1, 3)),
            format!("{}°", n(t1.rem_euclid(360.0), 7)),
        );
        ctx.step(
            "t − T",
            "t − (geodesic azimuth − convergence)",
            format!(
                "{}° − ({}° − {}°)",
                n(t1.rem_euclid(360.0), 7),
                n(az1, 7),
                n(p1.2, 7)
            ),
            gp_base::display::quantity(d1, "arcsec", Precision::Decimals(3), fmt),
        );
    }
    let asec = units::by_symbol(QT::Angle, "arcsec").expect("arcsec");
    let m = units::by_symbol(QT::Length, "m").expect("m");
    Ok(Json::obj(vec![
        (
            "t_minus_t_from",
            ctx.out(
                "t_minus_t_from",
                Q {
                    value: d1,
                    unit: asec,
                },
            ),
        ),
        (
            "t_minus_t_to",
            ctx.out(
                "t_minus_t_to",
                Q {
                    value: d2,
                    unit: asec,
                },
            ),
        ),
        (
            "grid_bearing",
            ctx.out("grid_bearing", deg(t1.rem_euclid(360.0))),
        ),
        (
            "projected_bearing",
            ctx.out("projected_bearing", deg(big_t1.rem_euclid(360.0))),
        ),
        (
            "grid_distance",
            ctx.out(
                "grid_distance",
                Q {
                    value: grid_d,
                    unit: m,
                },
            ),
        ),
        (
            "ellipsoid_distance",
            ctx.out(
                "ellipsoid_distance",
                Q {
                    value: s12,
                    unit: m,
                },
            ),
        ),
        ("grid_used", Json::str(used)),
    ]))
}
