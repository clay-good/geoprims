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
