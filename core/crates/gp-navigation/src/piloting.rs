//! Piloting at the chart table (add-marine-navigation, "piloting"): the
//! current triangle, dead reckoning, a running fix, distance off by two
//! bearings, distance by vertical angle, and compass conversion. The current
//! triangle and the heading chain share their functions with the aviation
//! tools (`gp_geo::wind`, `gp_geo::compass`), so the two cannot disagree.

use geographiclib_rs::{DirectGeodesic, Geodesic};
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point;
use gp_geo::rhumb::Rhumb;
use gp_geo::{compass, wind};
use libm::{cos, sin, sqrt, tan};
use serde_json::Value;

use crate::sailing::meridional_parts;

const BOWDITCH: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 10, Dead Reckoning (§1008, estimated course and speed made good)",
    url: "https://msi.nga.mil/Publications/APN",
};
const BOWDITCH_PILOT: Reference = Reference {
    locator: "Chapter 11, Piloting (§1112 running fix; §1121 distance by two bearings, Table 18)",
    ..BOWDITCH
};
const BOWDITCH_DR: Reference = Reference {
    locator: "Chapter 10, Dead Reckoning; Chapter 13, The Sailings (§1306 Mercator sailing)",
    ..BOWDITCH
};
const BOWDITCH_VA: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume II",
    locator: "Tables 15 and 16, distance by vertical angle, and their explanation; Volume I §1209",
    ..BOWDITCH
};
const BOWDITCH_COMPASS: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume II",
    locator: "Chapter 4, Compass Error (§402, correcting and uncorrecting)",
    ..BOWDITCH
};

const NM: f64 = 1852.0;

fn deg_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}
fn nm_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "NM").expect("NM"),
    }
}
fn kt_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Speed, "kt").expect("kt"),
    }
}
const fn angle(name: &'static str, title: &'static str, help: &'static str) -> Field {
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
}
const fn course(name: &'static str, title: &'static str, help: &'static str) -> Field {
    angle(name, title, help).angle_range("[0,360)")
}
const fn knots(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Speed,
            unit: "kt",
        },
    )
    .precision(Precision::Decimals(1))
}
const fn nmf(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "NM",
        },
    )
    .precision(Precision::Decimals(2))
}
const fn text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 80 })
}

fn deg_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    point::plain_angle(ctx, name)
}
fn kt_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let u = units::by_symbol(QT::Speed, "kt").expect("kt");
    Ok(ctx.quantity(name)?.map(|q| q.to(u)))
}
fn nm_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let u = units::by_symbol(QT::Length, "NM").expect("NM");
    Ok(ctx.quantity(name)?.map(|q| q.to(u)))
}
fn hours_in(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let u = units::by_symbol(QT::Time, "h").expect("h");
    Ok(ctx.quantity(name)?.map(|q| q.to(u)))
}
fn c3(v: f64) -> String {
    format!("{:05.1}°", wrap_azimuth(v))
}

// ---------------------------------------------------------------- set and drift

pub static SET_AND_DRIFT: ToolDef = ToolDef {
    id: "navigation.piloting.set-and-drift",
    title: "Current triangle (set and drift)",
    summary: "The three current-triangle problems of Bowditch: track and speed made good from course, speed, set, and drift; the course to steer for a course made good; and the course and speed to use for a course and speed made good.",
    aliases: &["set and drift", "course to steer through a current", "current triangle"],
    keywords: &["current", "set", "drift", "course", "steer", "made", "good"],
    inputs: &[
        Field::new(
            "problem",
            "Problem",
            "Track made good from course and speed; course to steer at a speed; or course and speed to use",
            Kind::Choice(&["track", "course-to-steer", "course-and-speed"]),
        )
        .core(),
        course("course", "Course", "Steered through the water, or the course to make good, like 080°").required().core(),
        knots("speed", "Speed", "Through the water, or the speed to make good, like 10 kn").required().core(),
        course("set", "Set", "Direction the current flows toward, like 140°").required().core(),
        knots("drift", "Drift", "Speed of the current, like 2 kn").required().core(),
    ],
    outputs: &[
        text("answer", "Answer", ""),
        course("track", "Track made good", "Course over the ground"),
        knots("speed_made_good", "Speed made good", "Over the ground"),
        course("course_to_steer", "Course to steer", "Through the water"),
        knots("speed_to_use", "Speed through the water", ""),
        Field::new("current_angle", "Current correction", "Course to steer minus track", Kind::Quantity { q: QT::Angle, unit: "deg" }).precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "The vector triangle ground = water + current, solved by the same functions as the aviation wind triangle (gp_geo::wind): track from heading, heading for a course, or heading and speed for a ground vector (Bowditch §1008)",
    accuracy: "Exact for a steady current; Bowditch plots to 0.5° and 0.1 kn",
    when_to_use: "Use this to allow for a tidal or ocean current: what track and speed a ship actually makes good, which course to steer to stay on the planned track, or which course and speed to use to arrive on time. It is the marine form of the wind triangle and shares its arithmetic with the aviation tools.",
    limitations: "The current is taken as steady in set and drift over the run, which a tidal stream is not; re-solve as it changes. Leeway from the wind is not included unless you fold it into the set and drift. When the current is stronger than the ship's speed across the desired track, there is no course that holds it.",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "Course 080° at 10 kn, set 140°, drift 2 kn",
        input: r#"{"problem":"track","course":"80 deg","speed":"10 kt","set":"140 deg","drift":"2 kt"}"#,
        source: "Bowditch 2024 §1008 example 1: track 089°, speed made good 11.2 kn",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "vector-diagram", map: &[("value", "track")] }],
    related: &[
        Related { id: "aviation.wind.heading-groundspeed", reason: "alternative" },
        Related { id: "navigation.piloting.dead-reckoning", reason: "next" },
        Related { id: "navigation.sailing.traverse", reason: "alternative" },
    ],
    sentence: "Through the current: {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_set_and_drift,
    ..ToolDef::BLANK
};

fn run_set_and_drift(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let problem = ctx.choice("problem")?.unwrap_or("track");
    let c = wrap_azimuth(deg_in(ctx, "course")?.expect("required"));
    let v = kt_in(ctx, "speed")?.expect("required");
    let set = wrap_azimuth(deg_in(ctx, "set")?.expect("required"));
    let drift = kt_in(ctx, "drift")?.expect("required");
    if v.is_nan() || v <= 0.0 || drift.is_nan() || drift < 0.0 {
        return Err(ToolError::invalid(
            "/speed",
            "Speed must be more than zero and drift zero or more.",
        ));
    }
    // The wind functions take a wind FROM; a current is given TOWARD.
    let from = set + 180.0;
    let (cts, stw, track, smg) = match problem {
        "track" => {
            let (t, g) = wind::ground_vector(c, v, from, drift);
            (c, v, t, g)
        }
        "course-to-steer" => {
            let Some((wca, g)) = wind::heading_groundspeed(c, v, from, drift) else {
                return Err(ToolError::new(ErrorCode::NoSolution, "The current across the track is stronger than the ship's speed: no course holds this track.").at("/drift"));
            };
            if g <= 0.0 {
                return Err(ToolError::new(ErrorCode::NoSolution, "The current is stronger than the ship along the track: it would be carried backward.").at("/drift"));
            }
            (wrap_azimuth(c + wca), v, c, g)
        }
        _ => {
            let (h, s) = wind::air_vector(c, v, from, drift);
            (h, s, c, v)
        }
    };
    let answer = match problem {
        "track" => format!("track {}, speed made good {smg:.1} kn", c3(track)),
        "course-to-steer" => format!("course to steer {}, speed made good {smg:.1} kn", c3(cts)),
        _ => format!("course to steer {}, speed to use {stw:.1} kn", c3(cts)),
    };
    if ctx.explaining() {
        ctx.step(
            "Current",
            "set toward, drift",
            format!("{} at {drift:.1} kn", c3(set)),
            format!("from {}", c3(from)),
        );
        ctx.step(
            "Triangle",
            "ground = water + current",
            format!("{} at {v:.1} kn", c3(c)),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("track", ctx.out("track", deg_q(track))),
        ("speed_made_good", ctx.out("speed_made_good", kt_q(smg))),
        ("course_to_steer", ctx.out("course_to_steer", deg_q(cts))),
        ("speed_to_use", ctx.out("speed_to_use", kt_q(stw))),
        (
            "current_angle",
            ctx.out("current_angle", deg_q(wrap_lon(cts - track))),
        ),
    ]))
}

// ---------------------------------------------------------------- dead reckoning

pub static DEAD_RECKONING: ToolDef = ToolDef {
    id: "navigation.piloting.dead-reckoning",
    title: "Dead reckoning position",
    summary: "The position reached from a known position by course and speed over a time, or by course and distance, worked on the Mercator chart for short runs and along the rhumb line for longer ones.",
    aliases: &[
        "dead reckoning",
        "DR position",
        "position from course and speed",
    ],
    keywords: &[
        "dead",
        "reckoning",
        "position",
        "course",
        "speed",
        "time",
        "run",
    ],
    inputs: &[
        point::lat_field("lat", "Start latitude"),
        point::lon_field("lon", "Start longitude"),
        course("course", "Course", "True, like 050°")
            .required()
            .core(),
        knots("speed", "Speed", "Like 15 kn"),
        Field::new(
            "time",
            "Time run",
            "Like 2 h or 90 min",
            Kind::Quantity {
                q: QT::Time,
                unit: "h",
            },
        ),
        nmf(
            "distance",
            "Distance",
            "Instead of speed and time, like 30 NM",
        ),
    ],
    outputs: &[
        text("answer", "DR position", ""),
        Field::new(
            "lat",
            "DR latitude",
            "",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "lon",
            "DR longitude",
            "",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        nmf("distance", "Distance run", ""),
        text(
            "method",
            "Worked by",
            "Mercator chart, or rhumb line for longer runs",
        ),
        nmf(
            "method_difference",
            "Difference between the two",
            "Mercator chart against the exact rhumb line",
        )
        .precision(Precision::Decimals(3)),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Under 100 NM, Mercator sailing from the start (meridional parts on WGS 84: L₂ = L₁ + D cos C/60, DLo = m tan C), as the run is plotted on a Mercator chart; from 100 to 600 NM, the exact rhumb line on WGS 84 (Karney); the two compared",
    accuracy: "The Mercator chart and the exact rhumb line agree within a few hundredths of a mile on runs under 100 NM; the difference is reported",
    when_to_use: "Use this to plot a dead-reckoning position: where the ship will be, or should be, after running a course at a speed for a time. A DR is the basis for every estimated position and for planning the next fix.",
    limitations: "A DR allows for nothing but course and speed: current, leeway, and steering error move the real position, so plot an estimated position with the set and drift once it is known. Runs over 600 NM are refused; plan those as sailings.",
    references: &[BOWDITCH_DR],
    examples: &[Example {
        id: "primary",
        title: "From 36°55'N 76°00'W on 050° at 15 kn for 2 hours",
        input: r#"{"lat":"36°55'N","lon":"76°00'W","course":"50 deg","speed":"15 kt","time":"2 h"}"#,
        source: "A 30 NM run worked on the Mercator chart",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "navigation.piloting.set-and-drift",
            reason: "next",
        },
        Related {
            id: "navigation.sailing.mercator",
            reason: "parent",
        },
        Related {
            id: "navigation.rhumb.direct",
            reason: "alternative",
        },
    ],
    sentence: "The DR position is {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_dead_reckoning,
    ..ToolDef::BLANK
};

fn run_dead_reckoning(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat1, lon1) = point::read(ctx, "lat", "lon")?;
    let c = wrap_azimuth(deg_in(ctx, "course")?.expect("required"));
    let d = match (
        nm_in(ctx, "distance")?,
        kt_in(ctx, "speed")?,
        hours_in(ctx, "time")?,
    ) {
        (Some(d), _, _) => d,
        (None, Some(v), Some(t)) => v * t,
        _ => {
            return Err(ToolError::invalid(
                "/speed",
                "Give a speed and a time, or a distance.",
            ));
        }
    };
    if d.is_nan() || d < 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The run cannot be negative.",
        ));
    }
    if d > 600.0 {
        return Err(ToolError::new(ErrorCode::LimitExceeded, format!("A {d:.0} NM run is past the 600 NM a dead-reckoning plot is good for; work it as a sailing.")).at("/distance"));
    }
    let lat_m = lat1 + d * cos(c.to_radians()) / 60.0;
    if lat_m.abs() >= 89.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The run reaches the polar cap, where a Mercator plot fails.",
        )
        .at("/distance"));
    }
    let dlo = if (lat_m - lat1).abs() < 1e-12 {
        d * sin(c.to_radians()) / cos(lat1.to_radians())
    } else {
        (meridional_parts(lat_m) - meridional_parts(lat1)) * tan(c.to_radians())
    };
    let lon_m = wrap_lon(lon1 + dlo / 60.0);
    let g = Geodesic::wgs84();
    let rh = Rhumb::new(g.a, g.f).direct(lat1, lon1, c, d * NM);
    let gap: f64 = geographiclib_rs::InverseGeodesic::inverse(&g, lat_m, lon_m, rh.lat, rh.lon);
    let (lat2, lon2, method) = if d < 100.0 {
        (lat_m, lon_m, "Mercator chart (run under 100 NM)")
    } else {
        (
            rh.lat,
            rh.lon,
            "rhumb line on WGS 84 (run of 100 NM or more)",
        )
    };
    let answer = format!(
        "{} {}",
        dms::format(lat2, Axis::Lat, Style::Ddm, 1, true),
        dms::format(lon2, Axis::Lon, Style::Ddm, 1, true)
    );
    if ctx.explaining() {
        ctx.step(
            "Run",
            "D = speed × time",
            format!("{} for {d:.1} NM", c3(c)),
            format!("{d:.1} NM"),
        );
        ctx.step(
            "Position",
            "L₂ = L₁ + D cos C, DLo = m tan C",
            method.to_owned(),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("lat", ctx.out("lat", deg_q(lat2))),
        ("lon", ctx.out("lon", deg_q(lon2))),
        ("distance", ctx.out("distance", nm_q(d))),
        ("method", Json::str(method)),
        (
            "method_difference",
            ctx.out("method_difference", nm_q(gap / NM)),
        ),
    ]))
}

// ---------------------------------------------------------------- two bearings

/// Distance off at the second bearing and abeam, from relative bearings A
/// and B (degrees off the bow) and the run between them (Table 18).
fn two_bearings(a: f64, b: f64, run: f64) -> Option<(f64, f64)> {
    let (a, b) = (a.abs(), b.abs());
    if !(a > 0.0 && b > a && b < 180.0) {
        return None;
    }
    let d2 = run * sin(a.to_radians()) / sin((b - a).to_radians());
    Some((d2, d2 * sin(b.to_radians())))
}

pub static TWO_BEARING_FIX: ToolDef = ToolDef {
    id: "navigation.piloting.two-bearing-fix",
    title: "Distance off by two bearings on the bow",
    summary: "Distance from an object at the second bearing and when abeam, from two bearings relative to the bow and the run between them (Bowditch Table 18), with the doubling-the-angle and bow-and-beam presets.",
    aliases: &[
        "doubling the angle on the bow",
        "bow and beam bearing",
        "distance off by two bearings",
    ],
    keywords: &[
        "bearing", "bow", "beam", "doubling", "distance", "off", "abeam",
    ],
    inputs: &[
        Field::new(
            "preset",
            "Method",
            "Any two bearings, doubling the angle on the bow, or bow and beam (45° and 90°)",
            Kind::Choice(&["two-bearings", "doubling", "bow-and-beam"]),
        )
        .core(),
        angle(
            "first",
            "First relative bearing",
            "Degrees off the bow, like 30°",
        )
        .core(),
        angle(
            "second",
            "Second relative bearing",
            "Degrees off the bow, like 60°; twice the first when doubling",
        ),
        nmf(
            "run",
            "Run between bearings",
            "Distance made good, like 4.0 NM",
        )
        .core(),
        knots(
            "speed",
            "Speed",
            "Instead of the run, with the time, like 12 kn",
        ),
        Field::new(
            "time",
            "Time between bearings",
            "Like 10 min",
            Kind::Quantity {
                q: QT::Time,
                unit: "h",
            },
        ),
    ],
    outputs: &[
        text("answer", "Answer", ""),
        nmf("distance_second", "Distance at the second bearing", ""),
        nmf(
            "distance_abeam",
            "Distance abeam",
            "Predicted, if the course and speed hold",
        ),
        angle("first", "First relative bearing", ""),
        angle("second", "Second relative bearing", ""),
        text("assumption", "Assumes", ""),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Plane oblique triangle (Bowditch Table 18): distance at the second bearing = run × sin A / sin(B − A); abeam = that × sin B; doubling the angle (B = 2A) gives a distance equal to the run; bow and beam (45°, 90°) gives a distance abeam equal to the run",
    accuracy: "Exact for the plane triangle; Table 18 prints its factors to 0.01",
    when_to_use: "Use this when one charted object is in sight: take its bearing relative to the bow twice as you pass, note the run between, and get the distance off now and when it will be abeam. Doubling the angle on the bow and the bow-and-beam bearing are the classic shortcuts, where the distance off equals the run.",
    limitations: "It assumes no current and a steady course and speed between the bearings, using the run over the ground; a current across the course makes the distance wrong, so treat it as a running fix of modest accuracy. The second bearing must be broader on the bow than the first and on the same side.",
    references: &[BOWDITCH_PILOT],
    examples: &[Example {
        id: "primary",
        title: "Doubling the angle: 30° and 60° on the bow, 4.0 NM apart",
        input: r#"{"preset":"doubling","first":"30 deg","run":"4 NM"}"#,
        source: "Distance off 4.0 NM at the second bearing, 3.46 NM abeam",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.piloting.running-fix",
            reason: "alternative",
        },
        Related {
            id: "navigation.piloting.distance-by-vertical-angle",
            reason: "alternative",
        },
        Related {
            id: "navigation.piloting.compass-conversion",
            reason: "parent",
        },
    ],
    sentence: "Distance off: {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_two_bearing_fix,
    ..ToolDef::BLANK
};

fn run_value(ctx: &mut Ctx) -> Result<f64, ToolError> {
    match (
        nm_in(ctx, "run")?,
        kt_in(ctx, "speed")?,
        hours_in(ctx, "time")?,
    ) {
        (Some(r), _, _) if r > 0.0 => Ok(r),
        (Some(_), _, _) => Err(ToolError::invalid(
            "/run",
            "The run must be more than zero.",
        )),
        (None, Some(v), Some(t)) if v * t > 0.0 => Ok(v * t),
        _ => Err(ToolError::invalid(
            "/run",
            "Give the run between bearings, or a speed and the time between them.",
        )),
    }
}

fn run_two_bearing_fix(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let preset = ctx.choice("preset")?.unwrap_or("two-bearings");
    let (a, b) = match preset {
        "bow-and-beam" => (45.0, 90.0),
        "doubling" => {
            let a = deg_in(ctx, "first")?
                .ok_or_else(|| ToolError::invalid("/first", "Give the first relative bearing."))?;
            (a, 2.0 * a)
        }
        _ => {
            let a = deg_in(ctx, "first")?
                .ok_or_else(|| ToolError::invalid("/first", "Give the first relative bearing."))?;
            let b = deg_in(ctx, "second")?.ok_or_else(|| {
                ToolError::invalid("/second", "Give the second relative bearing.")
            })?;
            if a * b < 0.0 {
                return Err(ToolError::invalid(
                    "/second",
                    "Both bearings must be on the same side of the bow.",
                ));
            }
            (a, b)
        }
    };
    let run = run_value(ctx)?;
    let Some((d2, abeam)) = two_bearings(a, b, run) else {
        return Err(ToolError::invalid(
            "/second",
            "The second bearing must be broader on the bow than the first, and both under 180°.",
        ));
    };
    let answer = format!("{d2:.2} NM at the second bearing, {abeam:.2} NM abeam");
    if ctx.explaining() {
        ctx.step(
            "Distance at second bearing",
            "run × sin A / sin(B − A)",
            format!("{run:.2} NM, A {:.1}°, B {:.1}°", a.abs(), b.abs()),
            format!("{d2:.2} NM"),
        );
        ctx.step(
            "Distance abeam",
            "d₂ × sin B",
            format!("{d2:.2} × sin {:.1}°", b.abs()),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("distance_second", ctx.out("distance_second", nm_q(d2))),
        ("distance_abeam", ctx.out("distance_abeam", nm_q(abeam))),
        ("first", ctx.out("first", deg_q(a.abs()))),
        ("second", ctx.out("second", deg_q(b.abs()))),
        (
            "assumption",
            Json::str("no current, steady course and speed"),
        ),
    ]))
}

// ---------------------------------------------------------------- running fix

const LOP_POINT: &[Field] = &[
    Field::new(
        "line",
        "Line",
        "1 the first bearing advanced, 2 the second",
        Kind::Number { min: 1.0, max: 2.0 },
    )
    .precision(Precision::Decimals(0)),
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

pub static RUNNING_FIX: ToolDef = ToolDef {
    id: "navigation.piloting.running-fix",
    title: "Running fix from one object",
    summary: "A running fix from two true bearings of one charted object and the ship's course and speed between them: the earlier bearing advanced by the run and crossed with the later one, with the fix, the run, and both lines of position for the chart.",
    aliases: &[
        "running fix",
        "advance a line of position",
        "fix from two bearings of one light",
    ],
    keywords: &[
        "running", "fix", "bearing", "advance", "line", "position", "light",
    ],
    inputs: &[
        course(
            "course",
            "Course",
            "True, made good between the bearings, like 050°",
        )
        .required()
        .core(),
        course(
            "bearing1",
            "First bearing",
            "True bearing of the object, like 024°",
        )
        .required()
        .core(),
        course(
            "bearing2",
            "Second bearing",
            "True bearing of the object, like 359°",
        )
        .required()
        .core(),
        nmf("run", "Run between bearings", "Like 2.5 NM").core(),
        knots(
            "speed",
            "Speed",
            "Instead of the run, with the time, like 15 kn",
        ),
        Field::new(
            "time",
            "Time between bearings",
            "Like 10 min",
            Kind::Quantity {
                q: QT::Time,
                unit: "h",
            },
        ),
        Field::new(
            "object_lat",
            "Object latitude",
            "The charted object, to place the fix, like 36°55'N",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "object_lon",
            "Object longitude",
            "Like 76°00'W",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
    ],
    outputs: &[
        text("answer", "Fix", ""),
        nmf(
            "distance",
            "Distance from the object at the fix",
            "At the second bearing",
        ),
        nmf(
            "distance_abeam",
            "Distance abeam",
            "Predicted, if the course and speed hold",
        ),
        nmf("run", "Run between bearings", ""),
        Field::new(
            "lat",
            "Fix latitude",
            "",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .optional(),
        Field::new(
            "lon",
            "Fix longitude",
            "",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .optional(),
        Field::new(
            "lines",
            "Lines of position",
            "For the chart",
            Kind::List {
                items: LOP_POINT,
                min: 0,
                max: 4,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Relative bearings A = bearing₁ − course and B = bearing₂ − course; the object's distance at the second bearing run × sin A / sin(B − A), the solution of advancing the first line of position by the run (Bowditch §1112, §1121, Table 18); the fix placed from the object along the reciprocal of the second bearing by the WGS 84 geodesic",
    accuracy: "Exact for the plane triangle; as good as the bearings and the run made good",
    when_to_use: "Use this when only one charted object is in sight and you take its bearing twice: the first line of position is advanced along the course by the distance run and crossed with the second. Give the object's charted position to get the fix itself and the lines to plot.",
    limitations: "It assumes the course and speed made good between the bearings are known; an unknown current moves the fix, the classic weakness of a running fix. The bearings must be on the same side and the second broader on the bow than the first. Lines are drawn as geodesics, which on these scales are the straight lines of the chart.",
    references: &[BOWDITCH_PILOT],
    examples: &[Example {
        id: "primary",
        title: "A light bears 024° at 1130 and 359° at 1140, ship on 050° at 15 kn",
        input: r#"{"course":"50 deg","bearing1":"24 deg","bearing2":"359 deg","speed":"15 kt","time":"10 min","object_lat":"36°55'N","object_lon":"76°00'W"}"#,
        source: "Bowditch 2024 §1121: 2.6 NM at 1140 and 2.0 NM abeam (Table 18 factors 1.04 and 0.81, run 2.5 NM)",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polyline",
            map: &[("lines", "lines")],
        },
        Layer {
            kind: "point",
            map: &[("lat", "lat"), ("lon", "lon")],
        },
    ],
    related: &[
        Related {
            id: "navigation.piloting.two-bearing-fix",
            reason: "alternative",
        },
        Related {
            id: "navigation.piloting.dead-reckoning",
            reason: "parent",
        },
        Related {
            id: "navigation.piloting.compass-conversion",
            reason: "parent",
        },
    ],
    sentence: "Running fix: {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_running_fix,
    ..ToolDef::BLANK
};

fn run_running_fix(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = wrap_azimuth(deg_in(ctx, "course")?.expect("required"));
    let b1 = wrap_azimuth(deg_in(ctx, "bearing1")?.expect("required"));
    let b2 = wrap_azimuth(deg_in(ctx, "bearing2")?.expect("required"));
    let run = run_value(ctx)?;
    let (a, b) = (wrap_lon(b1 - c), wrap_lon(b2 - c));
    if a * b <= 0.0 {
        return Err(ToolError::invalid(
            "/bearing2",
            "Both bearings must be on the same side of the course.",
        ));
    }
    let Some((d2, abeam)) = two_bearings(a, b, run) else {
        return Err(ToolError::invalid(
            "/bearing2",
            "The second bearing must be broader on the bow than the first: the lines would not cross ahead.",
        ));
    };
    let mut out = vec![];
    let mut lines = vec![];
    let answer;
    if ctx.is_set("object_lat") || ctx.is_set("object_lon") {
        let (olat, olon) = point::read(ctx, "object_lat", "object_lon")?;
        let g = Geodesic::wgs84();
        let at = |lat: f64, lon: f64, az: f64, s: f64| -> (f64, f64) {
            let (la, lo, _): (f64, f64, f64) = g.direct(lat, lon, az, s * NM);
            (la, gp_base::angle::wrap_lon(lo))
        };
        let fix = at(olat, olon, b2 + 180.0, d2);
        // The first line of position, through the object, advanced by the run.
        let shift = at(olat, olon, c, run);
        let len = d2 * 1.6 + run;
        let row = |line: f64, p: (f64, f64)| {
            Json::obj([
                ("line", Json::Num(line)),
                ("lat", deg_q(p.0).to_json()),
                ("lon", deg_q(p.1).to_json()),
            ])
        };
        lines.push(row(1.0, shift));
        lines.push(row(1.0, at(shift.0, shift.1, b1 + 180.0, len)));
        lines.push(row(2.0, (olat, olon)));
        lines.push(row(2.0, at(olat, olon, b2 + 180.0, len)));
        answer = format!(
            "{} {}, {d2:.1} NM from the object",
            dms::format(fix.0, Axis::Lat, Style::Ddm, 1, true),
            dms::format(fix.1, Axis::Lon, Style::Ddm, 1, true)
        );
        out.push(("lat", ctx.out("lat", deg_q(fix.0))));
        out.push(("lon", ctx.out("lon", deg_q(fix.1))));
    } else {
        answer = format!("{d2:.1} NM from the object on {}", c3(b2));
    }
    if ctx.explaining() {
        ctx.step(
            "Relative bearings",
            "A = B₁ − C, B = B₂ − C",
            format!("{} and {} on course {}", c3(b1), c3(b2), c3(c)),
            format!("{:.1}° and {:.1}°", a.abs(), b.abs()),
        );
        ctx.step(
            "Distance at the second bearing",
            "run × sin A / sin(B − A)",
            format!("run {run:.2} NM"),
            answer.clone(),
        );
    }
    let mut res = vec![
        ("answer", Json::str(answer)),
        ("distance", ctx.out("distance", nm_q(d2))),
        ("distance_abeam", ctx.out("distance_abeam", nm_q(abeam))),
        ("run", ctx.out("run", nm_q(run))),
    ];
    res.extend(out);
    res.push(("lines", Json::Arr(lines)));
    Ok(Json::obj(res))
}

// ---------------------------------------------------------------- vertical angle

pub static VERTICAL_ANGLE: ToolDef = ToolDef {
    id: "navigation.piloting.distance-by-vertical-angle",
    title: "Distance by vertical sextant angle",
    summary: "Distance off an object of charted height from its vertical sextant angle: above the sea horizon for an object beyond it (Bowditch Table 15), or from its waterline for one within the horizon (Table 16).",
    aliases: &[
        "distance by vertical angle",
        "sextant distance off",
        "vertical sextant angle",
    ],
    keywords: &[
        "vertical",
        "angle",
        "sextant",
        "distance",
        "lighthouse",
        "height",
        "horizon",
    ],
    inputs: &[
        angle(
            "angle",
            "Corrected angle",
            "For index error and dip, like 0°30'",
        )
        .required()
        .core(),
        Field::new(
            "object_height",
            "Object height",
            "Charted height above sea level, like 120 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "eye_height",
            "Height of eye",
            "Above the water, like 20 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .core(),
        Field::new(
            "method",
            "Measured from",
            "The sea horizon (object beyond it), or the object's waterline (object within it)",
            Kind::Choice(&["sea-horizon", "waterline"]),
        ),
    ],
    outputs: &[
        text("answer", "Distance", ""),
        nmf("distance", "Distance off", ""),
        nmf("horizon", "Distance to the sea horizon", "1.17 √h"),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::NoSolution,
    ],
    warnings: &["BEYOND_HORIZON", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Sea horizon (Table 15): D = √((tan a / 0.0002419)² + (H − h) / 0.7349) − tan a / 0.0002419, the constants for terrestrial refraction; waterline (Table 16): the plane right triangle D = H / tan a, in NM of 6,076.1 ft; horizon 1.17 √h NM",
    accuracy: "Table 15's refraction constants hold for standard conditions; the result matches the tables to their 0.1 NM",
    when_to_use: "Use this to get a distance off a lighthouse, peak, or other object of known height with a sextant: measure the angle from the sea horizon up to the top, correct it for index error and dip, and enter the heights. For an object inside the horizon, measure from its waterline instead.",
    limitations: "Use the height of the light's center, not the tower's top, and remember charted heights are above mean high water, so the tide changes the real height a little. Refraction differing from standard skews the result. The sea-horizon form needs the object beyond the horizon and the waterline form needs it inside; the tool warns when the answer says otherwise.",
    references: &[BOWDITCH_VA],
    examples: &[Example {
        id: "primary",
        title: "An object 100 ft above the eye at 0°30'",
        input: r#"{"angle":"0°30'","object_height":"120 ft","eye_height":"20 ft"}"#,
        source: "Bowditch 2024 Vol. II Table 15: 1.8 NM",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.los.horizon",
            reason: "parent",
        },
        Related {
            id: "navigation.piloting.two-bearing-fix",
            reason: "alternative",
        },
        Related {
            id: "navigation.los.dip",
            reason: "parent",
        },
    ],
    sentence: "The object is {answer} off.",
    limits: &[("batchRows", 10_000)],
    run: run_vertical_angle,
    ..ToolDef::BLANK
};

fn run_vertical_angle(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let a = deg_in(ctx, "angle")?.expect("required");
    let ft = units::by_symbol(QT::Length, "ft").expect("ft");
    let big_h = ctx.req_quantity("object_height")?.to(ft);
    let h = ctx.quantity("eye_height")?.map_or(0.0, |q| q.to(ft));
    if big_h.is_nan() || big_h <= 0.0 || h.is_nan() || h < 0.0 {
        return Err(ToolError::invalid(
            "/object_height",
            "Heights must be positive.",
        ));
    }
    if !(-1.0..90.0).contains(&a) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "A corrected vertical angle runs from a little below the horizon up to under 90°.",
        )
        .at("/angle"));
    }
    let horizon = 1.17 * sqrt(h);
    let waterline = ctx.choice("method")? == Some("waterline");
    let d = if waterline {
        if a <= 0.0 {
            return Err(ToolError::new(
                ErrorCode::NoSolution,
                "From the waterline the angle must be above zero.",
            )
            .at("/angle"));
        }
        big_h / tan(a.to_radians()) / 6076.1
    } else {
        if big_h <= h {
            return Err(ToolError::new(
                ErrorCode::NoSolution,
                "The object must stand higher than the eye for the sea-horizon method.",
            )
            .at("/object_height"));
        }
        let t = tan(a.to_radians()) / 0.0002419;
        sqrt(t * t + (big_h - h) / 0.7349) - t
    };
    if waterline && d > horizon {
        ctx.warnings.push(Warning::new("BEYOND_HORIZON", format!("At {d:.1} NM the object's waterline lies beyond the {horizon:.1} NM sea horizon, so it cannot be seen: measure from the sea horizon instead.")));
    }
    let answer = format!("{d:.1} NM");
    if ctx.explaining() {
        ctx.step(
            "Sea horizon",
            "1.17 √h",
            format!("h = {h:.0} ft"),
            format!("{horizon:.1} NM"),
        );
        ctx.step(
            "Distance",
            if waterline {
                "D = H / tan a"
            } else {
                "D = √((tan a/0.0002419)² + (H − h)/0.7349) − tan a/0.0002419"
            },
            format!("H {big_h:.0} ft, h {h:.0} ft, a {a:.3}°"),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("distance", ctx.out("distance", nm_q(d))),
        ("horizon", ctx.out("horizon", nm_q(horizon))),
    ]))
}

// ---------------------------------------------------------------- compass conversion

const CARD_ROW: &[Field] = &[
    angle("heading", "Magnetic heading", "Like 090°").required(),
    Field::new(
        "deviation",
        "Deviation",
        "East positive, like -1.5 for 1.5° W",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
];

/// An east-positive angle from a number or text like "7 W", "7°W", "1.5 E".
fn ew_angle(ctx: &Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let bad = || {
        ToolError::invalid(
            &format!("/{name}"),
            "Give a number of degrees, east positive, or text like 7 W or 1.5° E.",
        )
    };
    match ctx.raw(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_f64().map(Some).ok_or_else(bad),
        Some(Value::String(s)) => {
            let t = s.trim().to_ascii_uppercase();
            let (num, sign) = if let Some(x) = t.strip_suffix('W') {
                (x, -1.0)
            } else if let Some(x) = t.strip_suffix('E') {
                (x, 1.0)
            } else {
                (t.as_str(), 1.0)
            };
            let num = num
                .trim()
                .trim_end_matches(['°', ' '])
                .trim_end_matches("DEG")
                .trim();
            let v = dms::parse_plain(num).map_err(|_| bad())?;
            Ok(Some(sign * v))
        }
        Some(_) => Err(bad()),
    }
}

fn ew(v: f64) -> String {
    if v.abs() < 1e-9 {
        "0.0°".to_owned()
    } else {
        format!("{:.1}° {}", v.abs(), if v < 0.0 { "W" } else { "E" })
    }
}

pub static COMPASS_CONVERSION: ToolDef = ToolDef {
    id: "navigation.piloting.compass-conversion",
    title: "Compass conversion (true, magnetic, compass)",
    summary: "Converts a heading or bearing among true, magnetic, and compass with variation and deviation (east and west as Bowditch signs them), the compass error, a deviation table interpolated by magnetic heading, and the relative bearing.",
    aliases: &[
        "compass conversion",
        "TVMDC",
        "true virtual makes dull companions",
    ],
    keywords: &[
        "compass",
        "variation",
        "deviation",
        "magnetic",
        "true",
        "error",
        "bearing",
    ],
    inputs: &[
        course("heading", "Ship's heading", "Like 215°")
            .required()
            .core(),
        Field::new(
            "heading_reference",
            "Heading is",
            "True, magnetic, or by compass",
            Kind::Choice(&["true", "magnetic", "compass"]),
        )
        .core(),
        Field::new(
            "variation",
            "Variation",
            "From the chart, like 7 W or -7",
            Kind::Text { max_len: 16 },
        )
        .required()
        .core(),
        Field::new(
            "deviation",
            "Deviation",
            "For this heading, like 1.5 W; or give a deviation table",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "deviation_table",
            "Deviation table",
            "Magnetic heading and east-positive deviation, like 090°, -1.5",
            Kind::List {
                items: CARD_ROW,
                min: 1,
                max: 72,
            },
        ),
        course(
            "bearing",
            "Bearing",
            "An object's bearing to convert, like 306.5°",
        ),
        Field::new(
            "bearing_reference",
            "Bearing is",
            "True, magnetic, or by compass",
            Kind::Choice(&["true", "magnetic", "compass"]),
        ),
    ],
    outputs: &[
        text("answer", "Answer", ""),
        course("true_heading", "True heading", ""),
        course("magnetic_heading", "Magnetic heading", ""),
        course("compass_heading", "Compass heading", ""),
        text("deviation", "Deviation", "On this heading"),
        text("compass_error", "Compass error", "Variation plus deviation"),
        course("true_bearing", "True bearing", "").optional(),
        course("magnetic_bearing", "Magnetic bearing", "").optional(),
        course("compass_bearing", "Compass bearing", "").optional(),
        course(
            "relative_bearing",
            "Relative bearing",
            "Clockwise from the bow",
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "True − variation = magnetic; magnetic − deviation = compass (east positive), the deviation read from the table at the magnetic heading by the function the aviation heading chain uses (gp_geo::compass); compass to true by fixed-point iteration on the table; a bearing takes the ship's heading's deviation (Bowditch Vol. II §402)",
    accuracy: "Exact arithmetic; a deviation table is interpolated linearly between its rows",
    when_to_use: "Use this for the TVMDC chain: turning a true course into the compass course to steer, or a compass bearing of a light into a true bearing to plot. Enter the variation from the chart and the deviation for the heading, or paste the ship's deviation table.",
    limitations: "Deviation belongs to the ship's heading, so a bearing uses the heading's deviation, not the bearing's. A deviation table is good only for the compass and the ship it was swung for, and only with degaussing as noted. Variation changes with place and year; use the chart's value or the magnetic tools.",
    references: &[BOWDITCH_COMPASS],
    examples: &[Example {
        id: "primary",
        title: "True heading 215°, variation 7° W, deviation 1.5° W; a light at 306.5° by compass",
        input: r#"{"heading":"215 deg","variation":"7 W","deviation":"1.5 W","bearing":"306.5 deg","bearing_reference":"compass"}"#,
        source: "Bowditch 2024 Vol. II §402: magnetic heading 222°, compass heading 223.5°, compass error 8.5° W; the light 305° magnetic, 298° true, 083° relative",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-chain",
            reason: "alternative",
        },
        Related {
            id: "geodesy.magnetic.true-to-magnetic",
            reason: "parent",
        },
        Related {
            id: "navigation.piloting.running-fix",
            reason: "next",
        },
    ],
    sentence: "The chain gives {answer}.",
    limits: &[("batchRows", 10_000)],
    run: run_compass,
    ..ToolDef::BLANK
};

fn run_compass(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let hdg = wrap_azimuth(deg_in(ctx, "heading")?.expect("required"));
    let reference = ctx.choice("heading_reference")?.unwrap_or("true");
    let var = ew_angle(ctx, "variation")?
        .ok_or_else(|| ToolError::invalid("/variation", "Give the variation."))?;
    if var.abs() > 180.0 {
        return Err(ToolError::invalid(
            "/variation",
            "Variation must be within 180° east or west.",
        ));
    }
    let dg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut card: Vec<(f64, f64)> = vec![];
    if let Some(d) = ew_angle(ctx, "deviation")? {
        card.push((0.0, d));
    } else if ctx.is_set("deviation_table") {
        let rows = ctx.rows("deviation_table")?;
        for (i, r) in rows.iter().enumerate() {
            let h = ctx
                .row_quantity("deviation_table", i, r, "heading")?
                .expect("required")
                .to(dg);
            let d = ctx
                .row_quantity("deviation_table", i, r, "deviation")?
                .expect("required")
                .to(dg);
            if d.abs() > 30.0 {
                return Err(ToolError::invalid(
                    &format!("/deviation_table/{i}/deviation"),
                    "A deviation over 30° means the compass needs adjusting, not interpolating.",
                ));
            }
            card.push((compass::norm(h), d));
        }
        card.sort_by(|a, b| a.0.total_cmp(&b.0));
        if card.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(ToolError::invalid(
                "/deviation_table",
                "Each heading appears once in a deviation table.",
            ));
        }
    }
    let (th, mh, dev, ch) = match reference {
        "true" => {
            let (m, d, c) = compass::true_to_compass(hdg, var, &card);
            (hdg, m, d, c)
        }
        "magnetic" => {
            let t = compass::norm(hdg + var);
            let (m, d, c) = compass::true_to_compass(t, var, &card);
            (t, m, d, c)
        }
        _ => {
            let (m, d, t) = compass::compass_to_true(hdg, var, &card);
            (t, m, d, hdg)
        }
    };
    let ce = var + dev;
    let mut answer = format!("TH {}, MH {}, CH {}, CE {}", c3(th), c3(mh), c3(ch), ew(ce));
    let mut extra = vec![];
    if let Some(b) = deg_in(ctx, "bearing")? {
        let b = wrap_azimuth(b);
        let (tb, mb, cb) = match ctx.choice("bearing_reference")?.unwrap_or("true") {
            "true" => (b, compass::norm(b - var), compass::norm(b - var - dev)),
            "magnetic" => (compass::norm(b + var), b, compass::norm(b - dev)),
            _ => (compass::norm(b + dev + var), compass::norm(b + dev), b),
        };
        let rb = compass::norm(tb - th);
        answer = format!(
            "{answer}; bearing TB {}, MB {}, CB {}, relative {}",
            c3(tb),
            c3(mb),
            c3(cb),
            c3(rb)
        );
        extra.push(("true_bearing", ctx.out("true_bearing", deg_q(tb))));
        extra.push(("magnetic_bearing", ctx.out("magnetic_bearing", deg_q(mb))));
        extra.push(("compass_bearing", ctx.out("compass_bearing", deg_q(cb))));
        extra.push(("relative_bearing", ctx.out("relative_bearing", deg_q(rb))));
    }
    if ctx.explaining() {
        ctx.step(
            "Variation",
            "T − V = M (east positive)",
            format!("{} − {}", c3(th), ew(var)),
            c3(mh),
        );
        ctx.step(
            "Deviation",
            "M − D = C",
            format!("{} − {}", c3(mh), ew(dev)),
            answer.clone(),
        );
    }
    let mut out = vec![
        ("answer", Json::str(answer)),
        ("true_heading", ctx.out("true_heading", deg_q(th))),
        ("magnetic_heading", ctx.out("magnetic_heading", deg_q(mh))),
        ("compass_heading", ctx.out("compass_heading", deg_q(ch))),
        ("deviation", Json::str(ew(dev))),
        ("compass_error", Json::str(ew(ce))),
    ];
    out.extend(extra);
    Ok(Json::obj(out))
}
