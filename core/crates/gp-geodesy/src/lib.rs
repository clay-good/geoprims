//! Geodesy: coordinate parsing and formatting, UTM, UPS, and MGRS
//! (add-geodesy-suite). The math lives in gp-geo so other modules share it.

pub mod crs;
pub mod datum;
pub mod frames;
pub mod geoid;
pub mod gridref;
pub mod magnetic;
pub mod methods;
pub mod spcs;
pub mod spcs2022;

use gp_base::ErrorCode;
use gp_base::angle::wrap_lon;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related,
    Stability, ToolDef,
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

const ALTERNATIVE_ROW: &[Field] = &[
    Field::new(
        "reading",
        "Reading",
        "How the text is read, like longitude first",
        Kind::Text { max_len: 80 },
    ),
    Field::new(
        "valid",
        "Valid",
        "yes when this reading gives a coordinate, no when it cannot",
        Kind::Text { max_len: 3 },
    ),
    lat_out("lat", "Latitude").optional(),
    lon_out("lon", "Longitude").optional(),
];

pub static PARSE: ToolDef = ToolDef {
    id: "geodesy.parse.coordinates",
    stability: gp_base::tool::Stability::Stable,
    version: "1.3.0",
    title: "Read any coordinate",
    summary: "Reads a coordinate in almost any notation (decimal, DMS, DDM, packed aviation, labeled, MGRS, UTM, UPS, or a grid code) and reports what it assumed.",
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
        "Maidenhead",
        "GARS",
        "GEOREF",
        "geohash",
        "Plus Code",
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
        Field::new(
            "alternatives",
            "Other readings",
            "Each other way the text could be read, and whether it gives a coordinate",
            Kind::List {
                items: ALTERNATIVE_ROW,
                min: 0,
                max: 4,
            },
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
    when_to_use: "Use this when a coordinate arrives as text and you do not want to hand-convert it: decimal degrees, degrees and minutes, degrees, minutes and seconds, packed aviation form, labeled pairs, MGRS, UTM, UPS, or a grid code (Maidenhead, GARS, GEOREF, geohash, or a full Plus Code, read as the center of its cell). It reports what it read, in your chosen output form, and says which assumptions it had to make.",
    limitations: "Text can be genuinely ambiguous — which number is the latitude, which hemisphere an unsigned pair means, whether a comma is a decimal mark — and this tool reports the ambiguity and its alternatives instead of guessing; a short code like fn20 is both a Maidenhead square and a geohash, and both readings are given. A short Plus Code needs a nearby place to complete it and is refused. It cannot know the datum of a bare pair: coordinates are treated as WGS 84 unless the notation itself carries one.",
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

pub static PARSE_DMS: ToolDef = ToolDef {
    id: "geodesy.parse.parse-dms",
    stability: gp_base::tool::Stability::Stable,
    title: "Degrees, minutes, and seconds to decimal degrees",
    summary: "Reads a latitude and longitude written in degrees, minutes, and seconds and returns them as decimal degrees.",
    aliases: &[
        "DMS to decimal degrees converter",
        "degrees minutes seconds to decimal",
        "convert DMS coordinates",
    ],
    keywords: &[
        "coordinates",
        "latitude",
        "longitude",
        "DMS",
        "parse",
        "decimal degrees",
    ],
    inputs: PARSE.inputs,
    outputs: PARSE.outputs,
    errors: PARSE.errors,
    warnings: PARSE.warnings,
    model: PARSE.model,
    accuracy: PARSE.accuracy,
    when_to_use: "Use this when a position comes from a chart, a deed, a survey plan, or a photo's metadata in degrees, minutes, and seconds, and the next step needs decimal degrees. Paste the text as written, with symbols, spaces, or dashes between the parts and hemisphere letters or signs. The tool returns the decimal latitude and longitude and the cleaned-up text.",
    limitations: "Only degrees, minutes, and seconds are accepted here, so a decimal pair, a grid reference, or another notation is refused with a note of what it reads as; the any-notation reader takes those. Text can still be ambiguous about which number is the latitude or which hemisphere an unsigned value means, and the assumption made is stated. No datum is applied: the numbers are whatever datum the text was on.",
    references: PARSE.references,
    examples: &[Example {
        id: "primary",
        title: "Sydney Airport from a chart",
        input: r#"{"text":"33°56'46\"S 151°10'38\"E"}"#,
        source: "ISO 6709 notation, read by exact arithmetic",
    }],
    primary_example: "primary",
    visualization: PARSE.visualization,
    related: &[
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "next",
        },
    ],
    sentence: PARSE.sentence,
    limits: PARSE.limits,
    run: run_parse_dms,
    ..ToolDef::BLANK
};

pub static PARSE_DDM: ToolDef = ToolDef {
    id: "geodesy.parse.parse-ddm",
    stability: gp_base::tool::Stability::Stable,
    title: "Degrees and decimal minutes to decimal degrees",
    summary: "Reads a latitude and longitude written in degrees and decimal minutes and returns them as decimal degrees.",
    aliases: &[
        "DDM to decimal degrees converter",
        "degrees decimal minutes to decimal",
        "GPS minutes to decimal degrees",
    ],
    keywords: &[
        "coordinates",
        "latitude",
        "longitude",
        "DDM",
        "parse",
        "decimal degrees",
    ],
    inputs: PARSE.inputs,
    outputs: PARSE.outputs,
    errors: PARSE.errors,
    warnings: PARSE.warnings,
    model: PARSE.model,
    accuracy: PARSE.accuracy,
    when_to_use: "Use this for a position read off a marine or aviation GPS, a handheld receiver, or a geocache listing, where the notation is degrees and decimal minutes. Paste the text as written, with hemisphere letters or signs. The tool returns the decimal latitude and longitude for a spreadsheet, a GIS, or another calculation, with the position restated in degrees, minutes, and seconds.",
    limitations: "Only degrees and decimal minutes are accepted here, so a decimal pair, a grid reference, or another notation is refused with a note of what it reads as; the any-notation reader takes those. Text can still be ambiguous about which number is the latitude or which hemisphere an unsigned value means, and the assumption made is stated. No datum is applied: the numbers are whatever datum the text was on.",
    references: PARSE.references,
    examples: &[Example {
        id: "primary",
        title: "A marine GPS fix off Lisbon",
        input: r#"{"text":"38°41.250'N 9°25.500'W"}"#,
        source: "ISO 6709 notation, read by exact arithmetic",
    }],
    primary_example: "primary",
    visualization: PARSE.visualization,
    related: &[
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "next",
        },
    ],
    sentence: PARSE.sentence,
    limits: PARSE.limits,
    run: run_parse_ddm,
    ..ToolDef::BLANK
};

fn run_parse_dms(ctx: &mut Ctx) -> Result<Json, ToolError> {
    parse(ctx, Some(("DMS", "degrees, minutes, and seconds")))
}

fn run_parse_ddm(ctx: &mut Ctx) -> Result<Json, ToolError> {
    parse(ctx, Some(("DDM", "degrees and decimal minutes")))
}

fn run_parse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    parse(ctx, None)
}

/// Reads a position. A focused operation accepts one notation and refuses
/// text that reads as another, naming what it reads as.
fn parse(ctx: &mut Ctx, only: Option<(&str, &str)>) -> Result<Json, ToolError> {
    let text = ctx.text("text")?.expect("required");
    let wgs = ellipsoid::CATALOG[0];
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut alternatives = Vec::new();
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
    } else if let Some(((notation, (la, lo), (h, w)), others)) = {
        // One token only, and only when it is not a coordinate: a grid
        // reference is never read in place of a number.
        let token = !text.trim().contains(char::is_whitespace);
        let found = if token && dms::parse_pair(&text).is_err() {
            grid_readings(&compact)
        } else {
            Ok(Vec::new())
        };
        let found = found.map_err(|e| {
            ToolError::invalid("/text", format!("This looks like a Plus Code, but {e}."))
        })?;
        let mut it = found.into_iter();
        it.next().map(|first| (first, it.collect::<Vec<_>>()))
    } {
        // A code names a cell, not a point: the position is its center.
        let size = |h: f64, w: f64| {
            let fmt = |d: f64| gp_base::num::format_f64(d).unwrap_or_default();
            format!("{}° by {}°", fmt(h), fmt(w))
        };
        ctx.warnings.push(Warning::new(
            "INPUT_NORMALIZED",
            format!("A {notation} code names a cell {} (latitude by longitude); this is its center.", size(h, w)),
        ).at("/text"));
        if !others.is_empty() {
            let names: Vec<&str> = others.iter().map(|o| o.0).collect();
            ctx.warnings.push(
                Warning::new(
                    "AMBIGUOUS_INPUT",
                    format!(
                        "{} was read as {notation}; it is also a valid {}.",
                        compact,
                        names.join(" and a valid ")
                    ),
                )
                .at("/text"),
            );
        }
        for (other, (ola, olo), _) in others {
            alternatives.push(Json::obj([
                ("reading", Json::str(format!("as {other}"))),
                ("valid", Json::str("yes")),
                ("lat", deg(ola).to_json()),
                ("lon", deg(olo).to_json()),
            ]));
        }
        (la, lo, notation)
    } else if let Some(u) = parse_utm_text(&text) {
        let (zone, north, e, n) = u.map_err(|e| ToolError::invalid("/text", e))?;
        if zone == 0 {
            let (la, lo) = utmups::ups_inverse(wgs.a, wgs.f, north, e, n);
            // UPS covers the caps, north of 84 and south of 80 degrees, with
            // half a degree of overlap with UTM.
            if (north && la < 83.5) || (!north && la > -79.5) {
                return Err(ToolError::invalid(
                    "/text",
                    format!(
                        "{text} is outside the UPS polar area; its easting and northing reach {la:.1} degrees of latitude."
                    ),
                ));
            }
            (la, lo, "UPS")
        } else {
            let (la, lo) = utmups::utm_inverse(wgs.a, wgs.f, zone, north, e, n);
            (la, lo, "UTM")
        }
    } else {
        let p = dms::parse_pair(&text).map_err(|e| ToolError::invalid("/text", format!("{e}.")))?;
        // A single token that is also a grid reference: the coordinate reading
        // stands, and the grid reading is offered.
        if !text.trim().contains(char::is_whitespace) {
            let grids = grid_readings(&compact).unwrap_or_default();
            if !grids.is_empty() {
                let names: Vec<&str> = grids.iter().map(|g| g.0).collect();
                ctx.warnings.push(
                    Warning::new(
                        "AMBIGUOUS_INPUT",
                        format!(
                            "{compact} was read as a coordinate; it is also a valid {}.",
                            names.join(" and a valid ")
                        ),
                    )
                    .at("/text"),
                );
            }
            for (other, (ola, olo), _) in grids {
                alternatives.push(Json::obj([
                    ("reading", Json::str(format!("as {other}"))),
                    ("valid", Json::str("yes")),
                    ("lat", deg(ola).to_json()),
                    ("lon", deg(olo).to_json()),
                ]));
            }
        }
        if let Some(a) = p.ambiguity {
            ctx.warnings
                .push(Warning::new("AMBIGUOUS_INPUT", a).at("/text"));
        }
        for alt in &p.alternatives {
            let mut row = vec![
                ("reading", Json::str(&alt.reading)),
                (
                    "valid",
                    Json::str(if alt.lat_lon.is_some() { "yes" } else { "no" }),
                ),
            ];
            if let Some((la, lo)) = alt.lat_lon {
                row.push(("lat", deg(la).to_json()));
                row.push(("lon", deg(lo).to_json()));
            }
            alternatives.push(Json::obj(row));
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
    if let Some((wanted, words)) = only
        && notation != wanted
    {
        return Err(
            ToolError::invalid("/text", format!("This reads as {notation}, not {words}."))
                .hint("The any-notation coordinate reader accepts it."),
        );
    }
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
        ("alternatives", Json::Arr(alternatives)),
    ]))
}

type GridReading = (&'static str, (f64, f64), (f64, f64));

/// Every grid reference the text is a valid code in, with its cell's center
/// and size: (notation, (lat, lon), (height, width) in degrees). Plus Codes,
/// Maidenhead locators, GARS, GEOREF, and geohashes. The same characters can
/// be a valid code in more than one of them, so all are returned.
fn grid_readings(s: &str) -> Result<Vec<GridReading>, String> {
    use gp_geo::{codes, gridref};
    let mut out = Vec::new();
    if s.is_empty() || s.chars().any(|c| !c.is_ascii_alphanumeric() && c != '+') {
        return Ok(out);
    }
    if s.contains('+') {
        codes::olc_check(s)?;
        if !codes::olc_is_full(s) {
            return Err("this is a short Plus Code: it names a place only near a reference location, so recover the full code with the Plus Code tool first".into());
        }
        let ((so, w, n, e), _) = codes::olc_decode(s);
        out.push((
            "Plus Code",
            ((so + n) / 2.0, (w + e) / 2.0),
            (n - so, e - w),
        ));
        return Ok(out);
    }
    let cell = |c: gridref::Cell| (c.center(), (c.height, c.width));
    if let Ok(c) = gridref::maidenhead_decode(s) {
        let (p, d) = cell(c);
        out.push(("Maidenhead", p, d));
    }
    if let Ok(c) = gridref::gars_decode(s) {
        let (p, d) = cell(c);
        out.push(("GARS", p, d));
    }
    if let Ok((c, _)) = gridref::georef_decode(s) {
        let (p, d) = cell(c);
        out.push(("GEOREF", p, d));
    }
    // A geohash needs a letter: digits alone are a number, not a code.
    if s.len() <= 12
        && s.chars().any(|c| c.is_ascii_alphabetic())
        && let Ok((so, w, n, e)) = codes::geohash_decode(s)
    {
        out.push(("geohash", ((so + n) / 2.0, (w + e) / 2.0), (n - so, e - w)));
    }
    // Upper case reads first as Maidenhead or GEOREF, lower case as a geohash.
    if s.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .take(2)
        .all(|c| c.is_ascii_lowercase())
    {
        out.sort_by_key(|r| r.0 != "geohash");
    }
    Ok(out)
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

/// `17N 586309 4477770` or `17T 586309E 4477770N` (zone, hemisphere or band,
/// easting, northing), or UPS as `Z 2426773 1530125` (zone 0; A, B, Y, or Z
/// as in MGRS, or N or S when both values are grid values, not degrees).
fn parse_utm_text(s: &str) -> Option<Result<(u8, bool, f64, f64), String>> {
    let w: Vec<String> = s.split_whitespace().map(str::to_ascii_uppercase).collect();
    if w.len() != 3 {
        return None;
    }
    let zd: String = w[0].chars().take_while(char::is_ascii_digit).collect();
    let letter = w[0][zd.len()..].to_owned();
    if letter.len() != 1 {
        return None;
    }
    let e: f64 = w[1].trim_end_matches('E').parse().ok()?;
    let n: f64 = w[2].trim_end_matches('N').parse().ok()?;
    if zd.is_empty() {
        let north = match letter.as_str() {
            "Y" | "Z" => true,
            "A" | "B" => false,
            "N" | "S" if e >= 100_000.0 && n >= 100_000.0 => letter == "N",
            _ => return None,
        };
        return Some(Ok((0, north, e, n)));
    }
    let zone: u8 = zd.parse().ok()?;
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
    stability: Stability::Stable,
    when_to_use: "Use this to write a position the way a form, a chart or a colleague wants it -- decimal degrees, degrees and decimal minutes, or degrees minutes seconds -- at a precision you choose, and to find out what that precision is actually worth on the ground before you decide how many digits to keep.",
    limitations: "The resolution reported is the ground size of one unit in the last place, not the accuracy of the number: a coordinate printed to six decimals is not thereby good to eleven centimetres. Longitude resolution follows the cosine of the latitude, so the same number of digits means different distances at different latitudes, and at the pole it means nothing at all. Rounding is carried across the components, so a value a hair under a whole degree prints as that degree rather than as fifty-nine minutes sixty seconds.",
    warnings: &["INPUT_NORMALIZED"],
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
    related: &[
        Related {
            id: "geodesy.parse.coordinates",
            reason: "inverse",
        },
        Related {
            id: "geodesy.parse.angle-arithmetic",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "alternative",
        },
    ],
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    sentence: "{formatted}, precise to about {latitude_resolution} on the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_format,
    ..ToolDef::BLANK
};

pub static FORMAT_DD: ToolDef = ToolDef {
    id: "geodesy.parse.format-dd",
    stability: gp_base::tool::Stability::Stable,
    title: "Coordinates in decimal degrees",
    summary: "Writes a latitude and longitude as decimal degrees, with hemisphere letters or signs and a chosen number of decimal places.",
    aliases: &[
        "decimal degrees formatter",
        "lat long to decimal degrees",
        "DD coordinates",
    ],
    keywords: &[
        "coordinates",
        "latitude",
        "longitude",
        "decimal degrees",
        "format",
        "notation",
    ],
    inputs: &[
        FORMAT.inputs[0],
        FORMAT.inputs[1],
        FORMAT.inputs[3],
        FORMAT.inputs[4],
    ],
    outputs: FORMAT.outputs,
    errors: FORMAT.errors,
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: FORMAT.model,
    accuracy: FORMAT.accuracy,
    when_to_use: "Use this when a system wants plain decimal degrees: a spreadsheet, a GIS import, a web map link, or an API. Enter the latitude and longitude and choose the decimal places; seven is the default, about a centimeter. Pick signed output for software and hemisphere letters for people. The ground size of the last decimal place comes back with the text.",
    limitations: "Decimal degrees say nothing about the datum, so the numbers are on whatever datum the input was; nothing is converted. Rounding to the decimal places chosen is the only loss. A decimal place of longitude covers less ground toward the poles than one of latitude, and both sizes are reported. Signed output drops the hemisphere letters, so the reader must know that south and west are negative.",
    references: FORMAT.references,
    examples: &[Example {
        id: "primary",
        title: "Sydney Opera House to 5 decimal places",
        input: r#"{"lat":-33.856784,"lon":151.215297,"decimals":5}"#,
        source: "ISO 6709 decimal-degree notation",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.parse.format",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "alternative",
        },
    ],
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    sentence: FORMAT.sentence,
    limits: FORMAT.limits,
    run: run_format_dd,
    ..ToolDef::BLANK
};

pub static FORMAT_DMS: ToolDef = ToolDef {
    id: "geodesy.parse.format-dms",
    stability: gp_base::tool::Stability::Stable,
    title: "Coordinates in degrees, minutes, and seconds",
    summary: "Writes a latitude and longitude as degrees, minutes, and seconds, with hemisphere letters or signs and a chosen number of decimal places on the seconds.",
    aliases: &[
        "DMS formatter",
        "decimal degrees to degrees minutes seconds",
        "DMS coordinates",
    ],
    keywords: &[
        "coordinates",
        "latitude",
        "longitude",
        "DMS",
        "format",
        "notation",
    ],
    inputs: &[
        FORMAT.inputs[0],
        FORMAT.inputs[1],
        FORMAT.inputs[3],
        FORMAT.inputs[4],
    ],
    outputs: FORMAT.outputs,
    errors: FORMAT.errors,
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: FORMAT.model,
    accuracy: FORMAT.accuracy,
    when_to_use: "Use this when a position has to be read aloud, written on a plan, or matched to a chart or a legal description that uses degrees, minutes, and seconds. Enter the latitude and longitude in decimal degrees and choose the decimal places on the seconds; three is the default, about three centimeters. The ground size of the last digit comes back with the text.",
    limitations: "The text is rounded once, at the seconds, so a value a hair under a whole minute prints as the next minute rather than as 60 seconds. No datum is applied or converted. Seconds of longitude cover less ground toward the poles than seconds of latitude, and both sizes are reported. Software that expects decimal degrees will not read this notation without parsing it first.",
    references: FORMAT.references,
    examples: &[Example {
        id: "primary",
        title: "The Eiffel Tower to a tenth of a second",
        input: r#"{"lat":48.858370,"lon":2.294481,"decimals":1}"#,
        source: "ISO 6709 sexagesimal notation",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.parse.format",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "alternative",
        },
    ],
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    sentence: FORMAT.sentence,
    limits: FORMAT.limits,
    run: run_format_dms,
    ..ToolDef::BLANK
};

pub static FORMAT_DDM: ToolDef = ToolDef {
    id: "geodesy.parse.format-ddm",
    stability: gp_base::tool::Stability::Stable,
    title: "Coordinates in degrees and decimal minutes",
    summary: "Writes a latitude and longitude as degrees and decimal minutes, with hemisphere letters or signs and a chosen number of decimal places on the minutes.",
    aliases: &[
        "DDM formatter",
        "decimal degrees to degrees decimal minutes",
        "GPS degrees and minutes",
    ],
    keywords: &[
        "coordinates",
        "latitude",
        "longitude",
        "DDM",
        "format",
        "notation",
    ],
    inputs: &[
        FORMAT.inputs[0],
        FORMAT.inputs[1],
        FORMAT.inputs[3],
        FORMAT.inputs[4],
    ],
    outputs: FORMAT.outputs,
    errors: FORMAT.errors,
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: FORMAT.model,
    accuracy: FORMAT.accuracy,
    when_to_use: "Use this for the notation marine and aviation GPS units show, and that geocaching and many handheld receivers default to: degrees and decimal minutes. Enter the latitude and longitude in decimal degrees and choose the decimal places on the minutes; five is the default, under two centimeters. The ground size of the last digit comes back with the text.",
    limitations: "The text is rounded once, at the minutes, so a value a hair under a whole degree prints as the next degree rather than as 60 minutes. No datum is applied or converted. A decimal place of a minute is 60 times coarser than the same place of a degree, so count the places you need. Minutes of longitude cover less ground toward the poles, and both sizes are reported.",
    references: FORMAT.references,
    examples: &[Example {
        id: "primary",
        title: "Cape Town harbor to a thousandth of a minute",
        input: r#"{"lat":-33.906389,"lon":18.420833,"decimals":3}"#,
        source: "ISO 6709 degrees and decimal minutes notation",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.parse.format",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "alternative",
        },
    ],
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    sentence: FORMAT.sentence,
    limits: FORMAT.limits,
    run: run_format_ddm,
    ..ToolDef::BLANK
};

fn run_format_dd(ctx: &mut Ctx) -> Result<Json, ToolError> {
    format(ctx, Some(Style::Dd))
}

fn run_format_dms(ctx: &mut Ctx) -> Result<Json, ToolError> {
    format(ctx, Some(Style::Dms))
}

fn run_format_ddm(ctx: &mut Ctx) -> Result<Json, ToolError> {
    format(ctx, Some(Style::Ddm))
}

fn run_format(ctx: &mut Ctx) -> Result<Json, ToolError> {
    format(ctx, None)
}

/// Formats a position. A focused operation fixes the notation; the general
/// one reads it from its `style` input.
fn format(ctx: &mut Ctx, fixed: Option<Style>) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let style = match fixed {
        Some(style) => style,
        None => match ctx.choice("style")? {
            Some("dd") => Style::Dd,
            Some("ddm") => Style::Ddm,
            _ => Style::Dms,
        },
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
    stability: Stability::Stable,
    when_to_use: "Use this for the turn between two headings or bearings when one of them may be on the other side of north: a course change, the angle between two legs, the correction from a heading flown to a heading wanted. It gives the short way round, signed, instead of the long way a plain subtraction gives.",
    limitations: "The answer is the smaller of the two ways round, so it is never more than half a turn: if you need the actual turn a vehicle made through 200 degrees, this is not it. A difference of exactly 180 degrees comes out as minus 180 rather than plus 180, which falls out of the expression the tool states rather than being a choice about which way to turn -- at a reversal there is no shorter way. This is plane angle arithmetic and knows nothing about magnetic or true north; converting between those is the declination tool.",
    warnings: &["UNIT_ASSUMED"],
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
    related: &[
        Related {
            id: "geodesy.parse.angle-arithmetic",
            reason: "alternative",
        },
        Related {
            id: "geodesy.magnetic.true-to-magnetic",
            reason: "next",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "alternative",
        },
    ],
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
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    stability: Stability::Stable,
    when_to_use: "Use this to add and subtract angles written in degrees, minutes and seconds without doing the sexagesimal carries by hand: summing the interior angles of a traverse, applying a correction to a bearing, or closing a set of observations. Decimal degrees mix freely with DMS in the same sum.",
    limitations: "The carry is exact and the printed seconds are rounded once at the end, so a sum a hair under a whole degree prints as that degree rather than as fifty-nine minutes sixty seconds -- but two sums each rounded to a hundredth of a second and then added by hand can still differ from the same terms summed here. Without a normalization the total is not wrapped, which is what a traverse needs; choose one if you want a bearing back. This is arithmetic on angles, not on directions: it does not know that a bearing and its reverse describe one line.",
    warnings: &[],
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
        Related {
            id: "geodesy.parse.format",
            reason: "next",
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
        // Past this the sum loses its hundredths of a second, and enough such
        // terms overflow to infinity.
        if !v.is_finite() || v.abs() > 1e9 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "An angle term must be within a billion degrees either way.",
            )
            .at(&format!("/terms/{i}/angle")));
        }
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

/// The forward tool's outputs: the shared ones, then the grid zone the
/// point falls in, which the map draws (grid-references "Grid overlays").
const UTM_FORWARD_OUTPUTS: &[Field] = &[
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
    Field::new(
        "band",
        "Latitude band",
        "The MGRS band letter, C to X; with the zone it names the grid zone",
        Kind::Text { max_len: 1 },
    )
    .optional(),
    Field::new(
        "zone_west",
        "Zone west edge",
        "Longitude where the standard zone begins",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(0))
    .angle_range("[-180,180]")
    .optional(),
    Field::new(
        "zone_east",
        "Zone east edge",
        "Longitude where the standard zone ends",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(0))
    .angle_range("[-180,180]")
    .optional(),
    Field::new(
        "band_south",
        "Band south edge",
        "Latitude where the band begins",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(0))
    .angle_range("[-90,90]")
    .optional(),
    Field::new(
        "band_north",
        "Band north edge",
        "Latitude where the band ends",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(0))
    .angle_range("[-90,90]")
    .optional(),
];

pub static UTM_FORWARD: ToolDef = ToolDef {
    id: "geodesy.utm.forward",
    version: "1.1.0",
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
    outputs: UTM_FORWARD_OUTPUTS,
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
    visualization: &[
        Layer {
            kind: "point",
            map: &[("easting", "easting"), ("northing", "northing")],
        },
        Layer {
            kind: "bbox",
            map: &[
                ("south", "band_south"),
                ("west", "zone_west"),
                ("north", "band_north"),
                ("east", "zone_east"),
            ],
        },
    ],
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
    assumptions: &[
        Assumption {
            name: "UTM scale factor on the central meridian k0",
            value: "0.9996",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false easting",
            value: "500000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false northing in the southern hemisphere",
            value: "10000000",
            unit: "m",
            source: "nga-utmups",
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

const CENTRAL_MERIDIAN_OUT: Field = Field::new(
    "central_meridian",
    "Central meridian",
    "Of the UTM zone used",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(0));

pub static CONVERGENCE: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.projection.convergence",
    title: "Grid convergence in UTM",
    summary: "The angle between true north and UTM grid north at a point, positive when grid north lies east of true north, with the point scale factor there.",
    aliases: &[
        "grid convergence calculator",
        "meridian convergence angle",
        "grid north to true north",
    ],
    keywords: &[
        "convergence",
        "grid north",
        "true north",
        "UTM",
        "transverse Mercator",
        "meridian convergence",
        "azimuth",
    ],
    inputs: UTM_FORWARD.inputs,
    outputs: &[
        UTM_FORWARD.outputs[4],
        UTM_FORWARD.outputs[5],
        UTM_FORWARD.outputs[0],
        CENTRAL_MERIDIAN_OUT,
    ],
    errors: UTM_FORWARD.errors,
    warnings: &["NONSTANDARD_ZONE", "INPUT_NORMALIZED"],
    model: "Transverse Mercator convergence from the Krüger series (Karney 2011): close to (λ − λ₀)·sin φ for small longitude differences; true azimuth = grid azimuth + convergence",
    accuracy: UTM_FORWARD.accuracy,
    when_to_use: "Use this to turn a grid bearing measured on a UTM map or computed from UTM coordinates into a true bearing, or the other way round. Enter the latitude and longitude. The tool returns the convergence in the point's standard zone, or in a zone you force. Add it to a grid azimuth to get the true azimuth; subtract it to go from true to grid.",
    limitations: "The convergence is for UTM, a transverse Mercator projection with six-degree zones, and differs in a State Plane zone or any other projection. It is zero on the zone's central meridian and grows toward the zone edges and the poles. It is the angle at a point, so a long line also needs the arc-to-chord correction. Magnetic north is a separate correction.",
    references: UTM_FORWARD.references,
    examples: &[Example {
        id: "primary",
        title: "Longyearbyen, far from its central meridian",
        input: r#"{"lat":78.2232,"lon":15.6267}"#,
        source: "Krüger-series transverse Mercator on WGS 84, zone 33",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.arc-to-chord",
            reason: "next",
        },
        Related {
            id: "geodesy.magnetic.grivation",
            reason: "next",
        },
    ],
    sentence: "Grid north is {convergence} from true north here, in UTM zone {zone}.",
    limits: UTM_FORWARD.limits,
    run: run_convergence,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

pub static SCALE_FACTOR: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.projection.scale-factor",
    title: "Point scale factor in UTM",
    summary: "The UTM grid scale factor at a point: how much a short distance is stretched or shrunk on the grid there, with the grid convergence.",
    aliases: &[
        "grid scale factor calculator",
        "point scale factor",
        "grid scale factor at a point",
    ],
    keywords: &[
        "scale factor",
        "grid scale",
        "UTM",
        "k0",
        "transverse Mercator",
        "distortion",
        "grid distance",
    ],
    inputs: UTM_FORWARD.inputs,
    outputs: &[
        UTM_FORWARD.outputs[5],
        UTM_FORWARD.outputs[4],
        UTM_FORWARD.outputs[0],
        CENTRAL_MERIDIAN_OUT,
    ],
    errors: UTM_FORWARD.errors,
    warnings: &["NONSTANDARD_ZONE", "INPUT_NORMALIZED"],
    model: "Transverse Mercator point scale from the Krüger series (Karney 2011), with k₀ = 0.9996 on the central meridian: grid distance = ellipsoid distance × scale factor",
    accuracy: UTM_FORWARD.accuracy,
    when_to_use: "Use this before comparing a distance from UTM coordinates with one measured on the ground. Enter the latitude and longitude. The tool returns the point scale factor in the standard zone, or a zone you force: 0.9996 on the central meridian, 1 about 180 km either side, and above 1 beyond that. Multiply an ellipsoid distance by it to get the grid distance.",
    limitations: "The factor holds at a point; over a long line use the mean of the ends and middle. It covers the projection only, so ground distances also need the elevation factor, and the two together make the combined factor. It is for UTM and differs in a State Plane zone or any other projection. Outside the standard zone the distortion grows quickly.",
    references: UTM_FORWARD.references,
    examples: &[Example {
        id: "primary",
        title: "Bergen, near the western edge of zone 32",
        input: r#"{"lat":60.3913,"lon":5.3221}"#,
        source: "Krüger-series transverse Mercator on WGS 84, zone 32",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.arc-to-chord",
            reason: "next",
        },
        Related {
            id: "geodesy.magnetic.grivation",
            reason: "next",
        },
    ],
    assumptions: &[UTM_FORWARD.assumptions[0]],
    sentence: "The grid scale factor here is {scale}, in UTM zone {zone}.",
    limits: UTM_FORWARD.limits,
    run: run_scale_factor,
    ..ToolDef::BLANK
};

/// Which single figure of the UTM projection a focused operation leads with.
#[derive(Clone, Copy, PartialEq)]
enum GridFigure {
    Convergence,
    Scale,
}

fn run_convergence(ctx: &mut Ctx) -> Result<Json, ToolError> {
    utm_forward(ctx, Some(GridFigure::Convergence))
}

fn run_scale_factor(ctx: &mut Ctx) -> Result<Json, ToolError> {
    utm_forward(ctx, Some(GridFigure::Scale))
}

fn run_utm_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    utm_forward(ctx, None)
}

fn utm_forward(ctx: &mut Ctx, figure: Option<GridFigure>) -> Result<Json, ToolError> {
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
    if let Some(figure) = figure {
        ctx.model = Some(format!(
            "Transverse Mercator, 6th-order Krüger series (Karney 2011), k0 = 0.9996, on {}",
            e.describe()
        ));
        let convergence = ("convergence", ctx.out("convergence", deg(g.convergence)));
        let scale = ("scale", Json::Num(g.scale));
        let mut out = if figure == GridFigure::Convergence {
            vec![convergence, scale]
        } else {
            vec![scale, convergence]
        };
        out.push(("zone", Json::Num(f64::from(zone))));
        out.push((
            "central_meridian",
            ctx.out("central_meridian", deg(6.0 * f64::from(zone) - 183.0)),
        ));
        return Ok(Json::obj(out));
    }
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
    let mut out = utm_json(ctx, &g);
    // The grid zone the point is in, when it is in its standard zone.
    if zone == standard {
        let (w, e, so, no) = utmups::zone_band_bounds(lat, lon);
        out.push(("band", Json::str(utmups::band_letter(lat).to_string())));
        out.push(("zone_west", ctx.out("zone_west", deg(w))));
        out.push(("zone_east", ctx.out("zone_east", deg(e))));
        out.push(("band_south", ctx.out("band_south", deg(so))));
        out.push(("band_north", ctx.out("band_north", deg(no))));
    }
    Ok(Json::obj(out))
}

pub static UTM_INVERSE: ToolDef = ToolDef {
    id: "geodesy.utm.inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "UTM to latitude and longitude",
    summary: "Converts a UTM zone, hemisphere, easting, and northing back to latitude and longitude, with the grid convergence and scale factor.",
    aliases: &["UTM to lat long", "UTM to geographic"],
    keywords: &[
        "UTM inverse",
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
    assumptions: &[
        Assumption {
            name: "UTM scale factor on the central meridian k0",
            value: "0.9996",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false easting",
            value: "500000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false northing in the southern hemisphere",
            value: "10000000",
            unit: "m",
            source: "nga-utmups",
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
    aliases: &["UPS converter", "lat long to UPS"],
    keywords: &["UPS", "polar", "stereographic", "arctic", "antarctic"],
    inputs: &[LAT, LON, E[0], E[1], E[2]],
    outputs: UTM_OUTPUTS,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for a position in the polar caps, where UTM stops: north of 84 degrees or south of 80. It gives the UPS easting and northing a military grid reference, a polar chart or a navigation system expects, with the grid convergence you need to turn a true bearing into a grid one.",
    limitations: "UPS covers only the caps: north of 84 degrees and south of 80. Between those and the equator, UTM is the projection, and this refuses rather than extrapolating a polar projection into latitudes it distorts badly. Grid north here is the prime meridian, not the meridian through your point, so the convergence is the longitude itself and is large -- at 120 degrees east a grid bearing and a true bearing differ by 120 degrees. The scale is 0.994 at the pole and grows outward, reaching 1 near 81 degrees 6 minutes, so a distance measured on the grid is not a distance on the ground without it.",
    warnings: &["INPUT_NORMALIZED"],
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
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "next",
        },
    ],
    assumptions: &[
        Assumption {
            name: "UPS scale factor at the pole k0",
            value: "0.994",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS false easting and northing",
            value: "2000000",
            unit: "m",
            source: "nga-utmups",
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
    keywords: &["UPS inverse", "polar", "inverse"],
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
    stability: Stability::Stable,
    when_to_use: "Use this to turn a UPS easting and northing back into latitude and longitude: a polar grid reference off a chart, a logged position, or the output of a system that works in the polar caps. Say which hemisphere it came from, since the two caps use the same numbers.",
    limitations: "The hemisphere is required and is not guessed: 2,000,000 by 2,000,000 is the north pole and the south pole both, and every other pair of numbers is equally ambiguous. Near the pole a longitude is a very short distance on the ground, so a coordinate rounded to the metre fixes the longitude only loosely up there -- at 89.99 degrees a metre of easting is five thousand times the longitude it is at the equator. At the pole itself the longitude has no meaning at all.",
    warnings: &["UNIT_ASSUMED"],
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
    related: &[
        Related {
            id: "geodesy.ups.forward",
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
    assumptions: &[
        Assumption {
            name: "UPS scale factor at the pole k0",
            value: "0.994",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS false easting and northing",
            value: "2000000",
            unit: "m",
            source: "nga-utmups",
        },
    ],
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
    stability: Stability::Stable,
    when_to_use: "Use this before projecting anything to UTM, or when reading a grid reference someone else produced: it gives the zone, the MGRS latitude band and the zone's central meridian for a point, with the Norway and Svalbard exceptions applied rather than left for you to remember.",
    limitations: "Two exceptions break the plain six-degree rule, and both are included: zone 32 is widened over southern Norway, and the even zones vanish over Svalbard. A point near any zone boundary is a point where two people using different software may disagree, so check the boundary rather than the answer. At exactly 84 degrees north this keeps the point in UTM, symmetric with the 80 degrees south edge; GeographicLib switches to UPS there instead, and both are defensible readings of the published range. Beyond those limits the zone is 0 and the band is a polar one, which means UPS is the projection, not UTM.",
    warnings: &["INPUT_NORMALIZED"],
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
    related: &[
        Related {
            id: "geodesy.utm.forward",
            reason: "next",
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

/// A corner of the referenced square's outline.
const SQUARE_ROW: &[Field] = &[lat_out("lat", "Latitude"), lon_out("lon", "Longitude")];

/// The square's outline in latitude and longitude, from its south-west corner
/// on the grid: each side sampled at four points along the grid line, so a
/// large square draws with the curve its sides have on the globe.
fn square_outline(ell: Ellipsoid, zone: u8, north: bool, e0: f64, n0: f64, size: f64) -> Json {
    const STEPS: usize = 4;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)];
    let mut rows = Vec::with_capacity(4 * STEPS);
    for w in corners.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        for k in 0..STEPS {
            let t = k as f64 / STEPS as f64;
            let (e, n) = (
                e0 + size * (x0 + (x1 - x0) * t),
                n0 + size * (y0 + (y1 - y0) * t),
            );
            let (lat, lon) = if zone == 0 {
                utmups::ups_inverse(ell.a, ell.f, north, e, n)
            } else {
                utmups::utm_inverse(ell.a, ell.f, zone, north, e, n)
            };
            rows.push(Json::obj([
                (
                    "lat",
                    Json::obj([("value", Json::Num(lat)), ("unit", Json::str("deg"))]),
                ),
                (
                    "lon",
                    Json::obj([
                        ("value", Json::Num(wrap_lon(lon))),
                        ("unit", Json::str("deg")),
                    ]),
                ),
            ]));
        }
    }
    Json::Arr(rows)
}

/// One point of a grid line: the line it belongs to, then where it is.
const GRID_ROW: &[Field] = &[
    Field::new(
        "line",
        "Line",
        "Which grid line the point is on",
        Kind::Number {
            min: 1.0,
            max: 64.0,
        },
    )
    .precision(Precision::Decimals(0)),
    lat_out("lat", "Latitude"),
    lon_out("lon", "Longitude"),
];

/// The grid around a square (grid-references "Grid overlays on the canvas"):
/// the lines of the square's own size, or of 1 km for a smaller square (a
/// 1 m grid is invisible at any zoom that shows the map), three squares
/// either side, and, for a square smaller than 100 km, the 100 km square
/// that holds it. Each line is
/// straight on the grid and sampled every half square, so it curves as it
/// should in latitude and longitude. The lines run on past a zone edge, where
/// the next zone's grid takes over.
fn grid_lines(ell: Ellipsoid, zone: u8, north: bool, e0: f64, n0: f64, size: f64) -> Json {
    let size = size.max(1_000.0);
    let (e0, n0) = ((e0 / size).floor() * size, (n0 / size).floor() * size);
    let at = |e: f64, n: f64| {
        let (lat, lon) = if zone == 0 {
            utmups::ups_inverse(ell.a, ell.f, north, e, n)
        } else {
            utmups::utm_inverse(ell.a, ell.f, zone, north, e, n)
        };
        (lat, wrap_lon(lon))
    };
    let mut lines: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut straight = |a: (f64, f64), b: (f64, f64), step: f64| {
        let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) / step)
            .ceil()
            .max(1.0) as usize;
        lines.push(
            (0..=n)
                .map(|k| {
                    let t = k as f64 / n as f64;
                    at(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
                })
                .collect(),
        );
    };
    let (lo, hi) = (-3.0, 4.0);
    for k in (lo as i32)..=(hi as i32) {
        let k = k as f64;
        straight(
            (e0 + k * size, n0 + lo * size),
            (e0 + k * size, n0 + hi * size),
            size / 2.0,
        );
        straight(
            (e0 + lo * size, n0 + k * size),
            (e0 + hi * size, n0 + k * size),
            size / 2.0,
        );
    }
    if size < 100_000.0 {
        let (e1, n1) = ((e0 / 1e5).floor() * 1e5, (n0 / 1e5).floor() * 1e5);
        let c = [
            (e1, n1),
            (e1 + 1e5, n1),
            (e1 + 1e5, n1 + 1e5),
            (e1, n1 + 1e5),
            (e1, n1),
        ];
        for w in c.windows(2) {
            straight(w[0], w[1], 5_000.0);
        }
    }
    let mut rows = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for &(lat, lon) in line {
            rows.push(Json::obj([
                ("line", Json::Num((i + 1) as f64)),
                (
                    "lat",
                    Json::obj([("value", Json::Num(lat)), ("unit", Json::str("deg"))]),
                ),
                (
                    "lon",
                    Json::obj([("value", Json::Num(lon)), ("unit", Json::str("deg"))]),
                ),
            ]));
        }
    }
    Json::Arr(rows)
}

const GRID_OUT: Field = Field::new(
    "grid",
    "Grid lines",
    "The grid around the square: its own size (at least 1 km) three squares out, and the 100 km square that holds it",
    Kind::List {
        items: GRID_ROW,
        min: 0,
        max: 2_000,
    },
);

const SQUARE_OUT: Field = Field::new(
    "square",
    "Square outline",
    "The referenced square's corners and sides, south-west corner first",
    Kind::List {
        items: SQUARE_ROW,
        min: 16,
        max: 16,
    },
);

pub static MGRS_FORWARD: ToolDef = ToolDef {
    version: "1.3.0",
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
        E[0],
        E[1],
        E[2],
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
        SQUARE_OUT.optional(),
        GRID_OUT.optional(),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "NGA MGRS over UTM and UPS: AA lettering, or AL on the legacy Clarke 1866 and Bessel 1841 ellipsoids",
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
        kind: "polygon",
        map: &[("rings", "square"), ("grid", "grid")],
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
    assumptions: &[
        Assumption {
            name: "UTM scale factor on the central meridian k0",
            value: "0.9996",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false easting",
            value: "500000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false northing in the southern hemisphere",
            value: "10000000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS scale factor at the pole k0",
            value: "0.994",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS false easting and northing",
            value: "2000000",
            unit: "m",
            source: "nga-utmups",
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

/// The ellipsoid for an MGRS reference, and its lettering: NGA keeps the
/// older "AL" row lettering for references on the Clarke 1866 and Bessel
/// 1841 ellipsoids (and Clarke 1880, not in this catalog).
fn mgrs_ellipsoid(ctx: &mut Ctx) -> Result<(Ellipsoid, mgrs::Lettering), ToolError> {
    let e = Ellipsoid::from_ctx(ctx)?;
    e.geodesic()?;
    let lettering = if matches!(e.id, "clarke1866" | "bessel1841") {
        mgrs::Lettering::Al
    } else {
        mgrs::Lettering::Aa
    };
    ctx.context.push((
        "lettering",
        Json::str(if lettering == mgrs::Lettering::Al {
            "AL"
        } else {
            "AA"
        }),
    ));
    Ok((e, lettering))
}

fn run_mgrs_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = PRECISIONS
        .iter()
        .position(|x| Some(*x) == ctx.choice("precision").ok().flatten())
        .map_or(5, |i| i as i32 - 1);
    let (ell, lettering) = mgrs_ellipsoid(ctx)?;
    let g = utmups::forward_auto(ell.a, ell.f, lat, lon);
    let s = mgrs::encode_with(&g, lat, p, lettering).map_err(|e| {
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
        let size = mgrs::square_size(p);
        out.push(("square_size", ctx.out("square_size", meters(size))));
        // MGRS truncates, so the square's south-west corner is the position with the dropped digits zeroed.
        let (e0, n0) = (
            (g.easting / size).floor() * size,
            (g.northing / size).floor() * size,
        );
        out.push(("square", square_outline(ell, g.zone, g.north, e0, n0, size)));
        out.push(("grid", grid_lines(ell, g.zone, g.north, e0, n0, size)));
    }
    Ok(Json::obj(out))
}

pub static MGRS_INVERSE: ToolDef = ToolDef {
    version: "1.2.0",
    id: "geodesy.grid-ref.mgrs-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "MGRS to latitude and longitude",
    summary: "Decodes an MGRS or USNG grid reference to the south-west corner and center of the square it names, with the square's size.",
    aliases: &["MGRS to lat long", "decode MGRS"],
    keywords: &["MGRS", "USNG", "decode", "grid reference"],
    inputs: &[
        Field::new(
            "mgrs",
            "MGRS reference",
            "Like 17TNE8630977770 or 17T NE 86309 77770",
            Kind::Text { max_len: 32 },
        )
        .required()
        .core(),
        E[0],
        E[1],
        E[2],
    ],
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
        SQUARE_OUT,
    ],
    errors: &[ErrorCode::Unsupported],
    warnings: &["BAND_ADJUSTED", "EXPERIMENTAL_TOOL"],
    model: "NGA MGRS over UTM and UPS: AA lettering, or AL on the legacy Clarke 1866 and Bessel 1841 ellipsoids",
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
    visualization: &[
        Layer {
            kind: "point",
            map: &[("lat", "lat"), ("lon", "lon")],
        },
        Layer {
            kind: "polygon",
            map: &[("rings", "square")],
        },
    ],
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
    assumptions: &[
        Assumption {
            name: "UTM scale factor on the central meridian k0",
            value: "0.9996",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false easting",
            value: "500000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UTM false northing in the southern hemisphere",
            value: "10000000",
            unit: "m",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS scale factor at the pole k0",
            value: "0.994",
            unit: "1",
            source: "nga-utmups",
        },
        Assumption {
            name: "UPS false easting and northing",
            value: "2000000",
            unit: "m",
            source: "nga-utmups",
        },
    ],
    sentence: "The center of this {square_size} square is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_mgrs_inverse,
    ..ToolDef::BLANK
};

pub static GRID_ZONE_LOOKUP: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.grid-ref.grid-zone-lookup",
    title: "Grid zone and 100 km square in MGRS",
    summary: "The MGRS grid zone designator and 100 km square identifier for a latitude and longitude, the first five characters of any MGRS reference there.",
    aliases: &[
        "grid zone designator",
        "100 km square ID in MGRS",
        "which MGRS square am I in",
    ],
    keywords: &[
        "grid zone designator",
        "GZD",
        "100 km square",
        "USNG",
        "square identifier",
        "map sheet",
    ],
    inputs: &[LAT, LON, E[0], E[1], E[2]],
    outputs: &[
        Field::new(
            "square_100km",
            "Grid zone and 100 km square",
            "Like 17TNE: the start of every MGRS reference in the square",
            Kind::Text { max_len: 8 },
        ),
        Field::new(
            "grid_zone",
            "Grid zone designator",
            "UTM zone and latitude band, like 17T, or a UPS letter",
            Kind::Text { max_len: 4 },
        ),
        Field::new(
            "square_id",
            "100 km square identifier",
            "Two letters, column then row",
            Kind::Text { max_len: 2 },
        ),
    ],
    errors: MGRS_FORWARD.errors,
    warnings: &[],
    model: MGRS_FORWARD.model,
    accuracy: "Exact: the zone, band, and square follow from the point's UTM or UPS coordinates",
    when_to_use: "Use this to find which MGRS or USNG map square a place falls in: to label a map sheet, set up a GPS receiver's grid, or check that a reference someone gave is in the right area. Enter the latitude and longitude. The tool returns the grid zone designator, such as 17T, the 100 km square letters, such as NE, and the two together, which begin every reference in that square.",
    limitations: "Squares are cut on UTM or UPS grid lines, so a place near a zone or band edge can be in a different square than a nearby one, and Norway and Svalbard use the zone exceptions. Near the poles the designator is a single UPS letter. The lettering follows the datum: Clarke 1866 and Bessel 1841 use the older AL scheme. It gives the square, not a position within it.",
    references: MGRS_FORWARD.references,
    examples: &[Example {
        id: "primary",
        title: "Longyearbyen, Svalbard",
        input: r#"{"lat":78.2232,"lon":15.6267}"#,
        source: "NGA MGRS grid zones, including the Svalbard exception",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.zone",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-precision",
            reason: "next",
        },
    ],
    sentence: "The point is in grid zone {grid_zone}, 100 km square {square_id}: references there begin {square_100km}.",
    limits: &[("batchRows", 10_000)],
    run: run_grid_zone_lookup,
    ..ToolDef::BLANK
};

fn run_grid_zone_lookup(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (ell, lettering) = mgrs_ellipsoid(ctx)?;
    let g = utmups::forward_auto(ell.a, ell.f, lat, lon);
    let encode = |p: i32| {
        mgrs::encode_with(&g, lat, p, lettering).map_err(|e| {
            ToolError::new(
                ErrorCode::OutOfDomain,
                format!("This point cannot be encoded: {e}."),
            )
            .at("/lat")
        })
    };
    let (gzd, square) = (encode(-1)?, encode(0)?);
    let id = square[gzd.len()..].to_owned();
    Ok(Json::obj([
        ("square_100km", Json::str(&square)),
        ("grid_zone", Json::str(gzd)),
        ("square_id", Json::str(id)),
    ]))
}

const PRECISION_ROW: &[Field] = &[
    Field::new(
        "reference",
        "Reference",
        "The same square, at this precision",
        Kind::Text { max_len: 32 },
    ),
    Field::new("square", "Square", "Its size", Kind::Text { max_len: 16 }),
];

pub static MGRS_PRECISION: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.grid-ref.mgrs-precision",
    title: "Precision of an MGRS reference",
    summary: "How precise an MGRS or USNG reference is: the size of the square its digits name, and the same reference at every coarser precision.",
    aliases: &[
        "precision of an MGRS reference",
        "how precise is this grid reference",
        "digits of an MGRS reference in meters",
    ],
    keywords: &[
        "USNG",
        "precision",
        "digits",
        "square size",
        "truncate",
        "grid reference",
    ],
    inputs: &[MGRS_INVERSE.inputs[0], E[0], E[1], E[2]],
    outputs: &[
        Field::new(
            "precision",
            "Precision",
            "The size of the square the reference names",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "digits",
            "Digits per coordinate",
            "0 (100 km square) to 5 (1 m), or more",
            Kind::Number {
                min: 0.0,
                max: 10.0,
            },
        )
        .precision(Precision::Decimals(0)),
        MGRS_INVERSE.outputs[4],
        Field::new(
            "coarser",
            "Coarser references",
            "The same place at each coarser precision, truncated as MGRS requires",
            Kind::List {
                items: PRECISION_ROW,
                min: 0,
                max: 12,
            },
        ),
    ],
    errors: MGRS_INVERSE.errors,
    warnings: &["BAND_ADJUSTED"],
    model: "An MGRS reference names a square: with n digits per coordinate it is 10^(5 − n) m across; a coarser reference keeps the first digits of each coordinate, never rounds (NGA MGRS)",
    accuracy: "Exact: precision is set by the count of digits",
    when_to_use: "Use this when a grid reference arrives and you need to know how much ground it pins down, or to shorten it for a radio call or a map label without moving it. Paste the reference with or without spaces. The tool returns the size of the square it names, such as 10 m for four digits a side, and the same reference cut to every coarser precision down to the 100 km square.",
    limitations: "MGRS references are truncated, never rounded, so a coarser reference names the square that contains the finer one, and its south-west corner is not the nearest point. The precision is what the digits claim, not how accurately the position was measured. References with more than five digits a side name squares smaller than a meter, which the standard allows but few receivers mean.",
    references: MGRS_INVERSE.references,
    examples: &[Example {
        id: "primary",
        title: "A 10-digit reference in Pittsburgh",
        input: r#"{"mgrs":"17T NE 86309 77770"}"#,
        source: "NGA MGRS: five digits a side name a 1 m square",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.mgrs-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.grid-ref.grid-zone-lookup",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.usng-inverse",
            reason: "alternative",
        },
    ],
    sentence: "The reference names a {precision} square, {digits} digits a side.",
    limits: &[("batchRows", 10_000)],
    run: run_mgrs_precision,
    ..ToolDef::BLANK
};

/// A square size in words: 100 km, 10 m, 1 cm.
fn square_words(size_m: f64) -> String {
    if size_m >= 1000.0 {
        format!("{} km", size_m / 1000.0)
    } else if size_m >= 1.0 {
        format!("{size_m} m")
    } else if size_m >= 0.01 {
        format!(
            "{} cm",
            gp_base::num::format_f64(size_m * 100.0).unwrap_or_default()
        )
    } else {
        format!(
            "{} mm",
            gp_base::num::format_f64(size_m * 1000.0).unwrap_or_default()
        )
    }
}

fn run_mgrs_precision(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("mgrs")?.expect("required");
    let (wgs, lettering) = mgrs_ellipsoid(ctx)?;
    // Decoding validates the reference; the rest is the text itself.
    let d = mgrs::decode_with(&s, wgs.a, wgs.f, lettering)
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
    let compact: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    // The coordinates are the digits after the last letter (the 100 km square's row).
    let digits_at = compact
        .rfind(|c: char| c.is_ascii_alphabetic())
        .map_or(0, |i| i + 1);
    let (head, tail) = compact.split_at(digits_at);
    let n = tail.len() / 2;
    let (east, north) = tail.split_at(n);
    let coarser: Vec<Json> = (0..n)
        .rev()
        .map(|k| {
            Json::obj([
                (
                    "reference",
                    Json::str(format!("{head}{}{}", &east[..k], &north[..k])),
                ),
                ("square", Json::str(square_words(10f64.powi(5 - k as i32)))),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("precision", Json::str(square_words(d.size))),
        ("digits", Json::Num(n as f64)),
        ("square_size", ctx.out("square_size", meters(d.size))),
        ("coarser", Json::Arr(coarser)),
    ]))
}

fn run_mgrs_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("mgrs")?.expect("required");
    let (wgs, lettering) = mgrs_ellipsoid(ctx)?;
    let d = mgrs::decode_with(&s, wgs.a, wgs.f, lettering)
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
        (
            "square",
            square_outline(wgs, d.zone, d.north, d.easting, d.northing, d.size),
        ),
    ]))
}

pub static TOOLS: &[&ToolDef] = &[
    &PARSE,
    &PARSE_DMS,
    &PARSE_DDM,
    &FORMAT,
    &FORMAT_DD,
    &FORMAT_DMS,
    &FORMAT_DDM,
    &BEARING_DIFFERENCE,
    &ANGLE_ARITHMETIC,
    &UTM_FORWARD,
    &CONVERGENCE,
    &SCALE_FACTOR,
    &UTM_INVERSE,
    &UTM_ZONE,
    &UPS_FORWARD,
    &UPS_INVERSE,
    &MGRS_FORWARD,
    &MGRS_INVERSE,
    &GRID_ZONE_LOOKUP,
    &MGRS_PRECISION,
    &magnetic::DECLINATION,
    &magnetic::UNCERTAINTY_ZONE,
    &magnetic::MODEL_COMPARE,
    &magnetic::TRUE_TO_MAGNETIC,
    &magnetic::MAGNETIC_TO_TRUE,
    &magnetic::GRIVATION,
    &magnetic::ISOGONIC,
    &spcs::FORWARD,
    &spcs::INVERSE,
    &spcs::LOOKUP,
    &crs::SEARCH,
    &crs::TRANSFORM,
    &spcs2022::FORWARD,
    &spcs2022::INVERSE,
    &spcs::ARC_TO_CHORD,
    &methods::WEB_MERCATOR_FORWARD,
    &methods::WEB_MERCATOR_INVERSE,
    &methods::LCC_FORWARD,
    &methods::LCC_INVERSE,
    &methods::ALBERS_FORWARD,
    &methods::ALBERS_INVERSE,
    &methods::POLAR_FORWARD,
    &methods::POLAR_INVERSE,
    &methods::EQC_FORWARD,
    &methods::EQC_INVERSE,
    &methods::AEQD_FORWARD,
    &methods::AEQD_INVERSE,
    &methods::GNOMONIC_FORWARD,
    &methods::GNOMONIC_INVERSE,
    &methods::ORTHO_FORWARD,
    &methods::ORTHO_INVERSE,
    &methods::HOTINE_FORWARD,
    &methods::HOTINE_INVERSE,
    &methods::TM_FORWARD,
    &methods::TM_INVERSE,
    &methods::TM_EXACT_FORWARD,
    &methods::TM_EXACT_INVERSE,
    &geoid::GEOID_HEIGHT,
    &geoid::HEIGHT_CONVERT,
    &geoid::ELLIPSOIDAL_TO_ORTHOMETRIC,
    &geoid::ORTHOMETRIC_TO_ELLIPSOIDAL,
    &frames::PARAMETERS,
    &frames::RADII,
    &frames::MERIDIAN_ARC,
    &frames::DEGREE_LENGTHS,
    &frames::AUXILIARY,
    &frames::TO_ECEF,
    &frames::FROM_ECEF,
    &frames::TO_LOCAL,
    &frames::ENU_TO_AER,
    &frames::AER_TO_ENU,
    &frames::ENU_NED,
    &frames::ROTATION_MATRIX,
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
    &datum::TRANSFORM,
];

pub static REGISTRY: Registry = Registry {
    module: "geodesy",
    tools: TOOLS,
};

gp_base::export_module!("geodesy", REGISTRY);
