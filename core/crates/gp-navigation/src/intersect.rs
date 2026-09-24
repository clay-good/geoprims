//! Course intersections and intercepts (add-navigation-and-geometry,
//! route-geometry "Course intersections and intercepts"). Two geodesic
//! courses meet where Newton's method on the two distances closes the gap,
//! seeded by the spherical answer; two rhumb courses are straight lines in
//! Mercator coordinates and meet exactly. The intercept is the first time
//! the range to a moving target equals the pursuer's distance run, found by
//! steps that cannot pass the first solution.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::wrap_azimuth;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, Stability, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point;
use gp_geo::tm::{tauf, taupf};
use libm::{asinh, atan, atan2, cos, sin, sinh, sqrt, tan};

use crate::{DIST_P, E, KARNEY, R1, deg, meters, setup, unit};

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
    .angle_range("[0,360)")
}

const fn dist(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Distance,
            unit: "m",
        },
    )
    .precision(DIST_P)
}

// ---------------------------------------------------------------- intersection

/// Unit vector of (lat, lon) in degrees.
fn vec3(lat: f64, lon: f64) -> [f64; 3] {
    let (p, l) = (lat.to_radians(), lon.to_radians());
    [cos(p) * cos(l), cos(p) * sin(l), sin(p)]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    sqrt(dot(a, a))
}
/// The unit tangent at `p` heading `az` degrees.
fn tangent(lat: f64, lon: f64, az: f64) -> [f64; 3] {
    let (p, l, a) = (lat.to_radians(), lon.to_radians(), az.to_radians());
    let east = [-sin(l), cos(l), 0.0];
    let north = [-sin(p) * cos(l), -sin(p) * sin(l), cos(p)];
    [
        sin(a) * east[0] + cos(a) * north[0],
        sin(a) * east[1] + cos(a) * north[1],
        sin(a) * east[2] + cos(a) * north[2],
    ]
}

/// Both spherical crossings of two great circles, as signed arc distances (m
/// on the mean sphere) along each course, or None for the same great circle.
fn spherical_pair(a: (f64, f64, f64), b: (f64, f64, f64)) -> Option<[(f64, f64); 2]> {
    let (pa, pb) = (vec3(a.0, a.1), vec3(b.0, b.1));
    let (ta, tb) = (tangent(a.0, a.1, a.2), tangent(b.0, b.1, b.2));
    let x = cross(cross(pa, ta), cross(pb, tb));
    let n = norm(x);
    if n < 1e-12 {
        return None;
    }
    let x = [x[0] / n, x[1] / n, x[2] / n];
    let along = |p: [f64; 3], t: [f64; 3], x: [f64; 3]| atan2(dot(x, t), dot(x, p)) * R1;
    Some([x, [-x[0], -x[1], -x[2]]].map(|x| (along(pa, ta, x), along(pb, tb, x))))
}

/// Of the two antipodal spherical crossings, the one ahead on both courses,
/// then the nearer.
fn spherical_seed(a: (f64, f64, f64), b: (f64, f64, f64)) -> Option<(f64, f64)> {
    let cands = spherical_pair(a, b)?;
    cands.into_iter().min_by(|u, v| {
        let score = |c: &(f64, f64)| {
            let behind = (c.0 < 0.0) as u8 + (c.1 < 0.0) as u8;
            (behind, c.0.abs() + c.1.abs())
        };
        let (su, sv) = (score(u), score(v));
        su.0.cmp(&sv.0).then(su.1.total_cmp(&sv.1))
    })
}

/// Geodesic crossing by Newton's method on (s1, s2): ((lat, lon), s1, s2).
fn geodesic_crossing(
    g: &Geodesic,
    a: (f64, f64, f64),
    b: (f64, f64, f64),
) -> Option<((f64, f64), f64, f64)> {
    newton(g, a, b, spherical_seed(a, b)?)
}

/// A crossing: ((lat, lon), distance along the first course, along the second).
type Crossing = ((f64, f64), f64, f64);

/// Newton's method on (s1, s2) from a seed: ((lat, lon), s1, s2).
fn newton(
    g: &Geodesic,
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    seed: (f64, f64),
) -> Option<((f64, f64), f64, f64)> {
    let (mut s1, mut s2) = seed;
    // The closest approach seen: rounding can keep the gap from settling
    // below the step threshold (it differs by a few ulps between hosts), so a
    // gap under a micrometer at the end still counts.
    let mut best: Option<(f64, Crossing)> = None;
    for _ in 0..50 {
        let (la1, lo1, az1): (f64, f64, f64) = g.direct(a.0, a.1, a.2, s1);
        let (la2, lo2, az2): (f64, f64, f64) = g.direct(b.0, b.1, b.2, s2);
        // Gap from the point on course 2 to the point on course 1, in the
        // tangent plane at the second.
        let (d, beta, _, _): (f64, f64, f64, f64) = g.inverse(la2, lo2, la1, lo1);
        // A nanometer, or rounding at 10,000 km, whichever is larger.
        if d <= 1e-9_f64.max(1e-15 * (s1.abs() + s2.abs())) {
            return Some(((la1, lo1), s1, s2));
        }
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, ((la1, lo1), s1, s2)));
        }
        let r = (d * sin(beta.to_radians()), d * cos(beta.to_radians()));
        let u1 = (sin(az1.to_radians()), cos(az1.to_radians()));
        let u2 = (sin(az2.to_radians()), cos(az2.to_radians()));
        // u1·ds1 − u2·ds2 = −r
        let det = -u1.0 * u2.1 + u1.1 * u2.0;
        if det.abs() < 1e-12 {
            return None;
        }
        let ds1 = (-r.0 * -u2.1 - -r.1 * -u2.0) / det;
        let ds2 = (u1.0 * -r.1 - u1.1 * -r.0) / det;
        // Rounding in the direct and inverse problems can hold the gap a little
        // above a nanometer; once the steps vanish, that is the answer.
        if d <= 1e-6 && ds1.abs() + ds2.abs() <= 1e-8_f64.max(1e-15 * (s1.abs() + s2.abs())) {
            return Some(((la1, lo1), s1, s2));
        }
        s1 += ds1;
        s2 += ds2;
    }
    best.filter(|(d, _)| *d <= 1e-6).map(|(_, found)| found)
}

/// Rhumb crossing in Mercator coordinates (λ, ψ): ((lat, lon), s1, s2) with
/// signed distances, or None for parallel courses.
fn rhumb_crossing(
    a_ell: f64,
    f: f64,
    a: (f64, f64, f64),
    b: (f64, f64, f64),
) -> Option<((f64, f64), f64, f64)> {
    let es = sqrt(f * (2.0 - f));
    let psi = |lat: f64| asinh(taupf(tan(lat.to_radians()), es));
    let lat_of = |p: f64| atan(tauf(sinh(p), es)).to_degrees();
    let (xa, ya, xb, yb) = (a.1.to_radians(), psi(a.0), b.1.to_radians(), psi(b.0));
    let (da, db) = (
        (sin(a.2.to_radians()), cos(a.2.to_radians())),
        (sin(b.2.to_radians()), cos(b.2.to_radians())),
    );
    let det = da.0 * -db.1 + da.1 * db.0;
    if det.abs() < 1e-12 {
        return None;
    }
    let rh = gp_geo::rhumb::Rhumb::new(a_ell, f);
    let mut best: Option<((f64, f64), f64, f64)> = None;
    // The same pair of rhumbs can meet again after winding around the Earth;
    // try the nearby windings and keep the nearest crossing ahead.
    for k in -1..=1 {
        let dx = xb + f64::from(k) * core::f64::consts::TAU - xa;
        let dy = yb - ya;
        // da·u − db·v = (dx, dy)
        let u = (dx * -db.1 - dy * -db.0) / det;
        let (x, y) = (xa + u * da.0, ya + u * da.1);
        if !y.is_finite() || y.abs() > 20.0 {
            continue;
        }
        let (lat, lon) = (lat_of(y), wrap_lon_deg(x.to_degrees()));
        let signed = |p: (f64, f64, f64)| {
            let (s, az) = rh.inverse(p.0, p.1, lat, lon);
            let off = (az - p.2 + 540.0).rem_euclid(360.0) - 180.0;
            if off.abs() > 90.0 { -s } else { s }
        };
        let (s1, s2) = (signed(a), signed(b));
        let better = best.is_none_or(|(_, b1, b2)| {
            let key = |x: f64, y: f64| ((x < 0.0) as u8 + (y < 0.0) as u8, x.abs() + y.abs());
            let (n, o) = (key(s1, s2), key(b1, b2));
            n.0 < o.0 || (n.0 == o.0 && n.1 < o.1)
        });
        if better {
            best = Some(((lat, lon), s1, s2));
        }
    }
    best
}

fn wrap_lon_deg(x: f64) -> f64 {
    (x + 180.0).rem_euclid(360.0) - 180.0
}

pub static COURSE_INTERSECTION: ToolDef = ToolDef {
    id: "navigation.route.course-intersection",
    version: "1.0.1",
    stability: Stability::Stable,
    title: "Where two courses cross",
    summary: "The point where two courses from two positions meet, on geodesics or rhumb lines, and how far each has to run to get there.",
    aliases: &[
        "course intersection",
        "intersection of two bearings",
        "two bearing fix",
        "cross bearing fix",
    ],
    keywords: &[
        "intersection",
        "course",
        "bearing",
        "fix",
        "cross",
        "geodesic",
        "rhumb",
    ],
    inputs: &[
        point::lat_field("lat1", "First position latitude"),
        point::lon_field("lon1", "First position longitude"),
        angle("course1", "First course", "True, like 045 deg")
            .required()
            .core(),
        point::lat_field("lat2", "Second position latitude"),
        point::lon_field("lon2", "Second position longitude"),
        angle("course2", "Second course", "True, like 315 deg")
            .required()
            .core(),
        Field::new(
            "method",
            "Line type",
            "geodesic (the shortest path, the default) or rhumb (constant course)",
            Kind::Choice(&["geodesic", "rhumb"]),
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        dist(
            "distance1",
            "Run from the first position",
            "Negative when the crossing is behind it",
        ),
        dist(
            "distance2",
            "Run from the second position",
            "Negative when the crossing is behind it",
        ),
        point::lat_field("lat", "Crossing latitude").precision(Precision::Decimals(7)),
        point::lon_field("lon", "Crossing longitude").precision(Precision::Decimals(7)),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Geodesic: Newton's method on the two distances, x1(s1) = x2(s2), using each geodesic's direction at its current end (Karney 2013 direct and inverse), started from the great-circle crossing on the mean sphere. Two courses cross twice, about half the world apart; each run is read within half a circumference either way, and of the two crossings the one behind fewer of the positions is reported, then the nearer. Rhumb: both are straight lines in Mercator coordinates (longitude, isometric latitude) and meet exactly",
    accuracy: "The geodesic crossing closes to under a nanometer; the rhumb crossing is exact to floating point",
    when_to_use: "Use this to find where two courses from two known positions meet: a two-bearing fix from two stations, where an aircraft's track crosses a ship's, where a survey line run on one bearing meets another, or where two search legs overlap. It gives the crossing point and how far each course must be run to reach it, negative when the crossing lies behind a position, on geodesics or on rhumb lines.",
    limitations: "Two courses cross twice, about half the world apart, and this reports one of those two: the one behind fewer of the positions, then the nearer, each run read within half a circumference either way. So when the near crossing lies behind one position, it is reported with a negative run rather than replaced by the far one on the other side of the Earth; for a crossing half the world away along a course, ahead and behind name the same place. Courses along the same line, or side by side, have no crossing and are refused. Geodesics and rhumb lines cross in different places, so choose the kind the courses are actually flown on.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Two courses across the Gulf of Maine",
        input: r#"{"lat1":42.36,"lon1":-71.0,"course1":"45 deg","lat2":43.66,"lon2":-70.26,"course2":"135 deg"}"#,
        source: "Worked on the ellipsoid: both runs checked by carrying each course its distance with the geodesic direct problem",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.route.intercept",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.direct",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.intersection",
            reason: "alternative",
        },
    ],
    sentence: "The courses cross {distance1} along the first and {distance2} along the second.",
    limits: &[("batchRows", 10_000)],
    run: run_intersection,
    ..ToolDef::BLANK
};

fn run_intersection(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (e, g) = setup(ctx)?;
    let dg = unit(QT::Angle, "deg");
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let (la2, lo2) = point::read(ctx, "lat2", "lon2")?;
    let c1 = wrap_azimuth(ctx.req_quantity("course1")?.to(dg));
    let c2 = wrap_azimuth(ctx.req_quantity("course2")?.to(dg));
    let (a, b) = ((la1, lo1, c1), (la2, lo2, c2));
    let rhumb = ctx.choice("method")? == Some("rhumb");
    if la1.abs() == 90.0 || la2.abs() == 90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "A course from a pole has no direction; start from a point off the pole.",
        )
        .at("/lat1"));
    }
    let found = if rhumb {
        rhumb_crossing(e.a, e.f, a, b)
    } else {
        geodesic_crossing(&g, a, b)
    };
    let ((lat, lon), s1, s2) = found.ok_or_else(|| {
        ToolError::new(
            ErrorCode::NoSolution,
            "These courses do not cross: they run along the same line or side by side.",
        )
    })?;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Run along the second course",
            "the distance that closes the gap between the two courses",
            format!("course {c2:.1}° from {la2:.4}°, {lo2:.4}°"),
            display::quantity(s2, "m", DIST_P, fmt),
        );
        ctx.step(
            "Run along the first course",
            "the matching distance on the first course",
            format!("course {c1:.1}° from {la1:.4}°, {lo1:.4}°"),
            display::quantity(s1, "m", DIST_P, fmt),
        );
    }
    Ok(Json::obj(vec![
        ("distance1", ctx.out("distance1", meters(s1))),
        ("distance2", ctx.out("distance2", meters(s2))),
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
    ]))
}

// ---------------------------------------------------------------- intercept

pub static INTERCEPT: ToolDef = ToolDef {
    id: "navigation.route.intercept",
    version: "1.0.1",
    stability: Stability::Stable,
    title: "Intercept a moving target",
    summary: "The course to steer, the time, and the meeting point for reaching a target that is moving on a steady course and speed, or why it cannot be reached.",
    aliases: &[
        "intercept course",
        "intercept calculator",
        "pursuit course",
        "rendezvous",
    ],
    keywords: &[
        "intercept",
        "target",
        "course to steer",
        "rendezvous",
        "time to intercept",
        "moving",
    ],
    inputs: &[
        point::lat_field("lat", "Your latitude"),
        point::lon_field("lon", "Your longitude"),
        Field::new(
            "speed",
            "Your speed",
            "Over the ground, like 20 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .required(),
        point::lat_field("target_lat", "Target latitude"),
        point::lon_field("target_lon", "Target longitude"),
        angle("target_course", "Target course", "True, like 090 deg")
            .required()
            .core(),
        Field::new(
            "target_speed",
            "Target speed",
            "Over the ground, like 12 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .required(),
    ],
    outputs: &[
        angle("course", "Course to steer", "Initial geodesic course, true")
            .precision(Precision::Decimals(2)),
        Field::new(
            "time",
            "Time to intercept",
            "From now",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(2)),
        dist("distance", "Your run", "Speed × time"),
        point::lat_field("meet_lat", "Meeting latitude").precision(Precision::Decimals(7)),
        point::lon_field("meet_lon", "Meeting longitude").precision(Precision::Decimals(7)),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "On WGS 84, the target runs its geodesic course: T(t) = direct(T0, course, v_T·t). The intercept is the first t where the geodesic range from you to T(t) equals v·t. Steps of f(t) ÷ (v + v_T), with f = range − v·t, never pass the first root because f changes no faster than v + v_T; a Newton step that brackets the root finishes it by regula falsi. You steer the geodesic to the meeting point",
    accuracy: "Solves to a millimeter of range for steady courses and speeds; real targets turn and change speed",
    when_to_use: "Use this to find the course to steer to meet something that is moving: a boat closing on a vessel on a steady course, an aircraft joining another, a patrol reaching a drifting target, a drone meeting a moving vehicle. Give your position and speed and the target's position, course, and speed, and it returns the course, the time, your run, and the meeting point, or says plainly that the target cannot be reached.",
    limitations: "It assumes both keep a steady speed and the target a steady geodesic course from now on, which real targets rarely do for long; rerun it as the picture changes. Speeds are over the ground, so wind or current has to be taken out first. You fly the geodesic to the meeting point, the shortest path, not a constant heading, and the answer is the first meeting, not the one that needs the least speed. A target that is opening faster than you can close is never met, and the search stops once the target has run half the Earth's girth or you have run 40,000 km, so a meeting beyond that is reported as unreachable.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "20 kt pursuer, target 10 NM north running east at 12 kt",
        input: r#"{"lat":40.0,"lon":-70.0,"speed":"20 kt","target_lat":40.1665,"target_lon":-70.0,"target_course":"90 deg","target_speed":"12 kt"}"#,
        source: "Worked in the plane first (t = 10 ÷ √(20² − 12²) = 0.625 h) and refined on the ellipsoid; checked by running both to the meeting point",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.route.cpa",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.course-intersection",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.direct",
            reason: "parent",
        },
    ],
    sentence: "Steer {course} to meet the target in {time}, after {distance}.",
    limits: &[("batchRows", 10_000)],
    run: run_intercept,
    ..ToolDef::BLANK
};

/// The first t ≥ 0 (hours) with range(t) = v·t, or None within `horizon`.
fn first_meeting(f: impl Fn(f64) -> f64, v_sum: f64, horizon: f64) -> Option<f64> {
    const TOL: f64 = 1e-3; // m
    let mut t = 0.0;
    // Each step costs three geodesic solves; 20,000 keeps a grazing case under
    // about a second in Wasm, and ordinary cases bracket in a few steps.
    for _ in 0..20_000 {
        let ft = f(t);
        if ft <= TOL {
            return Some(t);
        }
        // Try a Newton step; if it brackets the root, finish by regula falsi.
        let h = 1e-6_f64.max(t * 1e-9);
        let d = (f(t + h) - ft) / h;
        if d < 0.0 {
            let tn = t - ft / d;
            if tn <= horizon {
                let fn_ = f(tn);
                if fn_ <= 0.0 {
                    let (mut lo, mut hi, mut flo, mut fhi) = (t, tn, ft, fn_);
                    for _ in 0..200 {
                        let m = lo + (hi - lo) * flo / (flo - fhi);
                        let fm = f(m);
                        if fm.abs() <= TOL || (hi - lo) < 1e-12 {
                            return Some(m);
                        }
                        if fm > 0.0 {
                            (lo, flo) = (m, fm);
                            fhi /= 2.0; // Illinois
                        } else {
                            (hi, fhi) = (m, fm);
                            flo /= 2.0;
                        }
                    }
                    return Some(lo);
                }
            }
        }
        t += ft / v_sum;
        if t > horizon {
            return None;
        }
    }
    None
}

fn run_intercept(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = Geodesic::wgs84();
    let dg = unit(QT::Angle, "deg");
    let (la, lo) = point::read(ctx, "lat", "lon")?;
    let (tla, tlo) = point::read(ctx, "target_lat", "target_lon")?;
    let v = ctx.req_quantity("speed")?.base(); // m/s
    let vt = ctx.req_quantity("target_speed")?.base();
    let tc = wrap_azimuth(ctx.req_quantity("target_course")?.to(dg));
    if v.is_nan() || v <= 0.0 || vt.is_nan() || vt < 0.0 {
        return Err(ToolError::invalid(
            "/speed",
            "Your speed must be above zero and the target's zero or more.",
        ));
    }
    let (v_mh, vt_mh) = (v * 3600.0, vt * 3600.0); // m per hour
    let target = |t: f64| -> (f64, f64) { g.direct(tla, tlo, tc, vt_mh * t) };
    let range = |t: f64| -> f64 {
        let (a, b) = target(t);
        g.inverse(la, lo, a, b)
    };
    let f = |t: f64| range(t) - v_mh * t;
    // Half the Earth's girth of target travel, or 40,000 km of yours.
    let horizon = if vt_mh > 0.0 {
        (2.0e7 / vt_mh).min(4.0e7 / v_mh)
    } else {
        4.0e7 / v_mh
    };
    let t = first_meeting(f, v_mh + vt_mh, horizon).ok_or_else(|| {
        let h = 1e-4;
        let opening = (range(h) - range(0.0)) / h;
        let msg = if opening >= v_mh {
            format!(
                "The target cannot be reached: it is opening at {:.1} kt, faster than your {:.1} kt can close.",
                opening / 1852.0,
                v_mh / 1852.0
            )
        } else {
            "The target cannot be reached on its present course and speed.".to_owned()
        };
        ToolError::new(ErrorCode::NoSolution, msg).at("/speed")
    })?;
    let (mla, mlo) = target(t);
    let (s, course, _, _): (f64, f64, f64, f64) = g.inverse(la, lo, mla, mlo);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Time to meet",
            "the first t with range(t) = your speed × t",
            format!(
                "{:.1} kt against a target at {:.1} kt",
                v_mh / 1852.0,
                vt_mh / 1852.0
            ),
            display::quantity(t * 60.0, "min", Precision::Decimals(2), fmt),
        );
        ctx.step(
            "Course to steer",
            "the geodesic azimuth to the meeting point",
            format!("to {mla:.5}°, {mlo:.5}°"),
            display::quantity(wrap_azimuth(course), "deg", Precision::Decimals(2), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("course", ctx.out("course", deg(wrap_azimuth(course)))),
        (
            "time",
            ctx.out(
                "time",
                Q {
                    value: t,
                    unit: unit(QT::Time, "h"),
                },
            ),
        ),
        ("distance", ctx.out("distance", meters(s))),
        ("meet_lat", ctx.out("meet_lat", deg(mla))),
        ("meet_lon", ctx.out("meet_lon", deg(mlo))),
    ]))
}

// ---------------------------------------------------------------- segment intersection

pub static SEGMENT_INTERSECTION: ToolDef = ToolDef {
    id: "navigation.geodesic.intersection",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Where two geodesic segments cross",
    summary: "The crossing of the geodesics through two segments on the ellipsoid, nearest the segments' middles, and whether it falls within both segments or out on their extensions.",
    aliases: &[
        "geodesic intersection",
        "intersection of two lines on the earth",
        "great circle intersection",
        "do two routes cross",
    ],
    keywords: &[
        "intersection",
        "segment",
        "crossing",
        "great circle",
        "Karney",
    ],
    inputs: &[
        point::lat_field("a_start_lat", "Segment A start latitude"),
        point::lon_field("a_start_lon", "Segment A start longitude"),
        point::lat_field("a_end_lat", "Segment A end latitude"),
        point::lon_field("a_end_lon", "Segment A end longitude"),
        point::lat_field("b_start_lat", "Segment B start latitude"),
        point::lon_field("b_start_lon", "Segment B start longitude"),
        point::lat_field("b_end_lat", "Segment B end latitude"),
        point::lon_field("b_end_lon", "Segment B end longitude"),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        point::lat_field("lat", "Crossing latitude").precision(Precision::Decimals(9)),
        point::lon_field("lon", "Crossing longitude").precision(Precision::Decimals(9)),
        Field::new(
            "within",
            "Within both segments",
            "yes, or no when the crossing is on an extension",
            Kind::Text { max_len: 4 },
        ),
        Field::new(
            "position",
            "Where it falls",
            "Relative to each segment",
            Kind::Text { max_len: 120 },
        ),
        dist(
            "along_a",
            "Along A",
            "From A's start; negative is behind it",
        ),
        dist(
            "along_b",
            "Along B",
            "From B's start; negative is behind it",
        ),
        dist("length_a", "Length of A", "Start to end"),
        dist("length_b", "Length of B", "Start to end"),
    ],
    errors: &[ErrorCode::DegenerateGeometry, ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    when_to_use: "Use this to find where two great-circle routes cross: an airway against a boundary, a flight path against a corridor, a survey line against a parcel edge, or any two courses drawn between pairs of points. It says where they meet, how far along each the meeting lies, and whether it falls inside both segments or out beyond an end.",
    limitations: "Two geodesics on an ellipsoid meet twice, at antipodal points, and this returns the closer crossing, so a pair of very long routes can cross somewhere other than where you meant. A crossing beyond the end of a segment is still reported, with within saying no and the distance running past the segment's length, because knowing where two courses would meet if extended is often the question; it is not a claim that anything crosses. Segments that lie along the same geodesic have no single crossing. The edges are geodesics, so two rhumb lines drawn on a chart do not cross where this says.",
    model: "Each segment's geodesic from its start at the inverse azimuth; the two crossings of the matching great circles seed Newton's method on the distances along both geodesics (to a nanometer); the crossing kept is the one nearest the segments' midpoints, |x − a/2| + |y − b/2|, as in GeographicLib's Intersect class",
    accuracy: "Matches GeographicLib IntersectTool -i on 60 random segment pairs: to a micrometer, or 5e-12 of the distance for crossings thousands of kilometers out at a shallow angle, where both are limited by rounding",
    references: &[KARNEY, KARNEY_INTERSECT],
    examples: &[Example {
        id: "primary",
        title: "JFK to London and Reykjavík to Lisbon",
        input: r#"{"a_start_lat":40.6413,"a_start_lon":-73.7781,"a_end_lat":51.47,"a_end_lon":-0.4543,"b_start_lat":64.1466,"b_start_lon":-21.9426,"b_end_lat":38.7223,"b_end_lon":-9.1393}"#,
        source: "GeographicLib IntersectTool -i (Karney 2023)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.route.course-intersection",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.vertex",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "parent",
        },
    ],
    sentence: "The geodesics cross at {lat}, {lon}: {position}.",
    limits: &[("batchRows", 10_000)],
    run: run_segments,
    ..ToolDef::BLANK
};

const KARNEY_INTERSECT: gp_base::tool::Reference = gp_base::tool::Reference {
    title: "Geodesic intersections",
    issuer: "Karney, C. F. F., Journal of Surveying Engineering",
    year: 2024,
    edition: "Vol. 150, No. 3",
    locator: "04024005 (closest intersection and segment intersections)",
    url: "https://doi.org/10.1061/JSUED2.SUENG-1483",
};

fn run_segments(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (_, g) = setup(ctx)?;
    let a1 = point::read(ctx, "a_start_lat", "a_start_lon")?;
    let a2 = point::read(ctx, "a_end_lat", "a_end_lon")?;
    let b1 = point::read(ctx, "b_start_lat", "b_start_lon")?;
    let b2 = point::read(ctx, "b_end_lat", "b_end_lon")?;
    let (la, az_a, _, _): (f64, f64, f64, f64) = g.inverse(a1.0, a1.1, a2.0, a2.1);
    let (lb, az_b, _, _): (f64, f64, f64, f64) = g.inverse(b1.0, b1.1, b2.0, b2.1);
    if la == 0.0 || lb == 0.0 {
        return Err(ToolError::invalid(
            if la == 0.0 {
                "/a_end_lat"
            } else {
                "/b_end_lat"
            },
            "Each segment needs two different ends.",
        ));
    }
    let (a, b) = ((a1.0, a1.1, az_a), (b1.0, b1.1, az_b));
    let same = || {
        ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The two segments lie on the same geodesic, so they overlap rather than cross.",
        )
        .at("/b_start_lat")
    };
    let seeds = spherical_pair(a, b).ok_or_else(same)?;
    let (mid_a, mid_b) = (la / 2.0, lb / 2.0);
    let best = seeds
        .into_iter()
        .filter_map(|seed| newton(&g, a, b, seed))
        .min_by(|x, y| {
            let key = |c: &((f64, f64), f64, f64)| (c.1 - mid_a).abs() + (c.2 - mid_b).abs();
            key(x).total_cmp(&key(y))
        })
        .ok_or_else(same)?;
    let ((lat, lon), x, y) = best;
    let side = |s: f64, len: f64, name: &str| {
        if s < 0.0 {
            format!("before the start of {name}")
        } else if s > len {
            format!("past the end of {name}")
        } else {
            format!("on {name}")
        }
    };
    let inside = (0.0..=la).contains(&x) && (0.0..=lb).contains(&y);
    let position = if inside {
        "on both segments".to_owned()
    } else {
        format!("{}; {}", side(x, la, "A"), side(y, lb, "B"))
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |v: f64, d: u8| display::number(v, Precision::Decimals(d), fmt);
        ctx.step(
            "Distances to the crossing",
            "Newton's method on the distances along A and B, from the great-circle crossings",
            format!(
                "A {} m long at {}°, B {} m long at {}°",
                n(la, 3),
                n(az_a, 6),
                n(lb, 3),
                n(az_b, 6)
            ),
            format!("{} m along A, {} m along B", n(x, 3), n(y, 3)),
        );
        ctx.step(
            "Crossing latitude",
            "direct problem along A",
            format!("{} m from A's start", n(x, 3)),
            display::quantity(lat, "deg", Precision::Decimals(9), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(wrap_lon_deg(lon)))),
        ("within", Json::str(if inside { "yes" } else { "no" })),
        ("position", Json::str(position)),
        ("along_a", ctx.out("along_a", meters(x))),
        ("along_b", ctx.out("along_b", meters(y))),
        ("length_a", ctx.out("length_a", meters(la))),
        ("length_b", ctx.out("length_b", meters(lb))),
    ]))
}

// ---------------------------------------------------------------- vertex

pub static VERTEX: ToolDef = ToolDef {
    id: "navigation.geodesic.vertex",
    version: "1.0.1",
    title: "Highest point of a geodesic (vertex)",
    summary: "The vertex of a geodesic: the northernmost point it reaches, where it runs due east or west, and how far along the line from the start it lies.",
    aliases: &[
        "geodesic vertex",
        "maximum latitude of a great circle",
        "northernmost point of a route",
        "great circle vertex",
    ],
    keywords: &[
        "vertex",
        "maximum latitude",
        "great circle",
        "Clairaut",
        "route",
    ],
    inputs: &[
        point::lat_field("lat1", "Start latitude"),
        point::lon_field("lon1", "Start longitude"),
        Field::new(
            "lat2",
            "Second point latitude",
            "Or give an azimuth instead; decimal degrees, like 51.47",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon2",
            "Second point longitude",
            "Decimal degrees, east positive, like -0.4543",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-180,180)"),
        angle(
            "azimuth",
            "Azimuth",
            "Instead of a second point, like 51.38",
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        point::lat_field("vertex_lat", "Vertex latitude").precision(Precision::Decimals(9)),
        point::lon_field("vertex_lon", "Vertex longitude").precision(Precision::Decimals(9)),
        Field::new(
            "along",
            "Distance to the vertex",
            "Along the geodesic from the start; negative is behind it",
            Kind::Quantity {
                q: QT::Distance,
                unit: "m",
            },
        )
        .precision(DIST_P),
        Field::new(
            "within",
            "Between the two points",
            "yes when a second point is given and the vertex lies between them",
            Kind::Text { max_len: 4 },
        )
        .optional(),
        angle(
            "equator_azimuth",
            "Azimuth at the equator",
            "α₀, the Clairaut constant as an angle",
        )
        .precision(Precision::Decimals(9)),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: Stability::Stable,
    when_to_use: "Use this to find the highest latitude a great-circle route reaches, and whether it reaches it on the way. A route from New York to London tops out at 53.7 degrees north, well above either end, which is what decides whether it crosses an area of operations, a weather band or an airspace you care about.",
    limitations: "The vertex need not lie between the two points: when it does not, the route never reaches that latitude, and the `within` field says so -- reading the latitude without it is the mistake this tool invites. Vertices repeat around the globe, and the one reported is the one nearest the start, which may be behind it at a negative distance. A geodesic along the equator has no vertex, and a meridian's vertex is the pole, where longitude names nothing.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Clairaut's relation on the auxiliary sphere: sin α₀ = sin α₁ cos β₁ and σ₁ = atan2(sin β₁, cos α₁ cos β₁); the northern vertex is at σ = 90°, reached by the direct problem in arc length from the start",
    accuracy: "Exact to rounding: the azimuth at the vertex is 90° to within 1e-9°",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "The JFK to London geodesic",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543}"#,
        source: "Karney (2013) auxiliary sphere; GeographicLib GeodSolve -a",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.intersection",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.midpoint",
            reason: "alternative",
        },
    ],
    sentence: "The geodesic peaks at {vertex_lat}, {vertex_lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_vertex,
    ..ToolDef::BLANK
};

fn run_vertex(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (e, g) = setup(ctx)?;
    let (lat1, lon1) = point::read(ctx, "lat1", "lon1")?;
    let second = ctx.raw("lat2").is_some_and(|v| !v.is_null());
    let az_in = ctx
        .quantity("azimuth")?
        .map(|q| q.to(unit(QT::Angle, "deg")));
    let (azi1, len) = match (second, az_in) {
        (true, None) => {
            let (la2, lo2) = point::read(ctx, "lat2", "lon2")?;
            let (s12, az, _, _): (f64, f64, f64, f64) = g.inverse(lat1, lon1, la2, lo2);
            if s12 == 0.0 {
                return Err(ToolError::invalid(
                    "/lat2",
                    "The two points coincide, so they define no geodesic.",
                ));
            }
            (az, Some(s12))
        }
        (false, Some(az)) => (az, None),
        _ => {
            return Err(ToolError::invalid(
                "/azimuth",
                "Give a second point or an azimuth, not both.",
            ));
        }
    };
    // Reduced latitude and the auxiliary-sphere arc from the node to the start.
    let beta1 = atan((1.0 - e.f) * tan(lat1.to_radians()));
    let (sa, ca) = (sin(azi1.to_radians()), cos(azi1.to_radians()));
    let alpha0 = atan2(sa * cos(beta1), libm::hypot(ca, sa * sin(beta1)));
    let sigma1 = atan2(sin(beta1), ca * cos(beta1));
    let arc = ((90.0 - sigma1.to_degrees()) + 180.0).rem_euclid(360.0) - 180.0;
    let r = g._gen_direct(
        lat1,
        lon1,
        azi1,
        true,
        arc,
        geographiclib_rs::geodesic_capability::STANDARD,
    );
    let (vlat, vlon, s) = (r.1, r.2, r.4);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |v: f64, d: u8| display::number(v, Precision::Decimals(d), fmt);
        ctx.step(
            "Arc from the equator crossing",
            "σ₁ = atan2(sin β₁, cos α₁ cos β₁)",
            format!("β₁ = {}°, α₁ = {}°", n(beta1.to_degrees(), 9), n(azi1, 9)),
            format!("{}°", n(sigma1.to_degrees(), 9)),
        );
        ctx.step(
            "Vertex latitude",
            "direct problem to σ = 90°",
            format!("{}° of arc from the start", n(arc, 9)),
            display::quantity(vlat, "deg", Precision::Decimals(9), fmt),
        );
    }
    let mut out = vec![
        ("vertex_lat", ctx.out("vertex_lat", deg(vlat))),
        ("vertex_lon", ctx.out("vertex_lon", deg(wrap_lon_deg(vlon)))),
        ("along", ctx.out("along", meters(s))),
    ];
    if let Some(l) = len {
        out.push((
            "within",
            Json::str(if (0.0..=l).contains(&s) { "yes" } else { "no" }),
        ));
    }
    out.push((
        "equator_azimuth",
        ctx.out(
            "equator_azimuth",
            deg(alpha0.to_degrees().rem_euclid(360.0)),
        ),
    ));
    Ok(Json::obj(out))
}
