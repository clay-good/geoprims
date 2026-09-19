//! Route geometry: cross-track and along-track distance from a course line.
//! On the ellipsoid the foot of the perpendicular is found by Karney's
//! interception method (Algorithms for geodesics, section 8): project the
//! segment and the point with an ellipsoidal gnomonic projection centered on
//! the current estimate, take the foot of the perpendicular in the plane,
//! project it back, and repeat; it converges in a few steps.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::wrap_lon;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point;
use libm::{acos, asin, atan2, cos, hypot, sin};

use crate::{AZ_P, DIST_P, E, KARNEY, LAT1, LAT2, LON1, LON2, R1, deg, meters, setup, two_points};

/// Gnomonic forward: (x, y) of a point in the plane centered at (lat0, lon0),
/// or None when the point is 90° or more from the center (M12 ≤ 0).
fn gnomonic(g: &Geodesic, lat0: f64, lon0: f64, lat: f64, lon: f64) -> Option<(f64, f64)> {
    let (_, azi0, _, m12, big_m12, _, _, _): (f64, f64, f64, f64, f64, f64, f64, f64) =
        g.inverse(lat0, lon0, lat, lon);
    if big_m12 <= 0.0 {
        return None;
    }
    let rho = m12 / big_m12;
    let a = azi0.to_radians();
    Some((rho * sin(a), rho * cos(a)))
}

/// Gnomonic reverse by Newton's method on s: d(m12/M12)/ds = 1/M12².
fn gnomonic_reverse(g: &Geodesic, lat0: f64, lon0: f64, x: f64, y: f64) -> Option<(f64, f64)> {
    let azi0 = atan2(x, y).to_degrees();
    let rho = hypot(x, y);
    let a = g.a;
    let mut s = a * libm::atan(rho / a);
    for _ in 0..20 {
        let (lat, lon, _, m12, big_m12, _): (f64, f64, f64, f64, f64, f64) =
            g.direct(lat0, lon0, azi0, s);
        if big_m12 <= 0.0 {
            return None;
        }
        let ds = (rho - m12 / big_m12) * big_m12 * big_m12;
        s += ds;
        if ds.abs() <= 1e-12 * a {
            return Some((lat, lon));
        }
    }
    let (lat, lon): (f64, f64) = g.direct(lat0, lon0, azi0, s);
    Some((lat, lon))
}

/// The foot of the perpendicular from P to the geodesic through A and B, with
/// its parameter t along A→B in the final projection (0 at A, 1 at B) and the
/// cross product sign there (negative when P is right of course).
pub fn foot(
    g: &Geodesic,
    a: (f64, f64),
    b: (f64, f64),
    p: (f64, f64),
) -> Option<((f64, f64), f64, f64)> {
    let mut c = a;
    let (mut t, mut cross) = (0.0, 0.0);
    for _ in 0..30 {
        let pa = gnomonic(g, c.0, c.1, a.0, a.1)?;
        let pb = gnomonic(g, c.0, c.1, b.0, b.1)?;
        let pp = gnomonic(g, c.0, c.1, p.0, p.1)?;
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let len2 = dx * dx + dy * dy;
        t = ((pp.0 - pa.0) * dx + (pp.1 - pa.1) * dy) / len2;
        cross = dx * (pp.1 - pa.1) - dy * (pp.0 - pa.0);
        let (fx, fy) = (pa.0 + t * dx, pa.1 + t * dy);
        let next = gnomonic_reverse(g, c.0, c.1, fx, fy)?;
        // At the center the projection is exact to first order; stop when the
        // foot moves less than a micrometer.
        let moved: f64 = g.inverse(c.0, c.1, next.0, next.1);
        c = next;
        if moved < 1e-6 {
            break;
        }
    }
    Some((c, t, cross))
}

pub static CROSS_TRACK: ToolDef = ToolDef {
    id: "navigation.route.cross-track",
    title: "Cross-track and along-track distance",
    summary: "How far a point is off the course line from A to B (right of course positive), how far along the course its closest point lies, and that point, on the ellipsoid to the millimeter.",
    aliases: &[
        "cross track error",
        "XTE",
        "off course distance",
        "along track distance",
        "distance from a line",
    ],
    keywords: &[
        "cross-track",
        "along-track",
        "XTE",
        "deviation",
        "course line",
        "perpendicular",
        "foot point",
    ],
    inputs: &[
        LAT1,
        LON1,
        // Six coordinates, five shown first: B's are required but not emphasized.
        Field {
            core: false,
            ..LAT2
        },
        Field {
            core: false,
            ..LON2
        },
        point::lat_field("lat", "Point latitude"),
        point::lon_field("lon", "Point longitude"),
        Field::new(
            "method",
            "Method",
            "ellipsoidal (default, exact) or spherical (the classic formula)",
            Kind::Choice(&["ellipsoidal", "spherical"]),
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        Field::new(
            "cross_track",
            "Cross-track distance",
            "Positive right of course, facing from A to B",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "along_track",
            "Along-track distance",
            "From A to the closest point; negative before A",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "foot_lat",
            "Closest point latitude",
            "On the course line",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-90,90]"),
        Field::new(
            "foot_lon",
            "Closest point longitude",
            "On the course line",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-180,180)"),
        Field::new(
            "within",
            "Within the segment",
            "yes or no",
            Kind::Text { max_len: 3 },
        ),
        Field::new(
            "segment",
            "Segment length",
            "A to B",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P),
        Field::new(
            "end_distance",
            "Distance to the nearest end",
            "When the closest point lies outside the segment",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P)
        .optional(),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::Unsupported,
    ],
    warnings: &[
        "FOOT_OUTSIDE_SEGMENT",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Closest point on the geodesic by Karney's interception method on WGS 84",
    accuracy: "Foot point within 1 mm of a brute-force search along the geodesic; the spherical method is the classic formula on the mean radius",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Off course on the JFK to London Heathrow line",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"lat":44,"lon":-60}"#,
        source: "navigation route-geometry scenario: a point south-east of the JFK-LHR geodesic near the start is right of course",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "foot_lat"), ("lon", "foot_lon")],
    }],
    related: &[Related {
        id: "navigation.geodesic.inverse",
        reason: "alternative",
    }],
    sentence: "The point is {cross_track} off course, {along_track} along it.{warn FOOT_OUTSIDE_SEGMENT} Its closest point is past the end of the segment.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_cross_track,
    ..ToolDef::BLANK
};

fn run_cross_track(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1, la2, lo2) = two_points(ctx)?;
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let spherical = ctx.choice("method")? == Some("spherical");
    let (e, g) = setup(ctx)?;
    let seg: f64 = g.inverse(la1, lo1, la2, lo2);
    if seg == 0.0 {
        return Err(ToolError::invalid(
            "/lat2",
            "A and B are the same point, so there is no course line.",
        ));
    }
    let (xt, at, foot_pt, t) = if spherical {
        spherical_cross_track((la1, lo1), (la2, lo2), (lat, lon))
    } else {
        let Some((f, t, cross)) = foot(&g, (la1, lo1), (la2, lo2), (lat, lon)) else {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The point or the segment is a quarter of the way around the Earth or more from the closest point, where the method does not apply. Use a shorter segment.",
            )
            .at("/lat"));
        };
        let xt: f64 = g.inverse(f.0, f.1, lat, lon);
        let af: f64 = g.inverse(la1, lo1, f.0, f.1);
        (
            if cross < 0.0 { xt } else { -xt },
            if t < 0.0 { -af } else { af },
            f,
            t,
        )
    };
    ctx.model = Some(if spherical {
        "Spherical cross-track and along-track formulas on the mean radius R1 = 6,371,008.771 m"
            .to_owned()
    } else {
        format!(
            "Closest point on the geodesic by Karney's interception method on {}",
            e.describe()
        )
    });
    let mut out = vec![
        ("cross_track", ctx.out("cross_track", meters(xt))),
        ("along_track", ctx.out("along_track", meters(at))),
        ("foot_lat", ctx.out("foot_lat", deg(foot_pt.0))),
        ("foot_lon", ctx.out("foot_lon", deg(wrap_lon(foot_pt.1)))),
        (
            "within",
            Json::str(if (0.0..=1.0).contains(&t) {
                "yes"
            } else {
                "no"
            }),
        ),
        ("segment", ctx.out("segment", meters(seg))),
    ];
    if !(0.0..=1.0).contains(&t) {
        let end = if t > 1.0 { (la2, lo2) } else { (la1, lo1) };
        let d: f64 = g.inverse(lat, lon, end.0, end.1);
        ctx.warnings.push(Warning::new(
            "FOOT_OUTSIDE_SEGMENT",
            format!("The closest point on the course line is {} the segment, so the nearest end ({}) may matter more.", if t > 1.0 { "past the end of" } else { "before the start of" }, if t > 1.0 { "B" } else { "A" }),
        ));
        out.push(("end_distance", ctx.out("end_distance", meters(d))));
    }
    Ok(Json::obj(out))
}

/// The classic spherical formulas (right of course positive), on R1.
fn spherical_cross_track(
    a: (f64, f64),
    b: (f64, f64),
    p: (f64, f64),
) -> (f64, f64, (f64, f64), f64) {
    let r = |d: f64| d.to_radians();
    let (p1, l1, p2, l2, p3, l3) = (r(a.0), r(a.1), r(b.0), r(b.1), r(p.0), r(p.1));
    let dist = |pa: f64, la: f64, pb: f64, lb: f64| {
        let h =
            ((pb - pa) / 2.0).sin().powi(2) + cos(pa) * cos(pb) * ((lb - la) / 2.0).sin().powi(2);
        2.0 * asin(h.sqrt().min(1.0))
    };
    let bearing = |pa: f64, la: f64, pb: f64, lb: f64| {
        atan2(
            sin(lb - la) * cos(pb),
            cos(pa) * sin(pb) - sin(pa) * cos(pb) * cos(lb - la),
        )
    };
    let d13 = dist(p1, l1, p3, l3);
    let (t12, t13) = (bearing(p1, l1, p2, l2), bearing(p1, l1, p3, l3));
    let dxt = asin(sin(d13) * sin(t13 - t12));
    let dat = acos((cos(d13) / cos(dxt)).clamp(-1.0, 1.0))
        * if cos(t13 - t12) < 0.0 { -1.0 } else { 1.0 };
    let d12 = dist(p1, l1, p2, l2);
    // The foot: dat along the great circle from A on course t12.
    let (pf, lf) = {
        let pf = asin(sin(p1) * cos(dat) + cos(p1) * sin(dat) * cos(t12));
        let lf = l1 + atan2(sin(t12) * sin(dat) * cos(p1), cos(dat) - sin(p1) * sin(pf));
        (pf.to_degrees(), lf.to_degrees())
    };
    (dxt * R1, dat * R1, (pf, lf), dat / d12)
}

// ---------------------------------------------------------------- fly-by turns

const G0: f64 = 9.806_65;

const fn qty_field(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    unit: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit })
}

pub static FLY_BY: ToolDef = ToolDef {
    id: "navigation.route.fly-by",
    title: "Fly-by turn anticipation",
    summary: "How early to start a fly-by turn at a waypoint: turn radius, lead distance, arc length, and time in the turn, from the inbound and outbound courses, speed, and bank angle or turn rate.",
    aliases: &[
        "turn anticipation",
        "lead distance",
        "fly-by waypoint",
        "turn lead",
        "when to start the turn",
    ],
    keywords: &[
        "fly-by",
        "turn",
        "lead",
        "anticipation",
        "radius",
        "waypoint",
        "bank",
    ],
    inputs: &[
        qty_field(
            "inbound",
            "Inbound course",
            "Degrees, like 360",
            QT::Angle,
            "deg",
        )
        .required()
        .core()
        .angle_range("unbounded"),
        qty_field(
            "outbound",
            "Outbound course",
            "Degrees, like 090",
            QT::Angle,
            "deg",
        )
        .required()
        .core()
        .angle_range("unbounded"),
        qty_field(
            "speed",
            "Speed",
            "Groundspeed or true airspeed, like 120 kt",
            QT::Speed,
            "kt",
        )
        .required()
        .core(),
        qty_field(
            "bank",
            "Bank angle",
            "Like 25°; or give a turn rate",
            QT::Angle,
            "deg",
        )
        .core(),
        qty_field(
            "turn_rate",
            "Turn rate",
            "Like 3 °/s (standard rate, the default)",
            QT::AngularRate,
            "deg/s",
        ),
    ],
    outputs: &[
        qty_field(
            "lead_distance",
            "Lead distance",
            "Start the turn this far before the waypoint",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1)),
        qty_field(
            "radius",
            "Turn radius",
            "R = V² / (g tan φ)",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1)),
        qty_field("turn_angle", "Course change", "0 to 180°", QT::Angle, "deg")
            .precision(Precision::Decimals(1)),
        Field::new(
            "direction",
            "Turn direction",
            "left or right",
            Kind::Text { max_len: 5 },
        ),
        qty_field(
            "arc_length",
            "Arc length",
            "Distance flown in the turn",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1)),
        qty_field(
            "turn_time",
            "Time in the turn",
            "Arc length / speed",
            QT::Time,
            "s",
        )
        .precision(Precision::Decimals(1)),
        qty_field(
            "bank_used",
            "Bank angle",
            "Given, or from the turn rate",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "FLY_OVER_RECOMMENDED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Coordinated level turn at constant speed: R = V²/(g tan φ), lead = R tan(Δψ/2)",
    accuracy: "Exact for a steady coordinated turn in still air; wind changes the ground track",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 90° fly-by at 120 kt and 25° of bank",
        input: r#"{"inbound":"360 deg","outbound":"090 deg","speed":"120 kt","bank":"25 deg"}"#,
        source: "navigation route-geometry scenario: radius 833.4 m (0.450 NM), lead 833.4 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.route.cross-track",
        reason: "next",
    }],
    sentence: "Start the {direction} turn {lead_distance} before the waypoint; the radius is {radius}.{warn FLY_OVER_RECOMMENDED} A course change this large is better flown as a fly-over.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_fly_by,
    ..ToolDef::BLANK
};

fn run_fly_by(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = crate::unit(QT::Angle, "deg");
    let inbound = ctx.req_quantity("inbound")?.to(d);
    let outbound = ctx.req_quantity("outbound")?.to(d);
    let v = ctx.req_quantity("speed")?.base();
    if v <= 0.0 {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "Speed must be more than zero.").at("/speed"),
        );
    }
    let bank = ctx.quantity("bank")?.map(|b| b.to(d));
    let rate = ctx
        .quantity("turn_rate")?
        .map(|r| r.to(crate::unit(QT::AngularRate, "deg/s")));
    let bank = match (bank, rate) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/turn_rate",
                "Give a bank angle or a turn rate, not both.",
            ));
        }
        (Some(b), None) => b,
        (None, r) => {
            let w = r.unwrap_or(3.0).to_radians();
            if w <= 0.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The turn rate must be more than zero.",
                )
                .at("/turn_rate"));
            }
            libm::atan(v * w / G0).to_degrees()
        }
    };
    if !(bank > 0.0 && bank < 89.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The bank angle must be more than 0° and less than 89°.",
        )
        .at("/bank"));
    }
    let radius = v * v / (G0 * libm::tan(bank.to_radians()));
    let dpsi = (outbound - inbound + 180.0).rem_euclid(360.0) - 180.0;
    let turn = dpsi.abs();
    let lead = radius * libm::tan((turn / 2.0).to_radians());
    let arc = radius * turn.to_radians();
    if turn > 120.0 {
        ctx.warnings.push(Warning::new(
            "FLY_OVER_RECOMMENDED",
            format!("A {turn:.0}° course change needs a lead of {:.0} m; above 120° fly over the waypoint instead.", lead),
        ));
    }
    let m = crate::unit(QT::Length, "m");
    let q = |value: f64, unit| gp_base::tool::Q { value, unit };
    Ok(Json::obj([
        ("lead_distance", ctx.out("lead_distance", q(lead, m))),
        ("radius", ctx.out("radius", q(radius, m))),
        ("turn_angle", ctx.out("turn_angle", deg(turn))),
        (
            "direction",
            Json::str(if dpsi >= 0.0 { "right" } else { "left" }),
        ),
        ("arc_length", ctx.out("arc_length", q(arc, m))),
        (
            "turn_time",
            ctx.out("turn_time", q(arc / v, crate::unit(QT::Time, "s"))),
        ),
        ("bank_used", ctx.out("bank_used", deg(bank))),
    ]))
}

// ---------------------------------------------------------------- time, speed, distance

pub static TSD: ToolDef = ToolDef {
    id: "navigation.route.time-speed-distance",
    title: "Time, speed, and distance",
    summary: "Solves for time, speed, or distance from the other two, and the arrival time from a departure time and UTC offset.",
    aliases: &[
        "time speed distance",
        "ete",
        "eta calculator",
        "how long will it take",
        "groundspeed calculator",
    ],
    keywords: &[
        "time",
        "speed",
        "distance",
        "ETE",
        "ETA",
        "groundspeed",
        "flight time",
    ],
    inputs: &[
        qty_field("distance", "Distance", "Like 250 NM", QT::Distance, "NM").core(),
        qty_field("speed", "Speed", "Like 125 kt", QT::Speed, "kt").core(),
        qty_field("time", "Time", "Like 2 h or 90 min", QT::Time, "h").core(),
        Field::new(
            "departure",
            "Departure time",
            "Local clock time, like 14:30",
            Kind::Text { max_len: 5 },
        )
        .core(),
        Field::new(
            "utc_offset",
            "UTC offset",
            "Of the departure time, like -06:00 or Z",
            Kind::Text { max_len: 6 },
        ),
    ],
    outputs: &[
        Field::new(
            "ete",
            "Time en route",
            "Hours and minutes",
            Kind::Text { max_len: 20 },
        ),
        qty_field("time", "Time", "Decimal", QT::Time, "h").precision(Precision::Decimals(3)),
        qty_field(
            "distance",
            "Distance",
            "Given or solved",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(1)),
        qty_field("speed", "Speed", "Given or solved", QT::Speed, "kt")
            .precision(Precision::Decimals(1)),
        Field::new(
            "eta",
            "Arrival time",
            "Local, same offset as departure",
            Kind::Text { max_len: 20 },
        )
        .optional(),
        Field::new(
            "eta_utc",
            "Arrival time (UTC)",
            "Zulu",
            Kind::Text { max_len: 20 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "distance = speed × time",
    accuracy: "Exact arithmetic; arrival times round to the minute",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "250 NM at 125 kt, departing 14:30 at UTC−6",
        input: r#"{"distance":"250 NM","speed":"125 kt","departure":"14:30","utc_offset":"-06:00"}"#,
        source: "navigation route-geometry scenario: 250 NM at 125 kt is 2 h 00 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.geodesic.inverse",
        reason: "alternative",
    }],
    sentence: "At {speed}, {distance} takes {ete}.",
    limits: &[("batchRows", 10_000)],
    run: run_tsd,
    ..ToolDef::BLANK
};

/// "2 h 05 min", or "45 min" under an hour.
fn hm(hours: f64) -> String {
    let total = (hours * 60.0).round() as i64;
    let (h, m) = (total / 60, total % 60);
    if h == 0 {
        format!("{m} min")
    } else {
        format!("{h} h {m:02} min")
    }
}

/// Minutes from a "±hh:mm", "±hhmm", "±hh", or "Z" offset.
fn parse_offset(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("z") || s == "0" {
        return Some(0);
    }
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' | 0xE2 => -1, // also the Unicode minus (U+2212, starts with 0xE2)
        _ => return None,
    };
    let body: String = s
        .trim_start_matches(['+', '-', '\u{2212}'])
        .chars()
        .filter(|c| *c != ':')
        .collect();
    let (h, m) = match body.len() {
        1 | 2 => (body.parse::<i64>().ok()?, 0),
        4 => (
            body[..2].parse::<i64>().ok()?,
            body[2..].parse::<i64>().ok()?,
        ),
        _ => return None,
    };
    (h <= 14 && m < 60).then_some(sign * (h * 60 + m))
}

/// "hh:mm" plus a day note for an arrival `minutes` after midnight.
fn clock(minutes: i64) -> String {
    let day = minutes.div_euclid(1440);
    let m = minutes.rem_euclid(1440);
    let note = match day {
        0 => String::new(),
        1 => " (next day)".to_owned(),
        -1 => " (previous day)".to_owned(),
        d => format!(" ({d:+} days)"),
    };
    format!("{:02}:{:02}{note}", m / 60, m % 60)
}

fn run_tsd(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = ctx.quantity("distance")?.map(|q| q.base());
    let v = ctx.quantity("speed")?.map(|q| q.base());
    let t = ctx.quantity("time")?.map(|q| q.base());
    let (dist, speed, time) = match (d, v, t) {
        (Some(d), Some(v), None) => {
            if v <= 0.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "Speed must be more than zero.",
                )
                .at("/speed"));
            }
            (d, v, d / v)
        }
        (Some(d), None, Some(t)) => {
            if t <= 0.0 {
                return Err(
                    ToolError::new(ErrorCode::OutOfDomain, "Time must be more than zero.")
                        .at("/time"),
                );
            }
            (d, d / t, t)
        }
        (None, Some(v), Some(t)) => (v * t, v, t),
        _ => {
            return Err(ToolError::invalid(
                "/distance",
                "Give exactly two of distance, speed, and time.",
            ));
        }
    };
    if dist < 0.0 || speed < 0.0 || time < 0.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Distance, speed, and time cannot be negative.",
        )
        .at("/distance"));
    }
    let hours = time / 3600.0;
    let mut out = vec![
        ("ete", Json::str(hm(hours))),
        (
            "time",
            ctx.out(
                "time",
                gp_base::tool::Q {
                    value: time,
                    unit: crate::unit(QT::Time, "s"),
                },
            ),
        ),
        ("distance", ctx.out("distance", meters(dist))),
        (
            "speed",
            ctx.out(
                "speed",
                gp_base::tool::Q {
                    value: speed,
                    unit: crate::unit(QT::Speed, "m/s"),
                },
            ),
        ),
    ];
    if let Some(dep) = ctx.text("departure")? {
        let parts: Vec<&str> = dep.trim().split(':').collect();
        let parsed = match parts.as_slice() {
            [h, m] => h
                .parse::<i64>()
                .ok()
                .zip(m.parse::<i64>().ok())
                .filter(|(h, m)| *h < 24 && *m < 60),
            _ => None,
        };
        let Some((h, m)) = parsed else {
            return Err(ToolError::invalid(
                "/departure",
                "Use a 24-hour clock time like 14:30.",
            ));
        };
        let arrive = h * 60 + m + (hours * 60.0).round() as i64;
        out.push(("eta", Json::str(clock(arrive))));
        if let Some(off) = ctx.text("utc_offset")? {
            let Some(off) = parse_offset(&off) else {
                return Err(ToolError::invalid(
                    "/utc_offset",
                    "Use an offset like -06:00, +05:30, or Z.",
                ));
            };
            out.push(("eta_utc", Json::str(format!("{}Z", clock(arrive - off)))));
        }
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- closest point of approach

pub static CPA: ToolDef = ToolDef {
    id: "navigation.route.cpa",
    title: "Closest point of approach",
    summary: "When two moving objects come closest, how close, and the bearing and range then, in a local flat plane (for separations under 500 km).",
    aliases: &[
        "closest point of approach",
        "CPA TCPA",
        "collision course",
        "miss distance",
    ],
    keywords: &[
        "CPA",
        "TCPA",
        "traffic",
        "collision",
        "separation",
        "miss distance",
        "intercept",
    ],
    inputs: &[
        qty_field("a_course", "A course", "Degrees true", QT::Angle, "deg")
            .required()
            .core()
            .angle_range("unbounded"),
        qty_field(
            "a_speed",
            "A speed",
            "Like 10 m/s or 120 kt",
            QT::Speed,
            "kt",
        )
        .required()
        .core(),
        qty_field("b_east", "B east of A", "Now, like 1000 m", QT::Length, "m")
            .required()
            .core(),
        qty_field(
            "b_north",
            "B north of A",
            "Now, like 1200 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qty_field("b_course", "B course", "Degrees true", QT::Angle, "deg")
            .required()
            .angle_range("unbounded"),
        qty_field("b_speed", "B speed", "Like 10 m/s", QT::Speed, "kt").required(),
    ],
    outputs: &[
        qty_field(
            "separation",
            "Separation at CPA",
            "Closest distance",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
        qty_field(
            "time",
            "Time to CPA",
            "From now; zero when already diverging",
            QT::Time,
            "s",
        )
        .precision(Precision::Decimals(1)),
        qty_field(
            "bearing",
            "Bearing of B at CPA",
            "From A, degrees true",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        qty_field(
            "current_separation",
            "Separation now",
            "Straight-line",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "DIVERGING",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Constant velocities in a local east-north plane",
    accuracy: "Exact in the plane; the flat-plane approximation is good to about 0.1% under 500 km",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A heading east, B crossing southbound",
        input: r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s"}"#,
        source: "navigation route-geometry scenario: CPA at t = 110 s with separation 141.42 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.route.cross-track",
        reason: "alternative",
    }],
    sentence: "Closest approach is {separation} in {time}.{warn DIVERGING} They are already moving apart.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_cpa,
    ..ToolDef::BLANK
};

fn run_cpa(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = crate::unit(QT::Angle, "deg");
    let vel = |course: f64, speed: f64| {
        (
            speed * course.to_radians().sin(),
            speed * course.to_radians().cos(),
        )
    };
    let (ac, asp) = (
        ctx.req_quantity("a_course")?.to(d),
        ctx.req_quantity("a_speed")?.base(),
    );
    let (bc, bsp) = (
        ctx.req_quantity("b_course")?.to(d),
        ctx.req_quantity("b_speed")?.base(),
    );
    let (bx, by) = (
        ctx.req_quantity("b_east")?.base(),
        ctx.req_quantity("b_north")?.base(),
    );
    if asp < 0.0 || bsp < 0.0 {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "Speeds cannot be negative.").at("/a_speed"),
        );
    }
    let now = hypot(bx, by);
    if now > 500_000.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The flat-plane method applies to separations under 500 km.",
        )
        .at("/b_east"));
    }
    let (va, vb) = (vel(ac, asp), vel(bc, bsp));
    let (vx, vy) = (vb.0 - va.0, vb.1 - va.1);
    let v2 = vx * vx + vy * vy;
    let mut t = if v2 == 0.0 {
        0.0
    } else {
        -(bx * vx + by * vy) / v2
    };
    if t < 0.0 || v2 == 0.0 {
        if v2 != 0.0 {
            ctx.warnings.push(Warning::new("DIVERGING", "The closest approach was in the past; they are moving apart, so the current separation is shown."));
        }
        t = 0.0;
    }
    let (rx, ry) = (bx + vx * t, by + vy * t);
    let bearing = (atan2(rx, ry).to_degrees() + 360.0) % 360.0;
    let m = crate::unit(QT::Length, "m");
    let q = |value: f64, unit| gp_base::tool::Q { value, unit };
    Ok(Json::obj([
        ("separation", ctx.out("separation", q(hypot(rx, ry), m))),
        ("time", ctx.out("time", q(t, crate::unit(QT::Time, "s")))),
        ("bearing", ctx.out("bearing", deg(bearing))),
        (
            "current_separation",
            ctx.out("current_separation", q(now, m)),
        ),
    ]))
}
