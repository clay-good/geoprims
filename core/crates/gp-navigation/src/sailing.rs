//! The sailings as Bowditch states them (add-marine-navigation, "sailings"):
//! plane, traverse, parallel, mid-latitude, Mercator, and great-circle with
//! the composite track, in the forms a navigator checks (l, p, DLo, m), with
//! the exact ellipsoidal answer beside each short form.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point;
use gp_geo::rhumb::Rhumb;
use libm::{acos, asin, atan2, atanh, cos, sin, sqrt, tan};

const BOWDITCH: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 13, The Sailings (§1302–1308)",
    url: "https://msi.nga.mil/Publications/APN",
};
const BOWDITCH_T6: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume II",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Table 6, Meridional Parts (WGS 84), and its explanation",
    url: "https://msi.nga.mil/Publications/APN",
};

const WGS84_F: f64 = 1.0 / 298.257_223_563;
/// Meters per nautical mile.
const NM: f64 = 1852.0;

fn nm_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "NM").expect("NM"),
    }
}
fn deg_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

const fn nm_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "NM",
        },
    )
    .precision(Precision::Decimals(1))
}
const fn course_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1))
    .angle_range("[0,360)")
}
const fn text_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 40 })
}

/// Meridional parts on WGS 84, in minutes of arc of the equator (Bowditch
/// Table 6, whose series this closed form sums exactly).
pub(crate) fn meridional_parts(lat: f64) -> f64 {
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let e = sqrt(e2);
    let s = sin(lat.to_radians());
    10800.0 / core::f64::consts::PI * (atanh(s) - e * atanh(e * s))
}

/// Latitude difference in minutes, labeled: 187.3' N.
fn l_text(l: f64) -> String {
    format!("{:.1}' {}", l.abs(), if l < 0.0 { "S" } else { "N" })
}
fn p_text(p: f64) -> String {
    format!("{:.1} NM {}", p.abs(), if p < 0.0 { "W" } else { "E" })
}
fn dlo_text(d: f64) -> String {
    format!("{:.1}' {}", d.abs(), if d < 0.0 { "W" } else { "E" })
}
fn lat_text(v: f64) -> String {
    dms::format(v, Axis::Lat, Style::Ddm, 1, true)
}
fn lon_text(v: f64) -> String {
    dms::format(v, Axis::Lon, Style::Ddm, 1, true)
}

fn angle_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    point::plain_angle(ctx, name)
}
fn nm_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let u = units::by_symbol(QT::Length, "NM").expect("NM");
    Ok(ctx.quantity(name)?.map(|q| q.to(u)))
}

// ---------------------------------------------------------------- plane

pub static PLANE: ToolDef = ToolDef {
    id: "navigation.sailing.plane",
    title: "Plane sailing",
    summary: "Difference of latitude and departure from a course and distance, or the course and distance from them, by Bowditch's plane sailing.",
    aliases: &[
        "plane sailing",
        "difference of latitude and departure",
        "d lat and departure",
    ],
    keywords: &[
        "plane",
        "sailing",
        "departure",
        "latitude",
        "course",
        "distance",
        "bowditch",
    ],
    inputs: &[
        course_field("course", "Course", "True course, like 005°").core(),
        nm_field("distance", "Distance", "Run along the course, like 188 NM").core(),
        nm_field(
            "lat_difference",
            "Difference of latitude",
            "North positive, in minutes or NM, like 187.3",
        ),
        nm_field("departure", "Departure", "East positive, in NM, like 16.4"),
    ],
    outputs: &[
        text_out("answer", "Answer", "As Bowditch would write it"),
        nm_field(
            "lat_difference",
            "Difference of latitude (l)",
            "North positive; minutes of latitude",
        ),
        nm_field("departure", "Departure (p)", "East positive"),
        course_field("course", "Course", "True"),
        nm_field("distance", "Distance", ""),
        text_out("l_label", "l", "As Bowditch labels it"),
        text_out("p_label", "p", "As Bowditch labels it"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "l = D cos C, p = D sin C; or tan C = p / l, D = √(l² + p²) (Bowditch §1302)",
    accuracy: "Exact for the plane triangle; the sailing treats a short run as flat, which Bowditch limits to runs of a few hundred miles",
    when_to_use: "Use this to turn a course and distance run into how far north or south and east or west the ship went, or back again, as in a navigation exam or a quick check of a dead-reckoning plot. Enter a course and distance to get the difference of latitude and the departure, or enter those two to get the course and distance; the answer is labeled N or S and E or W the way Bowditch writes it.",
    limitations: "Plane sailing treats the Earth as flat over the run, so it gives difference of latitude and departure, not longitude; use parallel or mid-latitude sailing to turn departure into longitude. For long runs use Mercator or great-circle sailing.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "188.0 NM on course 005°",
        input: r#"{"course":"5 deg","distance":"188 NM"}"#,
        source: "Bowditch 2024 §1302: l = 187.3' N, p = 16.4 NM E",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.sailing.traverse",
            reason: "next",
        },
        Related {
            id: "navigation.sailing.mid-latitude",
            reason: "next",
        },
        Related {
            id: "navigation.rhumb.direct",
            reason: "alternative",
        },
    ],
    sentence: "The answer: {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_plane,
    ..ToolDef::BLANK
};

fn plane_out(ctx: &mut Ctx, answer: String, l: f64, p: f64, c: f64, d: f64) -> Json {
    Json::obj([
        ("answer", Json::str(answer)),
        ("lat_difference", ctx.out("lat_difference", nm_q(l))),
        ("departure", ctx.out("departure", nm_q(p))),
        ("course", ctx.out("course", deg_q(c))),
        ("distance", ctx.out("distance", nm_q(d))),
        ("l_label", Json::str(l_text(l))),
        ("p_label", Json::str(p_text(p))),
    ])
}

fn run_plane(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (c, d) = (angle_in(ctx, "course")?, nm_in(ctx, "distance")?);
    let (l, p) = (nm_in(ctx, "lat_difference")?, nm_in(ctx, "departure")?);
    match (c, d, l, p) {
        (Some(c), Some(d), _, _) => {
            if d < 0.0 {
                return Err(ToolError::invalid(
                    "/distance",
                    "The distance cannot be negative.",
                ));
            }
            let c = wrap_azimuth(c);
            let (l, p) = (d * cos(c.to_radians()), d * sin(c.to_radians()));
            let answer = format!("l = {}, p = {}", l_text(l), p_text(p));
            if ctx.explaining() {
                ctx.step(
                    "Difference of latitude",
                    "l = D cos C",
                    format!("{d} × cos {c}°"),
                    l_text(l),
                );
                ctx.step(
                    "Departure",
                    "p = D sin C",
                    format!("{d} × sin {c}°"),
                    answer.clone(),
                );
            }
            Ok(plane_out(ctx, answer, l, p, c, d))
        }
        (_, _, Some(l), Some(p)) => {
            let d = l.hypot(p);
            let c = wrap_azimuth(atan2(p, l).to_degrees());
            let answer = format!("C = {c:05.1}° T, D = {d:.1} NM");
            if ctx.explaining() {
                ctx.step(
                    "Course",
                    "tan C = p / l",
                    format!("{} / {}", p_text(p), l_text(l)),
                    format!("{c:.1}°"),
                );
                ctx.step(
                    "Distance",
                    "D = √(l² + p²)",
                    format!("l {l:.1}, p {p:.1}"),
                    answer.clone(),
                );
            }
            Ok(plane_out(ctx, answer, l, p, c, d))
        }
        _ => Err(ToolError::invalid(
            "/course",
            "Give a course and distance, or a difference of latitude and a departure.",
        )),
    }
}

// ---------------------------------------------------------------- traverse

const LEG: &[Field] = &[
    course_field("course", "Course", "True, like 158°").required(),
    nm_field("distance", "Distance", "Like 15.5 NM").required(),
];

pub static TRAVERSE: ToolDef = ToolDef {
    id: "navigation.sailing.traverse",
    title: "Traverse sailing",
    summary: "The single course and distance made good over several legs, by summing each leg's difference of latitude and departure (Bowditch's traverse sailing).",
    aliases: &[
        "traverse sailing",
        "course made good over legs",
        "traverse table",
    ],
    keywords: &[
        "traverse",
        "sailing",
        "legs",
        "course",
        "made",
        "good",
        "departure",
    ],
    inputs: &[Field::new(
        "legs",
        "Legs",
        "Course and distance of each leg, like 158° for 15.5 NM",
        Kind::List {
            items: LEG,
            min: 1,
            max: 1_000,
        },
    )
    .required()
    .core()],
    outputs: &[
        text_out("answer", "Answer", "As Bowditch would write it"),
        course_field("course", "Course made good", "True"),
        nm_field("distance", "Distance made good", ""),
        nm_field(
            "lat_difference",
            "Difference of latitude (l)",
            "North positive",
        ),
        nm_field("departure", "Departure (p)", "East positive"),
        text_out("l_label", "l", ""),
        text_out("p_label", "p", ""),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Each leg l = D cos C, p = D sin C; sums Σl, Σp; course made good tan C = Σp / Σl, distance √(Σl² + Σp²) (Bowditch §1303)",
    accuracy: "Exact sums; Bowditch's traverse tables round each leg to 0.1 NM, so its totals can differ by a few tenths",
    when_to_use: "Use this to find the single course and distance made good after several legs, such as a sailing vessel tacking or a ship on a survey pattern. A current can be added as one more leg: its set as the course and its drift times the hours as the distance. To find a bearing and distance from a light rather than from the start, enter the bearing and distance from the light to the start as the first leg, as Bowditch suggests.",
    limitations: "It is plane sailing over every leg, so it suits runs of a few hundred miles; it gives departure, not longitude, so finish with parallel or mid-latitude sailing to get the position. Leg order does not change the answer, and leeway and current are only counted if you enter them as legs.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "Bowditch's five-leg traverse",
        input: r#"{"legs":[{"course":"158 deg","distance":"15.5 NM"},{"course":"135 deg","distance":"33.7 NM"},{"course":"259 deg","distance":"16.1 NM"},{"course":"293 deg","distance":"39.0 NM"},{"course":"169 deg","distance":"40.4 NM"}]}"#,
        source: "Bowditch 2024 §1303: 192.3° T, 67.3 NM",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.sailing.plane",
            reason: "parent",
        },
        Related {
            id: "navigation.sailing.mid-latitude",
            reason: "next",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "alternative",
        },
    ],
    sentence: "Made good: {answer}.",
    limits: &[("batchRows", 1_000)],
    run: run_traverse,
    ..ToolDef::BLANK
};

fn run_traverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let nmu = units::by_symbol(QT::Length, "NM").expect("NM");
    let (mut l, mut p) = (0.0, 0.0);
    let rows = ctx.rows("legs")?;
    for (i, r) in rows.iter().enumerate() {
        let c = ctx
            .row_quantity("legs", i, r, "course")?
            .expect("required")
            .to(deg);
        let d = ctx
            .row_quantity("legs", i, r, "distance")?
            .expect("required")
            .to(nmu);
        if d < 0.0 {
            return Err(ToolError::invalid(
                &format!("/legs/{i}/distance"),
                "A distance cannot be negative.",
            ));
        }
        l += d * cos(c.to_radians());
        p += d * sin(c.to_radians());
    }
    let d = l.hypot(p);
    let c = wrap_azimuth(atan2(p, l).to_degrees());
    let answer = format!("C = {c:05.1}° T, D = {d:.1} NM");
    if ctx.explaining() {
        ctx.step(
            "Sums",
            "Σl = Σ D cos C, Σp = Σ D sin C",
            format!("{} legs", rows.len()),
            format!("{}, {}", l_text(l), p_text(p)),
        );
        ctx.step(
            "Course and distance made good",
            "tan C = Σp / Σl, D = √(Σl² + Σp²)",
            format!("{p:.1} / {l:.1}"),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("course", ctx.out("course", deg_q(c))),
        ("distance", ctx.out("distance", nm_q(d))),
        ("lat_difference", ctx.out("lat_difference", nm_q(l))),
        ("departure", ctx.out("departure", nm_q(p))),
        ("l_label", Json::str(l_text(l))),
        ("p_label", Json::str(p_text(p))),
    ]))
}

// ---------------------------------------------------------------- parallel

pub static PARALLEL: ToolDef = ToolDef {
    id: "navigation.sailing.parallel",
    title: "Parallel sailing",
    summary: "Departure from difference of longitude on a parallel of latitude, or the reverse: p = DLo cos L (Bowditch's parallel sailing).",
    aliases: &[
        "parallel sailing",
        "departure to difference of longitude",
        "dlo to departure",
    ],
    keywords: &[
        "parallel",
        "sailing",
        "departure",
        "longitude",
        "dlo",
        "east",
        "west",
    ],
    inputs: &[
        Field::new(
            "latitude",
            "Latitude",
            "Of the parallel, like 49°30'N",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core(),
        Field::new(
            "dlo",
            "Difference of longitude",
            "East positive, like 3°30'",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        nm_field("departure", "Departure", "East positive, like 136.4 NM"),
    ],
    outputs: &[
        text_out("answer", "Answer", "As Bowditch would write it"),
        nm_field("departure", "Departure (p)", "East positive"),
        Field::new(
            "dlo",
            "Difference of longitude",
            "East positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4)),
        text_out("dlo_label", "DLo", "In minutes, as Bowditch writes it"),
        text_out("p_label", "p", ""),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "p = DLo cos L, DLo = p sec L, with DLo in minutes and p in NM on Bowditch's sphere (Bowditch §1304)",
    accuracy: "Exact on the sphere of 1 NM per minute of arc; on WGS 84 a minute of longitude is up to 0.5% longer",
    when_to_use: "Use this when a ship runs due east or west along a parallel and you need the longitude change from the distance run, or the distance from the longitude change. It is the simplest of the spherical sailings and the step that mid-latitude sailing uses to turn departure into longitude. Enter the latitude and either the difference of longitude or the departure; the other comes back, labeled east or west.",
    limitations: "It holds only along a parallel; a course with any north or south component needs mid-latitude or Mercator sailing. It uses Bowditch's sphere, so it differs from the ellipsoid by a few tenths of a percent.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "DLo 3°30' on latitude 49°30' N",
        input: r#"{"latitude":"49°30'N","dlo":"3.5 deg"}"#,
        source: "Bowditch 2024 §1304: p = 136.4 NM E",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.sailing.mid-latitude",
            reason: "next",
        },
        Related {
            id: "navigation.sailing.plane",
            reason: "alternative",
        },
        Related {
            id: "navigation.rhumb.direct",
            reason: "alternative",
        },
    ],
    sentence: "Along the parallel: {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_parallel,
    ..ToolDef::BLANK
};

fn run_parallel(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let lat = point::lat_angle(ctx, "latitude")?;
    let k = cos(lat.to_radians());
    let (dlo, p) = match (angle_in(ctx, "dlo")?, nm_in(ctx, "departure")?) {
        (Some(dlo), _) => (dlo * 60.0, dlo * 60.0 * k),
        (None, Some(p)) => {
            if k < 1e-12 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "At a pole every meridian meets, so departure has no longitude.",
                )
                .at("/latitude"));
            }
            (p / k, p)
        }
        _ => {
            return Err(ToolError::invalid(
                "/dlo",
                "Give a difference of longitude or a departure.",
            ));
        }
    };
    let answer = format!("p = {}, DLo = {}", p_text(p), dlo_text(dlo));
    if ctx.explaining() {
        ctx.step(
            "Latitude",
            "the parallel sailed",
            lat_text(lat),
            format!("cos L = {k:.5}"),
        );
        ctx.step(
            "Departure and DLo",
            "p = DLo cos L",
            format!("on {}", lat_text(lat)),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("departure", ctx.out("departure", nm_q(p))),
        ("dlo", ctx.out("dlo", deg_q(dlo / 60.0))),
        ("dlo_label", Json::str(dlo_text(dlo))),
        ("p_label", Json::str(p_text(p))),
    ]))
}

// ---------------------------------------------------------------- mid-latitude and Mercator

const POS_INPUTS: [Field; 6] = [
    point::lat_field("lat1", "Start latitude"),
    point::lon_field("lon1", "Start longitude"),
    Field::new(
        "lat2",
        "End latitude",
        "For course and distance between two places, like 22°25.6'N",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .angle_range("[-90,90]"),
    Field::new(
        "lon2",
        "End longitude",
        "Like 172°21.2'E",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    ),
    course_field("course", "Course", "For the place reached, like 070°"),
    nm_field(
        "distance",
        "Distance",
        "For the place reached, like 1,253 NM",
    ),
];

const SAILING_OUTPUTS: &[Field] = &[
    text_out("answer", "Answer", "As Bowditch would write it"),
    course_field("course", "Course", "True"),
    nm_field("distance", "Distance", ""),
    Field::new(
        "lat2",
        "Arrival latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
    Field::new(
        "lon2",
        "Arrival longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
    text_out("arrival", "Arrival", "Degrees and minutes"),
    nm_field(
        "lat_difference",
        "Difference of latitude (l)",
        "North positive",
    ),
    nm_field("departure", "Departure (p)", "East positive"),
    text_out("dlo_label", "DLo", "In minutes"),
    Field::new(
        "mean_latitude",
        "Mean latitude (Lm)",
        "Mid-latitude sailing",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(4))
    .optional(),
    Field::new(
        "m1",
        "M₁",
        "Meridional parts at the start (Mercator)",
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(1))
    .optional(),
    Field::new(
        "m2",
        "M₂",
        "Meridional parts at the end (Mercator)",
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(1))
    .optional(),
    Field::new(
        "meridional_difference",
        "m",
        "Meridional difference M₂ − M₁ (Mercator)",
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(1))
    .optional(),
    course_field(
        "rhumb_course",
        "Rhumb course on WGS 84",
        "The exact ellipsoidal rhumb line",
    ),
    nm_field("rhumb_distance", "Rhumb distance on WGS 84", ""),
    Field::new(
        "course_difference",
        "Course difference",
        "This sailing minus the ellipsoidal rhumb",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    nm_field(
        "distance_difference",
        "Distance difference",
        "This sailing minus the ellipsoidal rhumb",
    )
    .precision(Precision::Decimals(2)),
];

/// The two places, or the start with a course and distance.
enum Problem {
    Between(f64, f64, f64, f64),
    Reach(f64, f64, f64, f64),
}

fn problem(ctx: &mut Ctx) -> Result<Problem, ToolError> {
    let (lat1, lon1) = point::read(ctx, "lat1", "lon1")?;
    if ctx.is_set("lat2") || ctx.is_set("lon2") {
        let (lat2, lon2) = point::read(ctx, "lat2", "lon2")?;
        return Ok(Problem::Between(lat1, lon1, lat2, lon2));
    }
    match (angle_in(ctx, "course")?, nm_in(ctx, "distance")?) {
        (Some(c), Some(d)) if d >= 0.0 => Ok(Problem::Reach(lat1, lon1, wrap_azimuth(c), d)),
        (Some(_), Some(_)) => Err(ToolError::invalid(
            "/distance",
            "The distance cannot be negative.",
        )),
        _ => Err(ToolError::invalid(
            "/lat2",
            "Give the end position, or a course and distance from the start.",
        )),
    }
}

fn dlo_minutes(lon1: f64, lon2: f64) -> f64 {
    wrap_lon(lon2 - lon1) * 60.0
}

fn sailing_answer(c: f64, d: f64, lat2: f64, lon2: f64) -> String {
    format!(
        "C = {c:05.1}° T, D = {d:.1} NM, arriving {} {}",
        lat_text(lat2),
        lon_text(lon2)
    )
}

/// Shared output: the sailing's answer with the ellipsoidal rhumb beside it.
#[allow(clippy::too_many_arguments)]
fn sailing_out(
    ctx: &mut Ctx,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
    c: f64,
    d: f64,
    l: f64,
    p: f64,
    dlo: f64,
    extra: Vec<(&'static str, Json)>,
) -> Json {
    let g = Geodesic::wgs84();
    let rh = Rhumb::new(g.a, g.f);
    let (rs, rc) = rh.inverse(lat1, lon1, lat2, lon2);
    let rc = wrap_azimuth(rc);
    let dc = (c - rc + 540.0).rem_euclid(360.0) - 180.0;
    let mut out = vec![
        ("answer", Json::str(sailing_answer(c, d, lat2, lon2))),
        ("course", ctx.out("course", deg_q(c))),
        ("distance", ctx.out("distance", nm_q(d))),
        ("lat2", ctx.out("lat2", deg_q(lat2))),
        ("lon2", ctx.out("lon2", deg_q(lon2))),
        (
            "arrival",
            Json::str(format!("{} {}", lat_text(lat2), lon_text(lon2))),
        ),
        ("lat_difference", ctx.out("lat_difference", nm_q(l))),
        ("departure", ctx.out("departure", nm_q(p))),
        ("dlo_label", Json::str(dlo_text(dlo))),
    ];
    out.extend(extra);
    out.push(("rhumb_course", ctx.out("rhumb_course", deg_q(rc))));
    out.push(("rhumb_distance", ctx.out("rhumb_distance", nm_q(rs / NM))));
    out.push(("course_difference", ctx.out("course_difference", deg_q(dc))));
    out.push((
        "distance_difference",
        ctx.out("distance_difference", nm_q(d - rs / NM)),
    ));
    Json::obj(out)
}

pub static MID_LATITUDE: ToolDef = ToolDef {
    id: "navigation.sailing.mid-latitude",
    title: "Mid-latitude sailing",
    summary: "Course and distance between two places, or the place reached from a course and distance, by Bowditch's mid-latitude sailing, with the exact ellipsoidal rhumb line beside it.",
    aliases: &[
        "mid-latitude sailing",
        "middle latitude sailing",
        "mean latitude sailing",
    ],
    keywords: &[
        "mid-latitude",
        "middle",
        "sailing",
        "mean",
        "latitude",
        "departure",
        "bowditch",
    ],
    inputs: &POS_INPUTS,
    outputs: SAILING_OUTPUTS,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "l = D cos C, p = D sin C, Lm = (L₁ + L₂)/2, DLo = p sec Lm; or p = DLo cos Lm, tan C = p / l, D = l sec C (Bowditch §1305), with no middle-latitude correction as Bowditch advises; the exact rhumb line on WGS 84 (Karney) beside it",
    accuracy: "Bowditch's sphere of 1 NM per minute; the difference from the ellipsoidal rhumb is reported with every answer",
    when_to_use: "Use this for a run of a few hundred miles that does not cross the equator: the course and distance between two positions, or the position reached, the way Bowditch works it with the mean latitude. The exact rhumb line on WGS 84 is shown beside it, so you can see how much the short form gives away.",
    limitations: "Mid-latitude sailing assumes the meridians converge evenly and uses a sphere, so it drifts from the exact answer on long runs and in high latitudes; a run crossing the equator should be split there, as Bowditch says. For long passages use Mercator or great-circle sailing.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "1,253 NM on course 070° from 15°17.0'N 151°37.0'E",
        input: r#"{"lat1":"15°17.0'N","lon1":"151°37.0'E","course":"70 deg","distance":"1253 NM"}"#,
        source: "Bowditch 2024 §1305: 22°25.6'N; 172°21.1'E printed from DLo rounded to 1,244.1' (172°21.2'E at full precision)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-rhumb",
        map: &[("distance", "rhumb_distance")],
    }],
    related: &[
        Related {
            id: "navigation.sailing.mercator",
            reason: "alternative",
        },
        Related {
            id: "navigation.rhumb.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.sailing.plane",
            reason: "parent",
        },
    ],
    sentence: "The sailing gives {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_mid_latitude,
    ..ToolDef::BLANK
};

fn run_mid_latitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat1, lon1, lat2, lon2, c, d, l, p, dlo, lm) = match problem(ctx)? {
        Problem::Between(lat1, lon1, lat2, lon2) => {
            let l = (lat2 - lat1) * 60.0;
            let lm = (lat1 + lat2) / 2.0;
            let dlo = dlo_minutes(lon1, lon2);
            let p = dlo * cos(lm.to_radians());
            let c = wrap_azimuth(atan2(p, l).to_degrees());
            (lat1, lon1, lat2, lon2, c, l.hypot(p), l, p, dlo, lm)
        }
        Problem::Reach(lat1, lon1, c, d) => {
            let (l, p) = (d * cos(c.to_radians()), d * sin(c.to_radians()));
            let lat2 = lat1 + l / 60.0;
            if lat2.abs() > 90.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The run passes beyond the pole.",
                )
                .at("/distance"));
            }
            let lm = (lat1 + lat2) / 2.0;
            let k = cos(lm.to_radians());
            if k < 1e-12 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The mean latitude is a pole, where departure has no longitude.",
                )
                .at("/distance"));
            }
            let dlo = p / k;
            (
                lat1,
                lon1,
                lat2,
                wrap_lon(lon1 + dlo / 60.0),
                c,
                d,
                l,
                p,
                dlo,
                lm,
            )
        }
    };
    if ctx.explaining() {
        ctx.step(
            "Mean latitude",
            "Lm = (L₁ + L₂) / 2",
            format!("{} and {}", lat_text(lat1), lat_text(lat2)),
            lat_text(lm),
        );
        ctx.step(
            "Difference of longitude",
            "DLo = p sec Lm",
            format!("p = {}", p_text(p)),
            dlo_text(dlo),
        );
        ctx.step(
            "Course and distance",
            "tan C = p / l, D = √(l² + p²)",
            format!("l = {}", l_text(l)),
            sailing_answer(c, d, lat2, lon2),
        );
    }
    let extra = vec![("mean_latitude", ctx.out("mean_latitude", deg_q(lm)))];
    Ok(sailing_out(
        ctx, lat1, lon1, lat2, lon2, c, d, l, p, dlo, extra,
    ))
}

pub static MERCATOR: ToolDef = ToolDef {
    id: "navigation.sailing.mercator",
    title: "Mercator sailing",
    summary: "Course and distance between two places, or the place reached, by Bowditch's Mercator sailing with meridional parts on WGS 84, with the exact ellipsoidal rhumb line beside it.",
    aliases: &[
        "mercator sailing",
        "meridional parts",
        "mercator course and distance",
    ],
    keywords: &[
        "mercator",
        "sailing",
        "meridional",
        "parts",
        "rhumb",
        "course",
        "bowditch",
    ],
    inputs: &POS_INPUTS,
    outputs: SAILING_OUTPUTS,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "M = (10800/π)[atanh(sin L) − e·atanh(e sin L)] on WGS 84, the closed form of Bowditch Table 6's series; m = M₂ − M₁; tan C = DLo / m; D = l sec C, or DLo cos L along a parallel (Bowditch §1306); the exact rhumb line on WGS 84 (Karney) beside it",
    accuracy: "Course as exact as the meridional parts; distance on Bowditch's sphere of 1 NM per minute of latitude, which the ellipsoidal rhumb beside it corrects",
    when_to_use: "Use this for the rhumb-line course between two places far apart, the line a Mercator chart draws straight, worked as Bowditch does with meridional parts. It also gives the place reached on a course and distance.",
    limitations: "The distance is Bowditch's l sec C, which counts a minute of latitude as one nautical mile everywhere, so it differs from the exact rhumb distance on the ellipsoid by up to about 0.5%; both are shown. Near due east or west a small error in course makes a large error in distance, so the parallel form is used there.",
    references: &[BOWDITCH, BOWDITCH_T6],
    examples: &[Example {
        id: "primary",
        title: "Off Bermuda to Chesapeake Light",
        input: r#"{"lat1":"32°14.7'N","lon1":"66°28.9'W","lat2":"36°58.7'N","lon2":"75°42.2'W"}"#,
        source: "Bowditch 2024 §1306: m = 343.7, DLo = 553.3' W, 301.8° T; its 538.7 NM uses 58.18521° where atan(553.3/343.7) is 58.1526°, which gives 538.2 NM",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-rhumb",
        map: &[("distance", "rhumb_distance")],
    }],
    related: &[
        Related {
            id: "navigation.rhumb.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.sailing.great-circle",
            reason: "alternative",
        },
        Related {
            id: "navigation.sailing.mid-latitude",
            reason: "alternative",
        },
    ],
    sentence: "The sailing gives {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_mercator,
    ..ToolDef::BLANK
};

fn run_mercator(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat1, lon1, lat2, lon2, c, d, l, p, dlo) = match problem(ctx)? {
        Problem::Between(lat1, lon1, lat2, lon2) => {
            if lat1.abs() == 90.0 || lat2.abs() == 90.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "A pole has infinite meridional parts: a Mercator course cannot reach it.",
                )
                .at("/lat1"));
            }
            let m = meridional_parts(lat2) - meridional_parts(lat1);
            let dlo = dlo_minutes(lon1, lon2);
            let c = wrap_azimuth(atan2(dlo, m).to_degrees());
            let l = (lat2 - lat1) * 60.0;
            let d = if l.abs() < 1e-9 {
                dlo.abs() * cos(lat1.to_radians())
            } else {
                l / cos(c.to_radians())
            };
            (
                lat1,
                lon1,
                lat2,
                lon2,
                c,
                d.abs(),
                l,
                d.abs() * sin(c.to_radians()),
                dlo,
            )
        }
        Problem::Reach(lat1, lon1, c, d) => {
            let (l, p) = (d * cos(c.to_radians()), d * sin(c.to_radians()));
            let lat2 = lat1 + l / 60.0;
            if lat2.abs() >= 90.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The run reaches the pole, where a Mercator course cannot go.",
                )
                .at("/distance"));
            }
            let m = meridional_parts(lat2) - meridional_parts(lat1);
            let dlo = if l.abs() < 1e-9 {
                p / cos(lat1.to_radians())
            } else {
                m * tan(c.to_radians())
            };
            (
                lat1,
                lon1,
                lat2,
                wrap_lon(lon1 + dlo / 60.0),
                c,
                d,
                l,
                p,
                dlo,
            )
        }
    };
    let (m1, m2) = (meridional_parts(lat1), meridional_parts(lat2));
    if ctx.explaining() {
        ctx.step(
            "Meridional parts",
            "M = (10800/π)[atanh(sin L) − e atanh(e sin L)]",
            format!("{} and {}", lat_text(lat1), lat_text(lat2)),
            format!("M₁ {m1:.1}, M₂ {m2:.1}, m {:.1}", m2 - m1),
        );
        ctx.step(
            "Course",
            "tan C = DLo / m",
            format!("{} / {:.1}", dlo_text(dlo), m2 - m1),
            format!("{c:.1}°"),
        );
        ctx.step(
            "Distance",
            "D = l sec C",
            format!("l = {}", l_text(l)),
            sailing_answer(c, d, lat2, lon2),
        );
    }
    let extra = vec![
        ("m1", Json::Num(m1)),
        ("m2", Json::Num(m2)),
        ("meridional_difference", Json::Num(m2 - m1)),
    ];
    Ok(sailing_out(
        ctx, lat1, lon1, lat2, lon2, c, d, l, p, dlo, extra,
    ))
}

// ---------------------------------------------------------------- great circle

const GC_POINT: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
    Field::new(
        "lon",
        "Longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
];
const GC_LEG: &[Field] = &[
    text_out("leg", "Leg", ""),
    course_field("course", "Initial course", "True"),
    nm_field("distance", "Distance", ""),
];

pub static GREAT_CIRCLE: ToolDef = ToolDef {
    id: "navigation.sailing.great-circle",
    title: "Great-circle and composite sailing",
    summary: "The great-circle distance, initial and final courses, vertex, and points along the track by Bowditch's spherical formulas, with the WGS 84 geodesic beside it and a composite track under a limiting latitude.",
    aliases: &[
        "great circle sailing",
        "composite sailing",
        "great circle track points",
    ],
    keywords: &[
        "great",
        "circle",
        "composite",
        "vertex",
        "sailing",
        "limiting",
        "bowditch",
    ],
    inputs: &[
        point::lat_field("lat1", "Start latitude"),
        point::lon_field("lon1", "Start longitude"),
        point::lat_field("lat2", "End latitude"),
        point::lon_field("lon2", "End longitude"),
        Field::new(
            "limit",
            "Limiting latitude",
            "For a composite track, like 45°N",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "interval",
            "Points every",
            "Difference of longitude between track points, like 10°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
    ],
    outputs: &[
        text_out("answer", "Answer", "As Bowditch would write it"),
        nm_field(
            "distance",
            "Great-circle distance",
            "On Bowditch's sphere, 1 NM per minute of arc",
        ),
        course_field("course", "Initial course", "True"),
        course_field("final_course", "Final course", "True, on arrival"),
        Field::new(
            "vertex_lat",
            "Vertex latitude",
            "The track's highest latitude on the great circle",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "vertex_lon",
            "Vertex longitude",
            "",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4)),
        text_out(
            "vertex",
            "Vertex",
            "Degrees and minutes, and whether the track passes it",
        ),
        nm_field("geodesic_distance", "Geodesic on WGS 84", "Karney"),
        course_field("geodesic_course", "Geodesic initial course", ""),
        nm_field(
            "distance_difference",
            "Distance difference",
            "Sphere minus WGS 84",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "points",
            "Track points",
            "At the chosen longitude interval",
            Kind::List {
                items: GC_POINT,
                min: 0,
                max: 1_000,
            },
        ),
        Field::new(
            "legs",
            "Composite legs",
            "Great circle, parallel, great circle",
            Kind::List {
                items: GC_LEG,
                min: 0,
                max: 3,
            },
        )
        .optional(),
        nm_field("composite_distance", "Composite distance", "All three legs").optional(),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::DegenerateGeometry,
        ErrorCode::OutOfDomain,
    ],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "cos D = sin L₁ sin L₂ + cos L₁ cos L₂ cos DLo; tan C = sin DLo / (cos L₁ tan L₂ − sin L₁ cos DLo) (Bowditch §1307); vertex cos Lv = cos L₁ |sin C|; track points tan L = tan Lv cos(λ − λv); composite cos DLov = tan Lx / tan Lv with the legs tangent to the limiting parallel (§1308); the WGS 84 geodesic (Karney) beside it",
    accuracy: "Exact on the sphere of 1 NM per minute of arc; the WGS 84 geodesic beside it differs by up to about 0.5%",
    when_to_use: "Use this to plan an ocean passage on the shortest track: the distance, the course to start on, the vertex where the track is farthest from the equator, and points to steer for every so many degrees of longitude. Give a limiting latitude to keep the track out of high latitudes with a composite sailing.",
    limitations: "Bowditch's great circle is on a sphere; the geodesic on WGS 84 beside it is the true shortest track and differs by up to about 0.5%. The course along a great circle changes continuously, so in practice it is sailed as a series of rhumb lines between the track points. Antipodal places have no single great circle.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "22°S 116°E to 20°S 031°E across the Indian Ocean",
        input: r#"{"lat1":"22 S","lon1":"116 E","lat2":"20 S","lon2":"31 E"}"#,
        source: "Bowditch 2024 §1307 example 1: 4,693.5 NM, 253° T; example 3: final course 289.4° T",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("distance", "geodesic_distance")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.spherical-inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.sailing.mercator",
            reason: "alternative",
        },
    ],
    sentence: "The great circle: {answer}.",
    limits: &[("batchRows", 1_000)],
    run: run_great_circle,
    ..ToolDef::BLANK
};

fn unit(lat: f64, lon: f64) -> [f64; 3] {
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    [cos(la) * cos(lo), cos(la) * sin(lo), sin(la)]
}

/// Spherical distance (minutes of arc) and initial course from 1 to 2.
fn sphere_inverse(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
    let (a, b) = (lat1.to_radians(), lat2.to_radians());
    let dl = wrap_lon(lon2 - lon1).to_radians();
    let cd = (sin(a) * sin(b) + cos(a) * cos(b) * cos(dl)).clamp(-1.0, 1.0);
    let c = atan2(
        sin(dl) * cos(b),
        cos(a) * sin(b) - sin(a) * cos(b) * cos(dl),
    );
    (acos(cd).to_degrees() * 60.0, wrap_azimuth(c.to_degrees()))
}

fn run_great_circle(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat1, lon1) = point::read(ctx, "lat1", "lon1")?;
    let (lat2, lon2) = point::read(ctx, "lat2", "lon2")?;
    let (p1, p2) = (unit(lat1, lon1), unit(lat2, lon2));
    let dot = p1[0] * p2[0] + p1[1] * p2[1] + p1[2] * p2[2];
    if dot < -1.0 + 1e-15 {
        return Err(ToolError::new(ErrorCode::DegenerateGeometry, "The places are antipodal: every great circle through them has the same length, so there is no single track.").at("/lat2"));
    }
    if (lat1 - lat2).abs() < 1e-12 && wrap_lon(lon2 - lon1).abs() < 1e-12 {
        return Err(ToolError::invalid("/lat2", "The two places are the same."));
    }
    let (d, c1) = sphere_inverse(lat1, lon1, lat2, lon2);
    let (_, back) = sphere_inverse(lat2, lon2, lat1, lon1);
    let c2 = wrap_azimuth(back + 180.0);
    // The vertex: the great circle's point nearest a pole, on the side the
    // track's middle lies.
    let n = [
        p1[1] * p2[2] - p1[2] * p2[1],
        p1[2] * p2[0] - p1[0] * p2[2],
        p1[0] * p2[1] - p1[1] * p2[0],
    ];
    let nn = sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
    let n = [n[0] / nn, n[1] / nn, n[2] / nn];
    let mut v = [-n[2] * n[0], -n[2] * n[1], 1.0 - n[2] * n[2]];
    let vn = sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    let mid_z = p1[2] + p2[2];
    let (vlat, vlon) = if vn < 1e-15 {
        // An equatorial track: every point is a vertex.
        (0.0, lon1)
    } else {
        v = [v[0] / vn, v[1] / vn, v[2] / vn];
        if mid_z < 0.0 {
            v = [-v[0], -v[1], -v[2]];
        }
        (
            asin(v[2].clamp(-1.0, 1.0)).to_degrees(),
            atan2(v[1], v[0]).to_degrees(),
        )
    };
    // Whether the vertex lies between the ends along the track.
    let within = {
        let ang = |a: [f64; 3], b: [f64; 3]| {
            acos((a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0))
        };
        let pv = unit(vlat, vlon);
        (ang(p1, pv) + ang(pv, p2) - ang(p1, p2)).abs() < 1e-9
    };
    let g = Geodesic::wgs84();
    let (gs, gaz, _, _): (f64, f64, f64, f64) = g.inverse(lat1, lon1, lat2, lon2);
    let gs_nm = gs / NM;
    // Points every interval of longitude, from the start toward the end.
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut points = vec![];
    if let Some(step) = ctx.quantity("interval")?.map(|q| q.to(deg)) {
        if step.is_nan() || step <= 0.0 {
            return Err(ToolError::invalid(
                "/interval",
                "The interval must be more than zero.",
            ));
        }
        let span = wrap_lon(lon2 - lon1);
        let k = (span.abs() / step).floor() as usize;
        if k > 1_000 {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                "At most 1,000 track points; use a wider interval.",
            )
            .at("/interval"));
        }
        if vn >= 1e-15 && vlat.abs() < 90.0 {
            let tv = tan(vlat.to_radians());
            for i in 1..=k {
                let lon = wrap_lon(lon1 + span.signum() * step * i as f64);
                if (span.signum() * step * i as f64).abs() >= span.abs() {
                    break;
                }
                let lat = libm::atan(tv * cos((lon - vlon).to_radians())).to_degrees();
                points.push(Json::obj([
                    ("lat", deg_q(lat).to_json()),
                    ("lon", deg_q(lon).to_json()),
                ]));
            }
        }
    }
    let answer = format!("D = {d:.1} NM, initial course {c1:05.1}° T");
    let mut out = vec![
        ("answer", Json::str(answer.clone())),
        ("distance", ctx.out("distance", nm_q(d))),
        ("course", ctx.out("course", deg_q(c1))),
        ("final_course", ctx.out("final_course", deg_q(c2))),
        ("vertex_lat", ctx.out("vertex_lat", deg_q(vlat))),
        ("vertex_lon", ctx.out("vertex_lon", deg_q(vlon))),
        (
            "vertex",
            Json::str(format!(
                "{} {}{}",
                lat_text(vlat),
                lon_text(vlon),
                if within {
                    ", on the track"
                } else {
                    ", beyond the track's ends"
                }
            )),
        ),
        (
            "geodesic_distance",
            ctx.out("geodesic_distance", nm_q(gs_nm)),
        ),
        (
            "geodesic_course",
            ctx.out("geodesic_course", deg_q(wrap_azimuth(gaz))),
        ),
        (
            "distance_difference",
            ctx.out("distance_difference", nm_q(d - gs_nm)),
        ),
        ("points", Json::Arr(points)),
    ];
    if let Some(lim) = point::plain_angle(ctx, "limit")? {
        let lim_abs = lim.abs();
        let same_side = lim.signum() == vlat.signum() || lim == 0.0;
        if !(0.0..90.0).contains(&lim_abs) || lim_abs == 0.0 {
            return Err(ToolError::invalid(
                "/limit",
                "The limiting latitude must be between 0° and 90°.",
            ));
        }
        if within && same_side && vlat.abs() > lim_abs {
            if lat1.abs() >= lim_abs
                || lat2.abs() >= lim_abs
                || lat1.signum() != lim.signum()
                || lat2.signum() != lim.signum()
            {
                return Err(ToolError::new(ErrorCode::OutOfDomain, "A composite track needs both ends on the same side of the equator as the limit and nearer the equator than it.").at("/limit"));
            }
            let lv = lim_abs.to_radians() * lim.signum();
            let dlov =
                |lat: f64| acos((tan(lat.to_radians()) / tan(lv)).clamp(-1.0, 1.0)).to_degrees();
            let east = wrap_lon(lon2 - lon1) >= 0.0;
            let s = if east { 1.0 } else { -1.0 };
            let (dv1, dv2) = (dlov(lat1), dlov(lat2));
            let (la, lb) = (wrap_lon(lon1 + s * dv1), wrap_lon(lon2 - s * dv2));
            let along = s * wrap_lon(lb - la);
            if along < 0.0 {
                return Err(ToolError::new(ErrorCode::OutOfDomain, "The two great circles tangent to the limit cross before reaching it; the plain great circle stays below the limit there.").at("/limit"));
            }
            let leg_d = |lat: f64| {
                acos((sin(lat.to_radians()) / sin(lv)).clamp(-1.0, 1.0)).to_degrees() * 60.0
            };
            let (d1, d3) = (leg_d(lat1), leg_d(lat2));
            let d2 = along * 60.0 * cos(lv);
            let (_, c_a) = sphere_inverse(lat1, lon1, lim, la);
            let (_, c_c) = sphere_inverse(lim, lb, lat2, lon2);
            let par = if east { 90.0 } else { 270.0 };
            let legs = vec![
                Json::obj([
                    (
                        "leg",
                        Json::str(format!(
                            "Great circle to {} {}",
                            lat_text(lim),
                            lon_text(la)
                        )),
                    ),
                    ("course", deg_q(c_a).to_json()),
                    ("distance", nm_q(d1).to_json()),
                ]),
                Json::obj([
                    (
                        "leg",
                        Json::str(format!("Along {} to {}", lat_text(lim), lon_text(lb))),
                    ),
                    ("course", deg_q(par).to_json()),
                    ("distance", nm_q(d2).to_json()),
                ]),
                Json::obj([
                    (
                        "leg",
                        Json::str(format!(
                            "Great circle to {} {}",
                            lat_text(lat2),
                            lon_text(lon2)
                        )),
                    ),
                    ("course", deg_q(c_c).to_json()),
                    ("distance", nm_q(d3).to_json()),
                ]),
            ];
            out.push(("legs", Json::Arr(legs)));
            out.push((
                "composite_distance",
                ctx.out("composite_distance", nm_q(d1 + d2 + d3)),
            ));
        }
    }
    if ctx.explaining() {
        ctx.step(
            "Distance",
            "cos D = sin L₁ sin L₂ + cos L₁ cos L₂ cos DLo",
            format!(
                "{} {} to {} {}",
                lat_text(lat1),
                lon_text(lon1),
                lat_text(lat2),
                lon_text(lon2)
            ),
            format!("{d:.1} NM"),
        );
        ctx.step(
            "Initial course",
            "tan C = sin DLo / (cos L₁ tan L₂ − sin L₁ cos DLo)",
            format!("final course {c2:.1}°"),
            answer.clone(),
        );
    }
    Ok(Json::obj(out))
}
