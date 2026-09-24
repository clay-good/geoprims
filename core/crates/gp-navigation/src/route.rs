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
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
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
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
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
    when_to_use: "Use this when you want to know how far off a course line you are, and where along the line your closest point lies: checking a track against a planned leg, measuring a deviation, or finding where to rejoin. Right of course is positive, so the sign tells you which way to correct.",
    limitations: "It measures distance from the course line on the ellipsoid, not from a corridor's edge or an airway's protected width, and it does not know whether the closest point lies between the ends of the leg or beyond them, so check the along-track distance before acting on it. Heights play no part, and the answer is geometry rather than guidance.",
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
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.closest-point",
            reason: "next",
        },
        Related {
            id: "navigation.route.legs",
            reason: "parent",
        },
    ],
    assumptions: &[Assumption {
        name: "Sphere radius for the spherical method, GRS 80's mean radius R1",
        value: "6371008.771",
        unit: "m",
        source: "grs80",
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

const IFH: Reference = Reference {
    title: "Instrument Flying Handbook, FAA-H-8083-15B",
    issuer: "Federal Aviation Administration",
    year: 2012,
    edition: "FAA-H-8083-15B",
    locator: "Chapter 5: the standard-rate turn, 3° per second, 360° in two minutes",
    url: "https://www.faa.gov/sites/faa.gov/files/regulations_policies/handbooks_manuals/aviation/FAA-H-8083-15B.pdf",
};

pub static FLY_BY: ToolDef = ToolDef {
    id: "navigation.route.fly-by",
    stability: gp_base::tool::Stability::Stable,
    diagram_inline: true,
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
    warnings: &["FLY_OVER_RECOMMENDED", "INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Coordinated level turn at constant speed: R = V²/(g tan φ), lead = R tan(Δψ/2)",
    accuracy: "Exact for a steady coordinated turn in still air; wind changes the ground track",
    when_to_use: "Use this to know how far before a waypoint to start turning, so the aircraft rolls out on the outbound course instead of overshooting and correcting back. That lead distance is what a flight management system computes for a fly-by waypoint, and it is what a pilot hand-flying a route needs to anticipate: the sharper the turn and the faster the aircraft, the further out it begins. Give the bank angle you intend to use, or a turn rate — three degrees a second is the standard rate — and it also reports the radius, how long the turn takes, and how far it flies.",
    limitations: "This is still air. Wind bends the ground track, and a turn into or out of a strong wind starts and ends in different places than this says; nothing here models it. It is a level coordinated turn at constant speed, so it does not describe a climbing turn, a decelerating one, or the roll-in and roll-out, which take a second or two each and make the real lead slightly longer. It is geometry rather than procedure design: it does not know an aircraft's certified bank limits, a category's protected airspace, or what a published procedure requires, and a course change large enough that a fly-by turn would leave the protected area is flagged as better flown as a fly-over.",
    references: &[IFH, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 90° fly-by at 120 kt and 25° of bank",
        input: r#"{"inbound":"360 deg","outbound":"090 deg","speed":"120 kt","bank":"25 deg"}"#,
        source: "exact by hand, and tied to the FAA's published standard-rate turn: at 120 kt and 25° of bank R = V²/(g tan 25°) = 833.3860599902922 m and the lead for a 90° turn is R tan 45° = R; at the standard 3°/s instead, the radius is 1179.0198184247606 m, identical to the V·T/(2π) the two-minute definition gives, and the 90° arc is 1852.0000000000002 m — one nautical mile exactly",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.route.cross-track",
            reason: "next",
        },
        Related {
            id: "navigation.route.legs",
            reason: "parent",
        },
        Related {
            id: "navigation.route.closest-point",
            reason: "alternative",
        },
    ],
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
    stability: gp_base::tool::Stability::Stable,
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
    when_to_use: "Use this for the arithmetic every leg needs: any two of time, speed, and distance give the third, and a departure time with a UTC offset gives the arrival. It is the quickest way to turn a leg's distance and a groundspeed into an estimate, or to work out the speed a required arrival time implies.",
    limitations: "It assumes one constant speed over the whole leg, which a climb, a descent, a wind change, or a hold breaks. The speed it works with is whatever you give it, so a true airspeed entered where a groundspeed belongs produces a time that a wind will not honor. Arrival times round to the minute.",
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
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.legs",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.haversine",
            reason: "next",
        },
    ],
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
    let mut chars = s.chars();
    let sign = match chars.next()? {
        '+' => 1,
        '-' | '\u{2212}' => -1, // also the Unicode minus sign
        _ => return None,
    };
    let body: String = chars.filter(|c| *c != ':').collect();
    // Digits only, so the slices below fall on character boundaries.
    if !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
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
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let (nm, kt) = (dist / 1852.0, speed * 3600.0 / 1852.0);
        // One step for the quantity that was missing, then the time en route.
        match (d, v, t) {
            (Some(_), Some(_), None) => ctx.step(
                "Time",
                "time = distance / speed",
                format!("{} NM / {} kt", n(nm, 3), n(kt, 2)),
                format!("{} h", n(hours, 4)),
            ),
            (Some(_), None, Some(_)) => ctx.step(
                "Speed",
                "speed = distance / time",
                format!("{} NM / {} h", n(nm, 3), n(hours, 4)),
                format!("{} kt", n(kt, 2)),
            ),
            _ => ctx.step(
                "Distance",
                "distance = speed × time",
                format!("{} kt × {} h", n(kt, 2), n(hours, 4)),
                format!("{} NM", n(nm, 3)),
            ),
        }
        ctx.step(
            "Time en route",
            "ETE = the time written as hours and minutes",
            format!("{} h", n(hours, 4)),
            hm(hours),
        );
    }
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
    stability: gp_base::tool::Stability::Stable,
    diagram_inline: true,
    title: "Closest point of approach",
    summary: "When two moving objects come closest, how close, and the bearing and range then, in a local flat frame (for separations under 500 km), with climb rates and height for aircraft in 3D.",
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
        "vertical separation",
    ],
    inputs: &[
        qty_field(
            "a_course",
            "A course",
            "Degrees true, like 090",
            QT::Angle,
            "deg",
        )
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
        qty_field(
            "b_course",
            "B course",
            "Degrees true, like 090",
            QT::Angle,
            "deg",
        )
        .required()
        .angle_range("unbounded"),
        qty_field("b_speed", "B speed", "Like 10 m/s", QT::Speed, "kt").required(),
        qty_field(
            "at_time",
            "Scene time",
            "Positions this long from now, like 60 s; drives the animation",
            QT::Time,
            "s",
        ),
        qty_field(
            "b_up",
            "B above A",
            "Now, like 1000 ft; default 0. Any vertical input makes the approach 3D",
            QT::Length,
            "ft",
        ),
        qty_field(
            "a_vertical_speed",
            "A climb rate",
            "Negative to descend, like -500 ft/min; default 0",
            QT::VerticalSpeed,
            "ft/min",
        ),
        qty_field(
            "b_vertical_speed",
            "B climb rate",
            "Negative to descend, like 500 ft/min; default 0",
            QT::VerticalSpeed,
            "ft/min",
        ),
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
        qty_field(
            "horizontal_separation",
            "Horizontal separation at CPA",
            "When a vertical input is given",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty_field(
            "vertical_separation",
            "Vertical separation at CPA",
            "B above A; when a vertical input is given",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0))
        .optional(),
        qty_field(
            "scene_end",
            "Scene length",
            "Past the CPA, for the animation",
            QT::Time,
            "s",
        )
        .precision(Precision::Decimals(0)),
        qty_field(
            "separation_at",
            "Separation at the scene time",
            "When a scene time is given",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty_field(
            "a_east_at",
            "A east",
            "At the scene time, from A's start",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty_field(
            "a_north_at",
            "A north",
            "At the scene time",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty_field("b_east_at", "B east", "At the scene time", QT::Length, "m")
            .precision(Precision::Decimals(1))
            .optional(),
        qty_field(
            "b_north_at",
            "B north",
            "At the scene time",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["DIVERGING", "INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Constant velocities in a local east-north(-up) frame: t = −(r·v) / |v|², the 3D minimum when heights or climb rates are given",
    accuracy: "Exact in the plane; the flat-plane approximation is good to about 0.1% under 500 km",
    when_to_use: "Use this to see whether two moving things will pass close enough to matter: two aircraft on steady courses, two vessels crossing, a drone against traffic, a chase boat against a target. Give each one's course and speed and where the second stands relative to the first, and it reports when they are nearest, how near, and the bearing and range at that moment. With heights and climb rates it does the same in three dimensions, which is the case that matters in the air, where two tracks that cross on a chart may be a thousand feet apart.",
    limitations: "It assumes both keep their present course and speed, so it is a picture of the next few minutes and not a prediction: one turn and the answer is void. The frame is flat, which is why the separation should be kept under 500 km, where the approximation costs about a tenth of a percent. It is geometry, not separation standards: it does not know about wake turbulence, required separation minima, or the rules of the road, and a closest approach that looks comfortable here may still be a violation. Where the two are already moving apart the closest approach is in the past, which is flagged, and the present separation is the number that matters.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A heading east, B crossing southbound",
        input: r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s"}"#,
        source: "exactly solvable by hand: with r = (1000, 1200) m and a relative velocity of (−10, −10) m/s, t = −(r·v)/|v|² = 22000/200 = 110 s exactly, the relative position is then (−100, +100) m, so the separation is 100√2 = 141.4213562373095 m on a bearing of exactly 315°, and the present separation is √(1000² + 1200²) = 1562.0499351813308 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    timeline: Some(gp_base::tool::Timeline {
        input: "at_time",
        end: "scene_end",
        key: "time",
    }),
    related: &[
        Related {
            id: "navigation.route.cross-track",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.intercept",
            reason: "next",
        },
        Related {
            id: "navigation.route.closest-point",
            reason: "alternative",
        },
    ],
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
    let bz = ctx.quantity("b_up")?.map(|x| x.base());
    let vza = ctx.quantity("a_vertical_speed")?.map(|x| x.base());
    let vzb = ctx.quantity("b_vertical_speed")?.map(|x| x.base());
    let three_d = bz.is_some() || vza.is_some() || vzb.is_some();
    let bz = bz.unwrap_or(0.0);
    let vz = vzb.unwrap_or(0.0) - vza.unwrap_or(0.0);
    if hypot(bx, by) > 500_000.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The flat-plane method applies to separations under 500 km.",
        )
        .at("/b_east"));
    }
    let (va, vb) = (vel(ac, asp), vel(bc, bsp));
    let (vx, vy) = (vb.0 - va.0, vb.1 - va.1);
    let now = hypot(hypot(bx, by), bz);
    let v2 = vx * vx + vy * vy + vz * vz;
    let mut t = if v2 == 0.0 {
        0.0
    } else {
        -(bx * vx + by * vy + bz * vz) / v2
    };
    if t < 0.0 || v2 == 0.0 {
        if v2 != 0.0 {
            ctx.warnings.push(Warning::new("DIVERGING", "The closest approach was in the past; they are moving apart, so the current separation is shown."));
        }
        t = 0.0;
    }
    let (rx, ry, rz) = (bx + vx * t, by + vy * t, bz + vz * t);
    let bearing = (atan2(rx, ry).to_degrees() + 360.0) % 360.0;
    let m = crate::unit(QT::Length, "m");
    let sec = crate::unit(QT::Time, "s");
    let q = |value: f64, unit| gp_base::tool::Q { value, unit };
    // The scene runs past the CPA (at least a minute), so it can be seen passing.
    let end = (t * 1.5).max(60.0);
    let mut out = vec![
        (
            "separation",
            ctx.out("separation", q(hypot(hypot(rx, ry), rz), m)),
        ),
        ("time", ctx.out("time", q(t, sec))),
        ("bearing", ctx.out("bearing", deg(bearing))),
        (
            "current_separation",
            ctx.out("current_separation", q(now, m)),
        ),
        ("scene_end", ctx.out("scene_end", q(end, sec))),
    ];
    if three_d {
        out.push((
            "horizontal_separation",
            ctx.out("horizontal_separation", q(hypot(rx, ry), m)),
        ));
        out.push((
            "vertical_separation",
            ctx.out("vertical_separation", q(rz, m)),
        ));
    }
    if let Some(at) = ctx.quantity("at_time")?.map(|x| x.base()) {
        if !(0.0..=1e7).contains(&at) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The scene time must be between 0 and 10,000,000 s.",
            )
            .at("/at_time"));
        }
        let (ax, ay) = (va.0 * at, va.1 * at);
        let (bxa, bya) = (bx + vb.0 * at, by + vb.1 * at);
        out.push((
            "separation_at",
            ctx.out(
                "separation_at",
                q(hypot(hypot(bxa - ax, bya - ay), bz + vz * at), m),
            ),
        ));
        out.push(("a_east_at", ctx.out("a_east_at", q(ax, m))));
        out.push(("a_north_at", ctx.out("a_north_at", q(ay, m))));
        out.push(("b_east_at", ctx.out("b_east_at", q(bxa, m))));
        out.push(("b_north_at", ctx.out("b_north_at", q(bya, m))));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- multi-leg routes

const WAYPOINT: &[Field] = &[
    Field::new(
        "name",
        "Name",
        "Like KDEN; optional",
        Kind::Text { max_len: 24 },
    ),
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
];

/// A waypoint of the route, as the map layer reads it.
const PATH_ROW: &[Field] = &[
    qty_field("lat", "Latitude", "Degrees", QT::Angle, "deg").precision(Precision::Decimals(7)),
    qty_field("lon", "Longitude", "Degrees", QT::Angle, "deg").precision(Precision::Decimals(7)),
    Field::new("name", "Name", "Waypoint", Kind::Text { max_len: 24 }),
];

const LEG_ROW: &[Field] = &[
    Field::new("from", "From", "Waypoint", Kind::Text { max_len: 24 }),
    Field::new("to", "To", "Waypoint", Kind::Text { max_len: 24 }),
    qty_field("distance", "Distance", "This leg", QT::Distance, "NM")
        .precision(Precision::Decimals(1)),
    qty_field(
        "true_course",
        "True course",
        "Initial, at the leg's start",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(0))
    .angle_range("[0,360)"),
    qty_field(
        "final_course",
        "Final true course",
        "At the leg's end",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(0))
    .angle_range("[0,360)"),
    qty_field(
        "declination",
        "Declination",
        "At the leg's start, east positive",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(1))
    .optional(),
    qty_field(
        "magnetic_course",
        "Magnetic course",
        "True course minus east declination",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(0))
    .angle_range("[0,360)")
    .optional(),
    qty_field(
        "cumulative",
        "Cumulative distance",
        "From the first waypoint",
        QT::Distance,
        "NM",
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "time",
        "Leg time",
        "At the groundspeed",
        Kind::Text { max_len: 20 },
    )
    .optional(),
    Field::new(
        "eta",
        "Arrival time",
        "At the leg's end",
        Kind::Text { max_len: 24 },
    )
    .optional(),
];

pub static LEGS: ToolDef = ToolDef {
    id: "navigation.route.legs",
    stability: gp_base::tool::Stability::Stable,
    title: "Route legs, courses, and totals",
    summary: "Each leg's distance, true and magnetic course, and cumulative distance for a route of waypoints, with leg times and arrival times from a groundspeed and departure time.",
    aliases: &[
        "flight plan legs",
        "nav log",
        "route planner",
        "magnetic course calculator",
        "leg distances",
    ],
    keywords: &[
        "route",
        "legs",
        "waypoints",
        "nav log",
        "true course",
        "magnetic course",
        "ETA",
        "total distance",
    ],
    inputs: &[
        Field::new(
            "waypoints",
            "Waypoints",
            "In order: name (optional), latitude, longitude, like KDEN, 39.8617, -104.6731",
            Kind::List {
                items: WAYPOINT,
                min: 2,
                max: 100,
            },
        )
        .required()
        .core(),
        Field::new(
            "path",
            "Legs flown as",
            "geodesic (shortest, default) or rhumb (constant course)",
            Kind::Choice(&["geodesic", "rhumb"]),
        )
        .core(),
        Field::new(
            "date",
            "Date for magnetic courses",
            "Like 2026-09-18; leave empty for true courses only",
            Kind::Text { max_len: 12 },
        )
        .core(),
        qty_field(
            "groundspeed",
            "Groundspeed",
            "Like 120 kt, for leg times",
            QT::Speed,
            "kt",
        )
        .core(),
        Field::new(
            "departure",
            "Departure time",
            "Local clock time, like 14:30",
            Kind::Text { max_len: 5 },
        ),
        Field::new(
            "utc_offset",
            "UTC offset",
            "Of the departure time, like -06:00 or Z",
            Kind::Text { max_len: 6 },
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        qty_field(
            "total_distance",
            "Total distance",
            "All legs",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "total_time",
            "Total time",
            "At the groundspeed",
            Kind::Text { max_len: 20 },
        )
        .optional(),
        Field::new(
            "arrival",
            "Arrival time",
            "At the last waypoint",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        Field::new(
            "arrival_utc",
            "Arrival time (UTC)",
            "Zulu",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        Field::new(
            "legs_count",
            "Legs",
            "Number of legs",
            Kind::Number {
                min: 1.0,
                max: 99.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "legs",
            "Legs",
            "One row per leg",
            Kind::List {
                items: LEG_ROW,
                min: 1,
                max: 99,
            },
        ),
        Field::new(
            "path",
            "Route",
            "The waypoints in order, for drawing",
            Kind::List {
                items: PATH_ROW,
                min: 2,
                max: 100,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::Unsupported,
    ],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    when_to_use: "Use this to turn a list of waypoints into a flight or voyage plan: the distance and true course of every leg, the running total, and, with a groundspeed and a departure time, how long each leg takes and when you arrive. It is the table a navigation log is filled in from.",
    limitations: "The legs are geodesics, the shortest way between each pair, so the course changes continuously along one and the figure given is the course leaving the waypoint; a rhumb line holds one heading instead and is the other tool. The time assumes the groundspeed given holds for the whole route, which is a planning figure and not a forecast: wind, climb, and descent are not in it. Courses are true, not magnetic, so a compass needs the variation applied.",
    model: "Geodesic legs (Karney 2013) on WGS 84; magnetic declination from WMM2025",
    accuracy: "Distances and courses to nanometers; declination per WMM2025 (about ±0.5° typical)",
    references: &[KARNEY, gp_geo::magnetic::WMM_REPORT],
    examples: &[Example {
        id: "primary",
        title: "Denver to Aspen to Grand Junction to Denver at 120 kt",
        input: r#"{"waypoints":[{"name":"KDEN","lat":39.8617,"lon":-104.6731},{"name":"KASE","lat":39.2232,"lon":-106.8688},{"name":"KGJT","lat":39.1224,"lon":-108.5267},{"name":"KDEN","lat":39.8617,"lon":-104.6731}],"date":"2026-09-18","groundspeed":"120 kt","departure":"09:00","utc_offset":"-06:00"}"#,
        source: "navigation route-geometry scenario: a 4-waypoint route shows true and magnetic courses per leg, with the declination and model",
    }],
    assets: &["wmm2025"],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "path"), ("distance", "total_distance")],
    }],
    related: &[
        Related {
            id: "navigation.route.time-speed-distance",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "parent",
        },
        Related {
            id: "navigation.rhumb.inverse",
            reason: "alternative",
        },
    ],
    sentence: "The route is {total_distance} in {legs_count} legs.",
    limits: &[("batchRows", 1_000)],
    run: run_legs,
    ..ToolDef::BLANK
};

fn run_legs(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use gp_geo::magnetic as mag;
    let rows = ctx.rows("waypoints")?;
    let dunit = crate::unit(QT::Angle, "deg");
    let mut pts: Vec<(String, f64, f64)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("waypoints", i, r, "lat")?
            .expect("required")
            .to(dunit);
        let lon = ctx
            .row_quantity("waypoints", i, r, "lon")?
            .expect("required")
            .to(dunit);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/waypoints/{i}/lat")));
        }
        let name = r
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        pts.push((
            name.map_or_else(|| format!("WP{}", i + 1), str::to_owned),
            lat,
            lon,
        ));
    }
    let rhumb = ctx.choice("path")? == Some("rhumb");
    let (e, g) = setup(ctx)?;
    let rh = gp_geo::rhumb::Rhumb::new(e.a, e.f);
    // Magnetic declination at each leg's start, when a date is given.
    let mag_t = match ctx.text("date")? {
        None => None,
        Some(raw) => {
            let t = mag::parse_date(&raw).map_err(|m| ToolError::invalid("/date", m))?;
            let (lo, hi) = mag::Model::Wmm2025.window();
            if !(lo..=hi).contains(&t) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    format!(
                        "WMM2025 is valid from 2025.0 to 2030.0; {} is outside it.",
                        raw.trim()
                    ),
                )
                .at("/date"));
            }
            ctx.assets.push(gp_base::envelope::AssetRef {
                id: mag::Model::Wmm2025.id().into(),
                version: mag::Model::Wmm2025.version().into(),
            });
            Some(mag::coeffs_at(mag::Model::Wmm2025, t))
        }
    };
    let gs = ctx.quantity("groundspeed")?.map(|q| q.base());
    if gs.is_some_and(|v| v <= 0.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Groundspeed must be more than zero.",
        )
        .at("/groundspeed"));
    }
    let depart = match ctx.text("departure")? {
        None => None,
        Some(d) => {
            let parts: Vec<&str> = d.trim().split(':').collect();
            let hm = match parts.as_slice() {
                [h, m] => h
                    .parse::<i64>()
                    .ok()
                    .zip(m.parse::<i64>().ok())
                    .filter(|(h, m)| *h < 24 && *m < 60),
                _ => None,
            };
            let Some((h, m)) = hm else {
                return Err(ToolError::invalid(
                    "/departure",
                    "Use a 24-hour clock time like 14:30.",
                ));
            };
            Some(h * 60 + m)
        }
    };
    let offset = match ctx.text("utc_offset")? {
        None => None,
        Some(o) => Some(parse_offset(&o).ok_or_else(|| {
            ToolError::invalid("/utc_offset", "Use an offset like -06:00, +05:30, or Z.")
        })?),
    };
    let total_unit = ctx.output_unit("total_distance");
    let (mut cum, mut hours) = (0.0, 0.0);
    let mut legs = Vec::new();
    for w in pts.windows(2) {
        let ((n1, la1, lo1), (n2, la2, lo2)) = (&w[0], &w[1]);
        let (s, tc, fc) = if rhumb {
            let (s, c) = rh.inverse(*la1, *lo1, *la2, *lo2);
            (s, c, c)
        } else {
            let (s, a1, a2, _): (f64, f64, f64, f64) = g.inverse(*la1, *lo1, *la2, *lo2);
            (s, a1, a2)
        };
        cum += s;
        let wrap = |a: f64| a.rem_euclid(360.0);
        let len = |ctx: &mut Ctx, v: f64| ctx.emit("total_distance", meters(v), total_unit);
        let mut row = vec![
            ("from", Json::str(n1)),
            ("to", Json::str(n2)),
            ("distance", len(ctx, s)),
            (
                "true_course",
                Json::obj([("value", Json::Num(wrap(tc))), ("unit", Json::str("deg"))]),
            ),
            (
                "final_course",
                Json::obj([("value", Json::Num(wrap(fc))), ("unit", Json::str("deg"))]),
            ),
        ];
        if let Some(c) = &mag_t {
            let (b, sv) = mag::field(c, *la1, *lo1, 0.0);
            let d = mag::elements(b, sv).d;
            row.push((
                "declination",
                Json::obj([("value", Json::Num(d)), ("unit", Json::str("deg"))]),
            ));
            row.push((
                "magnetic_course",
                Json::obj([
                    ("value", Json::Num(wrap(tc - d))),
                    ("unit", Json::str("deg")),
                ]),
            ));
        }
        row.push(("cumulative", len(ctx, cum)));
        if let Some(v) = gs {
            let h = s / v / 3600.0;
            hours += h;
            row.push(("time", Json::str(hm(h))));
            if let Some(dep) = depart {
                row.push(("eta", Json::str(clock(dep + (hours * 60.0).round() as i64))));
            }
        }
        legs.push(Json::obj(row));
    }
    let n = legs.len();
    let mut out = vec![("total_distance", ctx.out("total_distance", meters(cum)))];
    if gs.is_some() {
        out.push(("total_time", Json::str(hm(hours))));
        if let Some(dep) = depart {
            let arrive = dep + (hours * 60.0).round() as i64;
            out.push(("arrival", Json::str(clock(arrive))));
            if let Some(off) = offset {
                out.push((
                    "arrival_utc",
                    Json::str(format!("{}Z", clock(arrive - off))),
                ));
            }
        }
    }
    out.push(("legs", Json::Arr(legs)));
    out.push(("legs_count", Json::Num(n as f64)));
    out.push((
        "path",
        Json::Arr(
            pts.iter()
                .map(|(name, la, lo)| {
                    Json::obj([
                        (
                            "lat",
                            Q {
                                value: *la,
                                unit: dunit,
                            }
                            .to_json(),
                        ),
                        (
                            "lon",
                            Q {
                                value: *lo,
                                unit: dunit,
                            }
                            .to_json(),
                        ),
                        ("name", Json::str(name.clone())),
                    ])
                })
                .collect(),
        ),
    ));
    ctx.model = Some(format!(
        "{} legs on {}{}",
        if rhumb {
            "Rhumb"
        } else {
            "Geodesic (Karney 2013)"
        },
        e.describe(),
        if mag_t.is_some() {
            "; magnetic declination from WMM2025 at each leg's start"
        } else {
            ""
        }
    ));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- closest point on a route

const ROUTE_POINT: &[Field] = &[
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
];

pub static CLOSEST_POINT: ToolDef = ToolDef {
    id: "navigation.route.closest-point",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Closest point on a route",
    summary: "The point on a multi-leg route closest to a position: which leg, how far along the route, and how far off it (right of course positive).",
    aliases: &[
        "closest point on route",
        "distance from route",
        "which leg am I on",
        "position on track",
    ],
    keywords: &[
        "route",
        "closest point",
        "leg",
        "along route",
        "cross-track",
        "polyline",
    ],
    inputs: &[
        Field::new(
            "route",
            "Route",
            "Waypoints in order: latitude, longitude, like 40, -105",
            Kind::List {
                items: ROUTE_POINT,
                min: 2,
                max: 1_000,
            },
        )
        .required()
        .core(),
        point::lat_field("lat", "Position latitude"),
        point::lon_field("lon", "Position longitude"),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        Field::new(
            "leg",
            "Leg",
            "1 is the first leg",
            Kind::Number { min: 1.0, max: 1e4 },
        )
        .precision(Precision::Decimals(0)),
        qty_field(
            "along_route",
            "Along the route",
            "From the first waypoint to the closest point",
            QT::Distance,
            "km",
        )
        .precision(DIST_P),
        qty_field(
            "cross_track",
            "Off the route",
            "Right of course positive, relative to that leg",
            QT::Distance,
            "km",
        )
        .precision(DIST_P),
        qty_field(
            "closest_lat",
            "Closest point latitude",
            "On the route",
            QT::Angle,
            "deg",
        )
        .precision(AZ_P)
        .angle_range("[-90,90]"),
        qty_field(
            "closest_lon",
            "Closest point longitude",
            "On the route",
            QT::Angle,
            "deg",
        )
        .precision(AZ_P)
        .angle_range("[-180,180)"),
        qty_field(
            "route_length",
            "Route length",
            "All legs",
            QT::Distance,
            "km",
        )
        .precision(DIST_P),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::Unsupported,
    ],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    when_to_use: "Use this when the course is a whole route rather than one leg: to say which leg a position belongs to, how far along the route its closest point lies, and how far off the route it is. It is the tool for placing a reported position, a diversion, or a point of interest against a filed plan, and for measuring how far a flown track wandered from it.",
    limitations: "It measures to the route line itself, not to a corridor or an airway's protected width, and the cross-track sign is right of course on the leg it chose, so the sign flips at a turn. Where two legs are nearly equidistant, as inside a sharp turn, the leg reported is the nearer by geometry and may not be the one being flown. A closest point that falls past the end of a leg is reported as that waypoint, which is correct as a distance but means the answer is a corner rather than a perpendicular. A leg a quarter of the Earth or more away is skipped, keeping its length in the total but never holding the answer. Heights play no part, and the route is geodesic legs, not rhumb lines.",
    model: "Closest point on each geodesic leg by Karney's interception method, on WGS 84",
    accuracy: "Within 1 mm, like the cross-track tool",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A position beside the second leg of a five-leg route",
        input: r#"{"route":[{"lat":40,"lon":-105},{"lat":40,"lon":-104},{"lat":41,"lon":-104},{"lat":41,"lon":-103},{"lat":40,"lon":-103},{"lat":40,"lon":-102}],"lat":40.5,"lon":-103.9}"#,
        source: "GeographicLib 2.7 GeodSolve: the closest point sits where the course to the position is 90.00000000000 from the leg, 8476.774536 m away, with the first leg and the run up to it summing to the along-route distance",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "closest_lat"), ("lon", "closest_lon")],
    }],
    related: &[
        Related {
            id: "navigation.route.cross-track",
            reason: "parent",
        },
        Related {
            id: "navigation.route.legs",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
    ],
    sentence: "The closest point is on leg {leg}, {along_route} along the route and {cross_track} off it.",
    limits: &[("batchRows", 1_000)],
    run: run_closest_point,
    ..ToolDef::BLANK
};

/// The closest approach found so far: distance to the point, leg index,
/// along-route distance, signed cross-track, and the closest point itself.
type Closest = (f64, usize, f64, f64, (f64, f64));

fn run_closest_point(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("route")?;
    let dunit = crate::unit(QT::Angle, "deg");
    let mut pts = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("route", i, r, "lat")?
            .expect("required")
            .to(dunit);
        let lon = ctx
            .row_quantity("route", i, r, "lon")?
            .expect("required")
            .to(dunit);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/route/{i}/lat")));
        }
        pts.push((lat, lon));
    }
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (e, g) = setup(ctx)?;
    let mut best: Option<Closest> = None;
    let mut before = 0.0;
    let mut too_far = false;
    for (k, w) in pts.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let seg: f64 = g.inverse(a.0, a.1, b.0, b.1);
        if seg == 0.0 {
            continue; // a repeated waypoint adds no leg
        }
        // A leg a quarter of the Earth or more away cannot be projected, but it
        // is also nowhere near the answer, so skip it and keep its length.
        // Refusing the whole route for one distant leg turned a position on a
        // route's own first waypoint into an error.
        let Some((f, t, cross)) = foot(&g, a, b, (lat, lon)) else {
            too_far = true;
            before += seg;
            continue;
        };
        // Clamp to the leg: past either end, the end itself is closest.
        let (c, along) = if t < 0.0 {
            (a, 0.0)
        } else if t > 1.0 {
            (b, seg)
        } else {
            (f, g.inverse(a.0, a.1, f.0, f.1))
        };
        let d: f64 = g.inverse(c.0, c.1, lat, lon);
        let signed = if cross < 0.0 { d } else { -d };
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, k + 1, before + along, signed, c));
        }
        before += seg;
    }
    let Some((_, leg, along, xt, c)) = best else {
        if too_far {
            return Err(ToolError::new(ErrorCode::OutOfDomain, "The position is a quarter of the Earth or more from every leg, where the method does not apply.").at("/lat"));
        }
        return Err(ToolError::invalid(
            "/route",
            "The route needs two different waypoints.",
        ));
    };
    ctx.model = Some(format!(
        "Closest point on each geodesic leg by Karney's interception method, on {}",
        e.describe()
    ));
    Ok(Json::obj([
        ("leg", Json::Num(leg as f64)),
        ("along_route", ctx.out("along_route", meters(along))),
        ("cross_track", ctx.out("cross_track", meters(xt))),
        ("closest_lat", ctx.out("closest_lat", deg(c.0))),
        ("closest_lon", ctx.out("closest_lon", deg(wrap_lon(c.1)))),
        ("route_length", ctx.out("route_length", meters(before))),
    ]))
}
