//! Geodesy: coordinate parsing and formatting, UTM, UPS, and MGRS
//! (add-geodesy-suite). The math lives in gp-geo so other modules share it.

pub mod datum;
pub mod frames;
pub mod geoid;
pub mod gridref;
pub mod magnetic;
pub mod spcs;

use gp_base::ErrorCode;
use gp_base::angle::wrap_lon;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::dms::{self, Axis, Style};
use gp_geo::ellipsoid::{self, Ellipsoid};
use gp_geo::{mgrs, point, utmups};

pub(crate) const KARNEY_TM: Reference = Reference {
    title: "Transverse Mercator with an accuracy of a few nanometers",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2011,
    edition: "Vol. 85, No. 8",
    locator: "pp. 475-485 (6th-order Krüger series)",
    url: "https://doi.org/10.1007/s00190-011-0445-3",
};
pub(crate) const NGA_UTM: Reference = Reference {
    title: "The Universal Grids and the Transverse Mercator and Polar Stereographic Map Projections, NGA.SIG.0012",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2014,
    edition: "NGA.SIG.0012_2.0.0_UTMUPS",
    locator: "Sections 3 (UTM zones, Norway and Svalbard exceptions) and 4 (UPS)",
    url: "https://earth-info.nga.mil/php/download.php?file=coord-utmups",
};
const NGA_MGRS: Reference = Reference {
    title: "Military Grid Reference System, NGA.STND.0037",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2014,
    edition: "NGA.STND.0037_2.0.0_GRIDS",
    locator: "MGRS lettering (AA scheme) and truncation",
    url: "https://earth-info.nga.mil/php/download.php?file=coord-grids",
};
const GEOGRAPHICLIB_MGRS: Reference = Reference {
    title: "GeographicLib MGRS and UTMUPS classes",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2022,
    edition: "GeographicLib 2.x",
    locator: "MGRS.cpp: band tolerance, square validation, UPS lettering",
    url: "https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1MGRS.html",
};
const DMS_REF: Reference = Reference {
    title: "Geographic Point Location, ISO 6709",
    issuer: "International Organization for Standardization",
    year: 2022,
    edition: "ISO 6709:2022",
    locator: "Annex D (degrees, minutes, and seconds notation)",
    url: "https://www.iso.org/standard/75147.html",
};

fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

fn meters(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");
const LL_P: Precision = Precision::Decimals(7);
const M_P: Precision = Precision::Decimals(3);
const PITTSBURGH: &str = r#"{"lat":40.446111,"lon":-79.982222}"#;

const fn lat_out(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Degrees, north positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(LL_P)
    .angle_range("[-90,90]")
}

const fn lon_out(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Degrees, east positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(LL_P)
    .angle_range("[-180,180)")
}

// ---------------------------------------------------------------- parse

pub static PARSE: ToolDef = ToolDef {
    id: "geodesy.parse.coordinates",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Read any coordinate",
    summary: "Reads a coordinate in almost any notation (decimal, DMS, DDM, packed aviation, labeled, MGRS, or UTM) and reports what it assumed.",
    aliases: &[
        "coordinate parser",
        "DMS to decimal",
        "coordinate converter",
    ],
    keywords: &[
        "DMS",
        "degrees minutes seconds",
        "decimal degrees",
        "parse",
        "coordinates",
        "MGRS",
        "UTM",
    ],
    inputs: &[Field::new(
        "text",
        "Coordinate",
        "Like 40°26'46\"N 79°58'56\"W, 402646N0795856W, or 17TNE8630977770",
        Kind::Text { max_len: 200 },
    )
    .required()
    .core()],
    outputs: &[
        lat_out("lat", "Latitude"),
        lon_out("lon", "Longitude"),
        Field::new(
            "notation",
            "Notation",
            "What the text was read as",
            Kind::Text { max_len: 20 },
        ),
        Field::new(
            "dms",
            "DMS",
            "Degrees, minutes, and seconds",
            Kind::Text { max_len: 60 },
        ),
    ],
    warnings: &[
        "AMBIGUOUS_INPUT",
        "INPUT_NORMALIZED",
        "BAND_ADJUSTED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "geoprims coordinate grammar (ISO 6709 notations, NGA MGRS)",
    accuracy: "Exact: every notation converts by exact arithmetic",
    when_to_use: "Use this when a coordinate arrives as text and you do not want to hand-convert it: decimal degrees, degrees and minutes, degrees, minutes and seconds, packed aviation form, labeled pairs, MGRS, or UTM. It reports what it read, in your chosen output form, and says which assumptions it had to make.",
    limitations: "Text can be genuinely ambiguous — which number is the latitude, which hemisphere an unsigned pair means, whether a comma is a decimal mark — and this tool reports the ambiguity and its alternatives instead of guessing. It cannot know the datum of a bare pair: coordinates are treated as WGS 84 unless the notation itself carries one.",
    references: &[DMS_REF, NGA_MGRS],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh in degrees, minutes, and seconds",
        input: r#"{"text":"40°26'46\"N 79°58'56\"W"}"#,
        source: "add-geodesy-suite parsing scenario: 40.446111…, -79.982222…",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.parse.format",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "next",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon} ({notation}).{warn AMBIGUOUS_INPUT} Check the assumption noted.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_parse,
    ..ToolDef::BLANK
};

fn run_parse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text = ctx.text("text")?.expect("required");
    let wgs = ellipsoid::CATALOG[0];
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (lat, lon, notation) = if looks_like_mgrs(&compact) {
        let d = mgrs::decode(&text, wgs.a, wgs.f)
            .map_err(|e| ToolError::invalid("/text", format!("This looks like MGRS, but {e}.")))?;
        if d.band_adjusted {
            ctx.warnings.push(Warning::new(
                "BAND_ADJUSTED",
                format!(
                    "The latitude band {} was adjusted: the point is on a band boundary.",
                    d.band
                ),
            ));
        }
        let (la, lo) = grid_center(&d, wgs);
        (la, lo, "MGRS")
    } else if let Some(u) = parse_utm_text(&text) {
        let (zone, north, e, n) = u.map_err(|e| ToolError::invalid("/text", e))?;
        let (la, lo) = utmups::utm_inverse(wgs.a, wgs.f, zone, north, e, n);
        (la, lo, "UTM")
    } else {
        let p = dms::parse_pair(&text).map_err(|e| ToolError::invalid("/text", format!("{e}.")))?;
        if let Some(a) = p.ambiguity {
            ctx.warnings
                .push(Warning::new("AMBIGUOUS_INPUT", a).at("/text"));
        }
        let lon = if p.lon_normalized {
            let w = wrap_lon(p.lon);
            ctx.warnings.push(
                Warning::new(
                    "INPUT_NORMALIZED",
                    format!(
                        "Longitude was normalized to {}.",
                        gp_base::num::format_f64(w).unwrap_or_default()
                    ),
                )
                .at("/text"),
            );
            w
        } else {
            p.lon
        };
        (p.lat, lon, p.notation)
    };
    let dms_text = format!(
        "{} {}",
        dms::format(lat, Axis::Lat, Style::Dms, 3, true),
        dms::format(lon, Axis::Lon, Style::Dms, 3, true)
    );
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        // A parser's work is what it took the text to be, and what it read.
        ctx.step(
            "Notation",
            "which way the position was written",
            text.trim().to_owned(),
            notation.to_owned(),
        );
        ctx.step(
            "Longitude",
            "read from that notation, east positive",
            text.trim().to_owned(),
            format!("{}°", n(lon, 7)),
        );
        ctx.step(
            "Latitude",
            "read from that notation, north positive",
            text.trim().to_owned(),
            format!("{}°", n(lat, 7)),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("notation", Json::str(notation)),
        ("dms", Json::str(dms_text)),
    ]))
}

fn looks_like_mgrs(s: &str) -> bool {
    let u = s.to_ascii_uppercase();
    let b = u.as_bytes();
    let nz = b.iter().take_while(|c| c.is_ascii_digit()).count();
    nz <= 2
        && b.len() >= nz + 3
        && b[nz..nz + 3].iter().all(u8::is_ascii_alphabetic)
        && b[nz + 3..].iter().all(u8::is_ascii_digit)
}

/// `17N 586309 4477770` or `17T 586309E 4477770N` (zone, hemisphere or band, easting, northing).
fn parse_utm_text(s: &str) -> Option<Result<(u8, bool, f64, f64), String>> {
    let w: Vec<String> = s.split_whitespace().map(str::to_ascii_uppercase).collect();
    if w.len() != 3 {
        return None;
    }
    let zd: String = w[0].chars().take_while(char::is_ascii_digit).collect();
    let letter = w[0][zd.len()..].to_owned();
    if zd.is_empty() || letter.len() != 1 {
        return None;
    }
    let zone: u8 = zd.parse().ok()?;
    let e: f64 = w[1].trim_end_matches('E').parse().ok()?;
    let n: f64 = w[2].trim_end_matches('N').parse().ok()?;
    let l = letter.as_bytes()[0];
    // N/S as hemisphere; a band letter (C–X) gives the hemisphere by its position.
    let north = match l {
        b'N' => true,
        b'S' => false,
        b'C'..=b'M' => false,
        b'P'..=b'X' => true,
        _ => {
            return Some(Err(format!(
                "{letter} is not a hemisphere or latitude band"
            )));
        }
    };
    if !(1..=60).contains(&zone) {
        return Some(Err(format!("zone {zone} does not exist (1 to 60)")));
    }
    Some(Ok((zone, north, e, n)))
}

pub(crate) fn grid_center(d: &mgrs::Decoded, e: Ellipsoid) -> (f64, f64) {
    let h = d.size / 2.0;
    if d.zone == 0 {
        utmups::ups_inverse(e.a, e.f, d.north, d.easting + h, d.northing + h)
    } else {
        utmups::utm_inverse(e.a, e.f, d.zone, d.north, d.easting + h, d.northing + h)
    }
}

pub static FORMAT: ToolDef = ToolDef {
    id: "geodesy.parse.format",
    title: "Format a coordinate",
    summary: "Writes a latitude and longitude as decimal degrees, DMS, or DDM with the precision you choose, and tells you what that precision means on the ground.",
    aliases: &["decimal to DMS", "format coordinates"],
    keywords: &["DMS", "DDM", "decimal degrees", "format", "precision"],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "style",
            "Style",
            "dd, dms (default), or ddm",
            Kind::Choice(&["dd", "dms", "ddm"]),
        )
        .core(),
        Field::new(
            "decimals",
            "Decimal places",
            "In the last component; default gives about 1 cm",
            Kind::Number {
                min: 0.0,
                max: 12.0,
            },
        )
        .core(),
        Field::new(
            "signs",
            "Hemisphere style",
            "letters (N, S, E, W; default) or signed",
            Kind::Choice(&["letters", "signed"]),
        ),
    ],
    outputs: &[
        Field::new(
            "formatted",
            "Formatted",
            "The coordinate as text",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "latitude_resolution",
            "Latitude resolution",
            "Ground size of one step in the last place, north-south",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "longitude_resolution",
            "Longitude resolution",
            "Ground size of one step in the last place, east-west",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(3)),
    ],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "Rounding carried across components; resolution from the WGS 84 radii of curvature",
    accuracy: "Exact formatting; resolution is the ground size of one unit in the last place",
    references: &[DMS_REF],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh in DMS",
        input: PITTSBURGH,
        source: "Exact conversion of 40.446111°, -79.982222°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "geodesy.parse.coordinates",
        reason: "inverse",
    }],
    sentence: "{formatted}, precise to about {latitude_resolution} on the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_format,
    ..ToolDef::BLANK
};

fn run_format(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let style = match ctx.choice("style")? {
        Some("dd") => Style::Dd,
        Some("ddm") => Style::Ddm,
        _ => Style::Dms,
    };
    let default = match style {
        Style::Dd => 7.0,
        Style::Ddm => 5.0,
        Style::Dms => 3.0,
    };
    let decimals = ctx.number("decimals")?.unwrap_or(default);
    if decimals.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/decimals",
            "Decimal places must be a whole number.",
        ));
    }
    let letters = ctx.choice("signs")? != Some("signed");
    let d = decimals as u32;
    let text = format!(
        "{} {}",
        dms::format(lat, Axis::Lat, style, d, letters),
        dms::format(lon, Axis::Lon, style, d, letters)
    );
    let wgs = ellipsoid::CATALOG[0];
    let (rlat, rlon) = dms::resolution(lat, style, d, wgs.a, wgs.f);
    Ok(Json::obj([
        ("formatted", Json::str(text)),
        (
            "latitude_resolution",
            ctx.out("latitude_resolution", meters(rlat)),
        ),
        (
            "longitude_resolution",
            ctx.out("longitude_resolution", meters(rlon)),
        ),
    ]))
}

pub static BEARING_DIFFERENCE: ToolDef = ToolDef {
    id: "geodesy.parse.bearing-difference",
    title: "Difference between two bearings",
    summary: "The smallest signed turn from one bearing to another, across north when needed.",
    aliases: &["angle between bearings", "heading difference"],
    keywords: &["bearing", "heading", "difference", "turn", "angle"],
    inputs: &[
        Field::new(
            "from",
            "From bearing",
            "Like 350 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("unbounded"),
        Field::new(
            "to",
            "To bearing",
            "Like 10 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("unbounded"),
    ],
    outputs: &[Field::new(
        "difference",
        "Difference",
        "Positive is clockwise (right)",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Significant(10))
    .angle_range("[-180,180)")],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Exact remainder: ((to − from + 180) mod 360) − 180",
    accuracy: "Exact",
    references: &[DMS_REF],
    examples: &[Example {
        id: "primary",
        title: "From 350° to 010°",
        input: r#"{"from":350,"to":10}"#,
        source: "add-geodesy-suite scenario: +20°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("angle", "difference")],
    }],
    sentence: "Turn {abs(difference)} {if difference < 0}left{else}right{/if}.",
    limits: &[("batchRows", 10_000)],
    run: run_bearing_difference,
    ..ToolDef::BLANK
};

fn run_bearing_difference(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let a = ctx.req_quantity("from")?.to(dg);
    let b = ctx.req_quantity("to")?.to(dg);
    Ok(Json::obj([(
        "difference",
        ctx.out("difference", deg(wrap_lon(b - a))),
    )]))
}

const TERM: &[Field] = &[
    Field::new(
        "angle",
        "Angle",
        "DMS or decimal degrees, like 45-30-15 or 45°30'15\"",
        Kind::Text { max_len: 40 },
    )
    .required(),
    Field::new(
        "operation",
        "Operation",
        "add (default) or subtract",
        Kind::Choice(&["add", "subtract"]),
    ),
];

pub static ANGLE_ARITHMETIC: ToolDef = ToolDef {
    id: "geodesy.parse.angle-arithmetic",
    title: "Add and subtract DMS angles",
    summary: "Adds and subtracts angles in degrees, minutes, and seconds (or decimal degrees), carrying seconds and minutes exactly, with the result normalized to 0-360° or ±180° if you like.",
    aliases: &[
        "DMS calculator",
        "add degrees minutes seconds",
        "subtract angles",
        "angle calculator",
    ],
    keywords: &[
        "DMS",
        "degrees minutes seconds",
        "angle",
        "add",
        "subtract",
        "sum",
    ],
    inputs: &[
        Field::new(
            "terms",
            "Angles",
            "In order, each added or subtracted, like 45-30-15 then 12-45-50",
            Kind::List {
                items: TERM,
                min: 1,
                max: 1000,
            },
        )
        .required()
        .core(),
        Field::new(
            "normalize",
            "Normalize",
            "none (default), 0-360, or plus-minus-180",
            Kind::Choice(&["none", "0-360", "plus-minus-180"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "dms",
            "Result",
            "Degrees, minutes, and seconds to 0.01″",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "degrees",
            "Result (decimal)",
            "Decimal degrees",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("unbounded"),
        Field::new(
            "seconds",
            "Result in seconds",
            "The total in seconds of arc",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Each angle to seconds of arc (3600 × degrees), summed with its sign, then normalized if chosen and written back as D°MM'SS.ss\" with rounding carried into minutes and degrees",
    accuracy: "Exact to 0.0001″ for sums of up to 1,000 angles",
    references: &[DMS_REF],
    examples: &[Example {
        id: "primary",
        title: "45°30'15\" + 12°45'50\" − 3°00'05\"",
        input: r#"{"terms":[{"angle":"45-30-15"},{"angle":"12-45-50"},{"angle":"3-00-05","operation":"subtract"}]}"#,
        source: "Sexagesimal arithmetic: 55°16'00\"",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.parse.bearing-difference",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
    ],
    sentence: "The result is {dms}.",
    limits: &[("batchRows", 10_000)],
    run: run_angle_arithmetic,
    ..ToolDef::BLANK
};

fn run_angle_arithmetic(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("terms")?;
    let mut total = 0.0f64;
    let mut shown = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let text = ctx.row_text("terms", i, r, "angle")?.expect("required");
        let t = text.trim();
        let (neg, body) = match t.strip_prefix('-').or_else(|| t.strip_prefix('\u{2212}')) {
            Some(rest) => (true, rest),
            None => (false, t),
        };
        let v = dms::parse_plain(&body.replace('-', " ")).map_err(|_| {
            ToolError::invalid(
                &format!("/terms/{i}/angle"),
                format!("\"{text}\" is not an angle."),
            )
        })?;
        let v = if neg { -v } else { v };
        let sub = r.get("operation").and_then(|x| x.as_str()) == Some("subtract");
        if let Some(op) = r.get("operation").and_then(|x| x.as_str())
            && op != "add"
            && op != "subtract"
        {
            return Err(ToolError::invalid(
                &format!("/terms/{i}/operation"),
                "The operation is add or subtract.",
            ));
        }
        let secs = v * 3600.0;
        total += if sub { -secs } else { secs };
        shown.push(format!(
            "{}{}",
            if sub {
                "− "
            } else if i > 0 {
                "+ "
            } else {
                ""
            },
            t
        ));
    }
    let full = 360.0 * 3600.0;
    let total = match ctx.choice("normalize")?.unwrap_or("none") {
        "0-360" => total.rem_euclid(full),
        "plus-minus-180" => {
            let r = (total + full / 2.0).rem_euclid(full) - full / 2.0;
            if r == -full / 2.0 { full / 2.0 } else { r }
        }
        _ => total,
    };
    // Round to 0.0001″ so sums of exact DMS values come out exact.
    let total = (total * 1e4).round() / 1e4;
    let degrees = total / 3600.0;
    let text = dms::format(degrees, Axis::Lon, Style::Dms, 2, false);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Sum in seconds of arc",
            "Σ ± 3600 × angle",
            shown.join(" "),
            format!(
                "{}″",
                gp_base::display::number(total, Precision::Decimals(4), fmt)
            ),
        );
        ctx.step(
            "Degrees, minutes, and seconds",
            "D = ⌊s / 3600⌋, M = ⌊(s mod 3600) / 60⌋, S = s mod 60",
            format!(
                "{}″",
                gp_base::display::number(total, Precision::Decimals(4), fmt)
            ),
            text.clone(),
        );
    }
    Ok(Json::obj(vec![
        ("dms", Json::str(text)),
        ("degrees", ctx.out("degrees", deg(degrees))),
        (
            "seconds",
            ctx.out(
                "seconds",
                Q {
                    value: total,
                    unit: unit(QT::Angle, "arcsec"),
                },
            ),
        ),
    ]))
}

// ---------------------------------------------------------------- UTM / UPS

const E: [Field; 3] = ellipsoid::FIELDS;

const UTM_OUTPUTS: &[Field] = &[
    Field::new(
        "zone",
        "Zone",
        "UTM zone number",
        Kind::Number {
            min: 1.0,
            max: 60.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "hemisphere",
        "Hemisphere",
        "N or S",
        Kind::Text { max_len: 1 },
    ),
    Field::new(
        "easting",
        "Easting",
        "Meters, with the 500,000 m false easting",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(M_P),
    Field::new(
        "northing",
        "Northing",
        "Meters; 10,000,000 m false northing in the south",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(M_P),
    Field::new(
        "convergence",
        "Grid convergence",
        "Bearing of grid north clockwise from true north",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
    .angle_range("unbounded"),
    Field::new(
        "scale",
        "Point scale factor",
        "Grid distance over ellipsoid distance here",
        Kind::Number { min: 0.0, max: 2.0 },
    )
    .precision(Precision::Decimals(9)),
    Field::new(
        "formatted",
        "UTM",
        "Zone, hemisphere, easting, northing",
        Kind::Text { max_len: 60 },
    ),
];

pub static UTM_FORWARD: ToolDef = ToolDef {
    id: "geodesy.utm.forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to UTM",
    summary: "Converts a latitude and longitude to UTM zone, easting, and northing, with the Norway and Svalbard zone exceptions, an optional forced zone, and the grid convergence and scale factor.",
    aliases: &["lat long to UTM", "UTM converter", "geographic to UTM"],
    keywords: &[
        "UTM",
        "easting",
        "northing",
        "zone",
        "transverse mercator",
        "grid",
    ],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "zone",
            "Force zone",
            "Optional neighboring zone, within 3 of the standard one",
            Kind::Number {
                min: 1.0,
                max: 60.0,
            },
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: UTM_OUTPUTS,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    warnings: &["NONSTANDARD_ZONE", "INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "Transverse Mercator, 6th-order Krüger series (Karney 2011), k0 = 0.9996",
    accuracy: "Better than 5 nm within 3,900 km of the central meridian",
    when_to_use: "Use this when work is done on a metric grid: UTM easting and northing for a latitude and longitude, with the zone, the grid convergence between grid north and true north, and the point scale factor. It handles the Norway and Svalbard zone exceptions and can force a neighboring zone when a project spans a boundary.",
    limitations: "UTM is a projection, so distances and directions carry scale distortion that the scale factor quantifies, and a grid bearing is not a true bearing by the amount of the convergence. Zones are six degrees wide and coordinates do not carry their zone, so a easting and northing without it is ambiguous. The datum is whatever the input's was.",
    references: &[KARNEY_TM, NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh",
        input: PITTSBURGH,
        source: "add-geodesy-suite scenario: 17N 586,309.953 m E, 4,477,770.428 m N (±1 mm)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("easting", "easting"), ("northing", "northing")],
    }],
    related: &[
        Related {
            id: "geodesy.utm.inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "next",
        },
        Related {
            id: "geodesy.ups.forward",
            reason: "alternative",
        },
    ],
    sentence: "UTM zone {zone}{hemisphere}: easting {easting}, northing {northing}.{warn NONSTANDARD_ZONE} This is not the point's standard zone.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_utm_forward,
    ..ToolDef::BLANK
};

fn fmt_m(x: f64) -> String {
    gp_base::display::number(x, M_P, gp_base::parse::NumberFormat::DecimalPoint).replace(',', "")
}

fn utm_json(ctx: &mut Ctx, g: &utmups::Grid) -> Vec<(&'static str, Json)> {
    let hemi = if g.north { "N" } else { "S" };
    let formatted = if g.zone > 0 {
        format!(
            "{}{hemi} {} {}",
            g.zone,
            fmt_m(g.easting),
            fmt_m(g.northing)
        )
    } else {
        format!("UPS {hemi} {} {}", fmt_m(g.easting), fmt_m(g.northing))
    };
    vec![
        ("zone", Json::Num(f64::from(g.zone))),
        ("hemisphere", Json::str(hemi)),
        ("easting", ctx.out("easting", meters(g.easting))),
        ("northing", ctx.out("northing", meters(g.northing))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale", Json::Num(g.scale)),
        ("formatted", Json::str(formatted)),
    ]
}

fn run_utm_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let e = Ellipsoid::from_ctx(ctx)?;
    e.geodesic()?; // rejects |f| beyond the series method
    if !utmups::in_utm_domain(lat) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "UTM covers 80° S to 84° N; this point is outside it.",
        )
        .at("/lat")
        .hint("Use geodesy.ups.forward (Universal Polar Stereographic) near the poles."));
    }
    let standard = utmups::standard_zone(lat, lon);
    let zone = match ctx.number("zone")? {
        Some(z) => {
            if z.fract() != 0.0 {
                return Err(ToolError::invalid(
                    "/zone",
                    "The zone must be a whole number.",
                ));
            }
            let z = z as u8;
            let diff = (i32::from(z) - i32::from(standard)).rem_euclid(60);
            if diff.min(60 - diff) > 3 {
                return Err(ToolError::invalid(
                    "/zone",
                    format!("Zone {z} is more than 3 zones from the standard zone {standard}."),
                ));
            }
            if z != standard {
                ctx.warnings.push(Warning::new("NONSTANDARD_ZONE", format!("Zone {z} was forced; the standard zone here is {standard}. Scale distortion is larger.")).at("/zone"));
            }
            z
        }
        None => standard,
    };
    let g = utmups::utm_forward(e.a, e.f, lat, lon, zone);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let cm = 6.0 * f64::from(zone) - 183.0;
        ctx.step(
            "Zone",
            "zone = ⌊(lon + 180) / 6⌋ + 1, unless one is given",
            format!("{}° longitude", n(lon, 6)),
            format!("zone {zone}"),
        );
        ctx.step(
            "Central meridian",
            "λ₀ = 6° × zone − 183°",
            format!("6° × {zone} − 183°"),
            format!("{}°", n(cm, 0)),
        );
        ctx.step(
            "Easting",
            "x from the transverse Mercator series, plus the 500,000 m false easting",
            format!("{}° east of {}°", n(lon - cm, 6), n(cm, 0)),
            format!("{} m", n(g.easting, 3)),
        );
    }
    ctx.model = Some(format!(
        "Transverse Mercator, 6th-order Krüger series (Karney 2011), k0 = 0.9996, on {}",
        e.describe()
    ));
    Ok(Json::obj(utm_json(ctx, &g)))
}

pub static UTM_INVERSE: ToolDef = ToolDef {
    id: "geodesy.utm.inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "UTM to latitude and longitude",
    summary: "Converts a UTM zone, hemisphere, easting, and northing back to latitude and longitude, with the grid convergence and scale factor.",
    aliases: &["UTM to lat long", "UTM to geographic"],
    keywords: &[
        "UTM",
        "easting",
        "northing",
        "inverse",
        "latitude",
        "longitude",
    ],
    inputs: &[
        Field::new(
            "zone",
            "Zone",
            "1 to 60, like 17",
            Kind::Number {
                min: 1.0,
                max: 60.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "hemisphere",
            "Hemisphere",
            "N or S",
            Kind::Choice(&["N", "S"]),
        )
        .required()
        .core(),
        Field::new(
            "easting",
            "Easting",
            "Like 586309.953 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "northing",
            "Northing",
            "Like 4477770.428 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        lat_out("lat", "Latitude"),
        lon_out("lon", "Longitude"),
        Field::new(
            "convergence",
            "Grid convergence",
            "Bearing of grid north clockwise from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7))
        .angle_range("unbounded"),
        Field::new(
            "scale",
            "Point scale factor",
            "Grid distance over ellipsoid distance here",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(9)),
    ],
    errors: &[ErrorCode::Unsupported],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Transverse Mercator, 6th-order Krüger series (Karney 2011), k0 = 0.9996",
    accuracy: "Better than 5 nm within 3,900 km of the central meridian",
    when_to_use: "Use this when UTM coordinates arrive from a map, a data file, or a GPS set to a grid: give the zone, hemisphere, easting, and northing and it returns latitude and longitude, with the convergence and scale factor at that point. It is also the way to check a dataset's zone: converting a few coordinates and seeing where they land tells you quickly whether the zone recorded with them is the one they were computed in.",
    limitations: "The zone and hemisphere are part of the coordinate: the same easting and northing exist in every zone, so a missing or wrong zone produces a plausible position in the wrong place. The result carries the datum the coordinates were on, which this tool does not change, and points far from the central meridian carry more projection distortion. A northing alone does not say which hemisphere it belongs to, because southern coordinates carry a false northing; the hemisphere has to come with the coordinate.",
    references: &[KARNEY_TM, NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh from UTM",
        input: r#"{"zone":17,"hemisphere":"N","easting":"586309.953 m","northing":"4477770.428 m"}"#,
        source: "add-geodesy-suite scenario: 40.446111°, -79.982222°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.utm.forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_utm_inverse,
    ..ToolDef::BLANK
};

fn run_utm_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let z = ctx.number("zone")?.expect("required");
    if z.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/zone",
            "The zone must be a whole number.",
        ));
    }
    let north = ctx.choice("hemisphere")? == Some("N");
    let m = unit(QT::Length, "m");
    let e_ = ctx.req_quantity("easting")?.to(m);
    let n_ = ctx.req_quantity("northing")?.to(m);
    let ell = Ellipsoid::from_ctx(ctx)?;
    ell.geodesic()?;
    if !(100_000.0..=900_000.0).contains(&e_) || !(0.0..=10_000_000.0).contains(&n_) {
        return Err(ToolError::invalid(
            "/easting",
            "Easting must be 100,000 to 900,000 m and northing 0 to 10,000,000 m in UTM.",
        ));
    }
    let (lat, lon) = utmups::utm_inverse(ell.a, ell.f, z as u8, north, e_, n_);
    if !lat.is_finite() || !lon.is_finite() {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "These coordinates are outside the projection's domain for this ellipsoid.",
        )
        .at("/easting"));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Off the central meridian",
            "x = easting − 500,000 m, the false easting the grid adds",
            format!("{} m − 500,000 m", n(e_, 3)),
            format!("{} m", n(e_ - 500_000.0, 3)),
        );
        ctx.step(
            "Zone's central meridian",
            "λ₀ = 6° × zone − 183°",
            format!("6° × {} − 183°", n(z, 0)),
            format!("{}°", n(6.0 * z - 183.0, 0)),
        );
        ctx.step(
            "Latitude",
            "φ from the transverse Mercator series, on this ellipsoid",
            format!("{} m north at zone {}", n(n_, 3), n(z, 0)),
            format!("{}°", n(lat, 8)),
        );
    }
    let g = utmups::utm_forward(ell.a, ell.f, lat, lon, z as u8);
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale", Json::Num(g.scale)),
    ]))
}

pub static UPS_FORWARD: ToolDef = ToolDef {
    id: "geodesy.ups.forward",
    title: "Latitude and longitude to UPS",
    summary: "Converts polar coordinates (84° N and beyond, or 80° S and beyond) to Universal Polar Stereographic easting and northing.",
    aliases: &["UPS converter", "polar stereographic"],
    keywords: &["UPS", "polar", "stereographic", "arctic", "antarctic"],
    inputs: &[LAT, LON, E[0], E[1], E[2]],
    outputs: UTM_OUTPUTS,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "Polar stereographic, k0 = 0.994, false easting and northing 2,000,000 m",
    accuracy: "Exact to double precision",
    references: &[NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "85° N on the prime meridian",
        input: r#"{"lat":85,"lon":0}"#,
        source: "add-geodesy-suite scenario: 2,000,000 m E, 1,444,542.609 m N (±1 mm)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("easting", "easting"), ("northing", "northing")],
    }],
    related: &[
        Related {
            id: "geodesy.ups.inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: "UPS {hemisphere}: easting {easting}, northing {northing}.",
    limits: &[("batchRows", 10_000)],
    run: run_ups_forward,
    ..ToolDef::BLANK
};

fn run_ups_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let e = Ellipsoid::from_ctx(ctx)?;
    e.geodesic()?;
    if (-79.5..83.5).contains(&lat) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "UPS covers 83.5° N to the North Pole and 79.5° S to the South Pole (with overlap).",
        )
        .at("/lat")
        .hint("Use geodesy.utm.forward between 80° S and 84° N."));
    }
    let g = utmups::ups_forward(e.a, e.f, lat, lon, lat > 0.0);
    Ok(Json::obj(utm_json(ctx, &g)))
}

pub static UPS_INVERSE: ToolDef = ToolDef {
    id: "geodesy.ups.inverse",
    title: "UPS to latitude and longitude",
    summary: "Converts a Universal Polar Stereographic easting and northing back to latitude and longitude.",
    aliases: &["UPS to lat long"],
    keywords: &["UPS", "polar", "inverse"],
    inputs: &[
        Field::new(
            "hemisphere",
            "Hemisphere",
            "N or S",
            Kind::Choice(&["N", "S"]),
        )
        .required()
        .core(),
        Field::new(
            "easting",
            "Easting",
            "Like 2000000 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "northing",
            "Northing",
            "Like 1444542.609 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[lat_out("lat", "Latitude"), lon_out("lon", "Longitude")],
    errors: &[ErrorCode::Unsupported],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Polar stereographic, k0 = 0.994",
    accuracy: "Exact to double precision",
    references: &[NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "Back to 85° N",
        input: r#"{"hemisphere":"N","easting":"2000000 m","northing":"1444542.609 m"}"#,
        source: "add-geodesy-suite scenario inverse",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[Related {
        id: "geodesy.ups.forward",
        reason: "inverse",
    }],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_ups_inverse,
    ..ToolDef::BLANK
};

fn run_ups_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let north = ctx.choice("hemisphere")? == Some("N");
    let m = unit(QT::Length, "m");
    let e_ = ctx.req_quantity("easting")?.to(m);
    let n_ = ctx.req_quantity("northing")?.to(m);
    let ell = Ellipsoid::from_ctx(ctx)?;
    ell.geodesic()?;
    let (lat, lon) = utmups::ups_inverse(ell.a, ell.f, north, e_, n_);
    if !lat.is_finite() || !lon.is_finite() {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "These coordinates are outside the projection's domain for this ellipsoid.",
        )
        .at("/easting"));
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(wrap_lon(lon)))),
    ]))
}

pub static UTM_ZONE: ToolDef = ToolDef {
    id: "geodesy.utm.zone",
    title: "UTM zone and band lookup",
    summary: "The UTM zone and MGRS latitude band for a point, including the Norway and Svalbard exceptions, and the zone's central meridian.",
    aliases: &["what UTM zone am I in", "UTM zone finder"],
    keywords: &[
        "UTM zone",
        "latitude band",
        "grid zone designator",
        "central meridian",
    ],
    inputs: &[LAT, LON],
    outputs: &[
        Field::new(
            "zone",
            "Zone",
            "UTM zone number, or 0 for UPS",
            Kind::Number {
                min: 0.0,
                max: 60.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "grid_zone",
            "Grid zone designator",
            "Zone and band, like 17T",
            Kind::Text { max_len: 4 },
        ),
        Field::new(
            "central_meridian",
            "Central meridian",
            "Of the zone",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(0))
        .angle_range("[-180,180)")
        .optional(),
    ],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "NGA UTM zone rules with the Norway and Svalbard exceptions",
    accuracy: "Exact",
    references: &[NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "Bergen, Norway (the 32V exception)",
        input: r#"{"lat":60,"lon":5}"#,
        source: "add-geodesy-suite scenario: zone 32, not 31",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "geodesy.utm.forward",
        reason: "next",
    }],
    sentence: "The grid zone is {grid_zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_utm_zone,
    ..ToolDef::BLANK
};

fn run_utm_zone(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let wgs = ellipsoid::CATALOG[0];
    let g = utmups::forward_auto(wgs.a, wgs.f, lat, lon);
    let gzd = mgrs::encode(&g, lat, -1).map_err(|e| ToolError::new(ErrorCode::Internal, e))?;
    let mut out = vec![
        ("zone", Json::Num(f64::from(g.zone))),
        ("grid_zone", Json::str(gzd)),
    ];
    if g.zone > 0 {
        out.push((
            "central_meridian",
            ctx.out("central_meridian", deg(utmups::central_meridian(g.zone))),
        ));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- MGRS

pub(crate) const PRECISIONS: &[&str] = &[
    "grid-zone",
    "100km",
    "10km",
    "1km",
    "100m",
    "10m",
    "1m",
    "0.1m",
    "0.01m",
    "0.001m",
];

pub static MGRS_FORWARD: ToolDef = ToolDef {
    id: "geodesy.grid-ref.mgrs-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to MGRS",
    summary: "Encodes a WGS 84 latitude and longitude as an MGRS grid reference at any precision, truncating (never rounding) as the standard requires.",
    aliases: &["lat long to MGRS", "MGRS converter"],
    keywords: &["MGRS", "USNG", "military grid", "grid reference"],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "precision",
            "Precision",
            "grid-zone, 100km, 10km, 1km, 100m, 10m, 1m (default), 0.1m, 0.01m, 0.001m",
            Kind::Choice(PRECISIONS),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "mgrs",
            "MGRS",
            "The grid reference",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "mgrs_spaced",
            "MGRS (spaced)",
            "With spaces, as USNG is usually written",
            Kind::Text { max_len: 28 },
        ),
        Field::new(
            "square_size",
            "Square size",
            "The side of the referenced square (none for a grid zone alone)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(1))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "NGA MGRS (AA lettering) over UTM and UPS on WGS 84",
    accuracy: "Exact; the reference names the square containing the point (truncation)",
    when_to_use: "Use this when a position has to be written the way military and emergency services read it: an MGRS reference at the precision you choose, from a 100 km square down to a meter. It is the form used on ground operations, in search and rescue, and on maps that carry the grid.",
    limitations: "The reference names the square that contains the point, because MGRS truncates rather than rounds: a ten-digit reference is a one-meter square whose south-west corner is the position given, and a shorter one names a larger square. It is defined on UTM and UPS, so it carries their zone structure, and the coordinates here are WGS 84.",
    references: &[NGA_MGRS, GEOGRAPHICLIB_MGRS],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at 1 m",
        input: PITTSBURGH,
        source: "add-geodesy-suite scenario: 17TNE8630977770",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "bbox",
        map: &[("size", "square_size")],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.mgrs-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.usng-forward",
            reason: "alternative",
        },
    ],
    sentence: "The MGRS reference is {mgrs_spaced}{if square_size > 0}, a {square_size} square{/if}.",
    limits: &[("batchRows", 10_000)],
    run: run_mgrs_forward,
    ..ToolDef::BLANK
};

pub(crate) fn spaced(s: &str) -> String {
    let b = s.as_bytes();
    let nz = b.iter().take_while(|c| c.is_ascii_digit()).count();
    let gzd_end = (nz + 1).min(s.len());
    let mut out = s[..gzd_end].to_owned();
    if s.len() > gzd_end {
        let sq_end = (gzd_end + 2).min(s.len());
        out.push(' ');
        out.push_str(&s[gzd_end..sq_end]);
        let digits = &s[sq_end..];
        if !digits.is_empty() {
            let h = digits.len() / 2;
            out.push_str(&format!(" {} {}", &digits[..h], &digits[h..]));
        }
    }
    out
}

fn run_mgrs_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = PRECISIONS
        .iter()
        .position(|x| Some(*x) == ctx.choice("precision").ok().flatten())
        .map_or(5, |i| i as i32 - 1);
    let wgs = ellipsoid::CATALOG[0];
    let g = utmups::forward_auto(wgs.a, wgs.f, lat, lon);
    let s = mgrs::encode(&g, lat, p).map_err(|e| {
        ToolError::new(
            ErrorCode::OutOfDomain,
            format!("This point cannot be encoded: {e}."),
        )
        .at("/lat")
    })?;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Grid position",
            "the UTM or UPS zone, easting, and northing the point falls at",
            format!("{}°, {}°", n(lat, 6), n(lon, 6)),
            format!(
                "zone {}, {} m E, {} m N",
                g.zone,
                n(g.easting, 0),
                n(g.northing, 0)
            ),
        );
        ctx.step(
            "Square",
            "how much of the easting and northing the chosen precision keeps",
            format!("precision {p}"),
            format!("{} m across", n(mgrs::square_size(p), 0)),
        );
        ctx.step(
            "Reference",
            "zone, band, 100 km square, then the kept digits",
            format!("zone {} at precision {p}", g.zone),
            s.clone(),
        );
    }
    let mut out = vec![
        ("mgrs", Json::str(&s)),
        ("mgrs_spaced", Json::str(spaced(&s))),
    ];
    if p >= 0 {
        out.push((
            "square_size",
            ctx.out("square_size", meters(mgrs::square_size(p))),
        ));
    }
    Ok(Json::obj(out))
}

pub static MGRS_INVERSE: ToolDef = ToolDef {
    id: "geodesy.grid-ref.mgrs-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "MGRS to latitude and longitude",
    summary: "Decodes an MGRS or USNG grid reference to the south-west corner and center of the square it names, with the square's size.",
    aliases: &["MGRS to lat long", "decode MGRS"],
    keywords: &["MGRS", "USNG", "decode", "grid reference"],
    inputs: &[Field::new(
        "mgrs",
        "MGRS reference",
        "Like 17TNE8630977770 or 17T NE 86309 77770",
        Kind::Text { max_len: 32 },
    )
    .required()
    .core()],
    outputs: &[
        lat_out("lat", "Center latitude"),
        lon_out("lon", "Center longitude"),
        lat_out("corner_lat", "South-west corner latitude"),
        lon_out("corner_lon", "South-west corner longitude"),
        Field::new(
            "square_size",
            "Square size",
            "The side of the referenced square",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(1)),
    ],
    warnings: &["BAND_ADJUSTED", "EXPERIMENTAL_TOOL"],
    model: "NGA MGRS (AA lettering) over UTM and UPS on WGS 84",
    accuracy: "Exact; the true point is anywhere in the square",
    when_to_use: "Use this when an MGRS or USNG reference arrives by radio, from a report, or off a map and you need to plot it: it returns the south-west corner and the center of the square it names, with the square's size so the precision is explicit. It is also how to check the precision of a reference you were given: the square's size is reported, so an eight-digit reference is visibly a ten-meter square rather than a point.",
    limitations: "The reference is a square and the true point is anywhere within it: at six digits that is a hundred-meter square, at four a kilometer. The center is offered for plotting, not as the reported position. A reference without its grid zone is ambiguous over long distances, and this reports what it assumed. Polar references use UPS rather than UTM and follow different lettering, which this handles but which is worth knowing when a reference looks unusual.",
    references: &[NGA_MGRS, GEOGRAPHICLIB_MGRS],
    examples: &[Example {
        id: "primary",
        title: "Decode a 1 m reference",
        input: r#"{"mgrs":"17TNE8630977770"}"#,
        source: "add-geodesy-suite scenario: 1 m square, center offset 0.5 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "bbox",
        map: &[("size", "square_size")],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.usng-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
    ],
    sentence: "The center of this {square_size} square is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_mgrs_inverse,
    ..ToolDef::BLANK
};

fn run_mgrs_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("mgrs")?.expect("required");
    let wgs = ellipsoid::CATALOG[0];
    let d = mgrs::decode(&s, wgs.a, wgs.f)
        .map_err(|e| ToolError::invalid("/mgrs", format!("This reference is not valid: {e}.")))?;
    if d.band_adjusted {
        ctx.warnings.push(Warning::new(
            "BAND_ADJUSTED",
            format!(
                "The latitude band {} was adjusted: the square lies on a band boundary.",
                d.band
            ),
        ));
    }
    let (clat, clon) = grid_center(&d, wgs);
    let (slat, slon) = if d.zone == 0 {
        utmups::ups_inverse(wgs.a, wgs.f, d.north, d.easting, d.northing)
    } else {
        utmups::utm_inverse(wgs.a, wgs.f, d.zone, d.north, d.easting, d.northing)
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Square the reference names",
            "the digits give a square, not a point: half as many digits, ten times the square",
            s.trim().to_owned(),
            format!("{} m across", n(d.size, 0)),
        );
        ctx.step(
            "Its south-west corner",
            "the grid easting and northing the reference resolves to",
            format!(
                "zone {}, {} m E, {} m N",
                d.zone,
                n(d.easting, 0),
                n(d.northing, 0)
            ),
            format!("{}°, {}°", n(slat, 6), n(wrap_lon(slon), 6)),
        );
        ctx.step(
            "Centre of the square",
            "the middle of it, which is the position to use",
            format!("{} m from the corner", n(d.size / 2.0, 1)),
            // The card prints seven decimals; the last step has to match it.
            format!("{}°", n(clat, 7)),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(clat))),
        ("lon", ctx.out("lon", deg(wrap_lon(clon)))),
        ("corner_lat", ctx.out("corner_lat", deg(slat))),
        ("corner_lon", ctx.out("corner_lon", deg(wrap_lon(slon)))),
        ("square_size", ctx.out("square_size", meters(d.size))),
    ]))
}

pub static TOOLS: &[&ToolDef] = &[
    &PARSE,
    &FORMAT,
    &BEARING_DIFFERENCE,
    &ANGLE_ARITHMETIC,
    &UTM_FORWARD,
    &UTM_INVERSE,
    &UTM_ZONE,
    &UPS_FORWARD,
    &UPS_INVERSE,
    &MGRS_FORWARD,
    &MGRS_INVERSE,
    &magnetic::DECLINATION,
    &magnetic::TRUE_TO_MAGNETIC,
    &magnetic::GRIVATION,
    &spcs::FORWARD,
    &spcs::INVERSE,
    &spcs::LOOKUP,
    &spcs::ARC_TO_CHORD,
    &geoid::GEOID_HEIGHT,
    &geoid::HEIGHT_CONVERT,
    &frames::PARAMETERS,
    &frames::RADII,
    &frames::AUXILIARY,
    &frames::TO_ECEF,
    &frames::FROM_ECEF,
    &frames::TO_LOCAL,
    &frames::FROM_LOCAL,
    &gridref::USNG_FORWARD,
    &gridref::USNG_INVERSE,
    &gridref::MAIDENHEAD_FORWARD,
    &gridref::MAIDENHEAD_INVERSE,
    &gridref::GARS_FORWARD,
    &gridref::GARS_INVERSE,
    &gridref::GEOREF_FORWARD,
    &gridref::GEOREF_INVERSE,
    &datum::HELMERT,
    &datum::ITRF,
    &datum::PLATE_MOTION,
    &datum::NAD83,
    &datum::LEGACY_SHIFT,
    &datum::NADCON5,
];

pub static REGISTRY: Registry = Registry {
    module: "geodesy",
    tools: TOOLS,
};

gp_base::export_module!("geodesy", REGISTRY);
