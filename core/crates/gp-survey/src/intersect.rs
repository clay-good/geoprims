//! Intersections and resection (add-survey-suite, survey/cogo-and-traverse,
//! "Intersections and resection"): bearing–bearing, bearing–distance, and
//! distance–distance intersections with every solution labeled, and
//! three-point resection by Tienstra's method with the danger circle checked.

use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use gp_base::units::Unit;
use libm::{atan2, cos, hypot, sin, sqrt, tan};

use crate::{common_unit, direction, len, len_out};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 11 (coordinate geometry: intersections and three-point resection)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const SOLUTION: &[Field] = &[
    Field::new(
        "label",
        "Solution",
        "Which side of the baseline from the first point to the second",
        Kind::Text { max_len: 40 },
    ),
    len_out("northing", "Northing", "Of the intersection"),
    len_out("easting", "Easting", "Of the intersection"),
];

const NORTHING1: Field = len("northing1", "First point northing", "Like 1000.00 ft")
    .required()
    .core();
const EASTING1: Field = len("easting1", "First point easting", "Like 1000.00 ft")
    .required()
    .core();
const DIRECTION1: Field = Field::new(
    "direction1",
    "Direction from the first point",
    "Bearing or azimuth, like N 45°00'00\" E",
    Kind::Text { max_len: 32 },
)
.core();
const DISTANCE1: Field = len(
    "distance1",
    "Distance from the first point",
    "Instead of a direction, like 300.00 ft",
)
.core();
const NORTHING2: Field = len("northing2", "Second point northing", "Like 1000.00 ft")
    .required()
    .core();
const EASTING2: Field = len("easting2", "Second point easting", "Like 1400.00 ft")
    .required()
    .core();
const DIRECTION2: Field = Field::new(
    "direction2",
    "Direction from the second point",
    "Bearing or azimuth, like N 30°00'00\" W",
    Kind::Text { max_len: 32 },
);
const DISTANCE2: Field = len(
    "distance2",
    "Distance from the second point",
    "Instead of a direction, like 250.00 ft",
);

const INTERSECTION_INPUTS: &[Field] = &[
    NORTHING1, EASTING1, DIRECTION1, DISTANCE1, NORTHING2, EASTING2, DIRECTION2, DISTANCE2,
];
const BEARING_BEARING_INPUTS: &[Field] = &[
    NORTHING1,
    EASTING1,
    DIRECTION1.required(),
    NORTHING2,
    EASTING2,
    DIRECTION2.required(),
];
const BEARING_DISTANCE_INPUTS: &[Field] = &[
    NORTHING1,
    EASTING1,
    DIRECTION1.required(),
    NORTHING2,
    EASTING2,
    DISTANCE2.required(),
];
const DISTANCE_DISTANCE_INPUTS: &[Field] = &[
    NORTHING1,
    EASTING1,
    DISTANCE1.required(),
    NORTHING2,
    EASTING2,
    DISTANCE2.required(),
];

pub static INTERSECTION: ToolDef = ToolDef {
    id: "survey.cogo.intersection",
    stability: gp_base::tool::Stability::Stable,
    title: "Intersection (bearings and distances)",
    summary: "Where two lines cross, a line meets a circle, or two circles meet: bearing–bearing, bearing–distance, and distance–distance intersections from two known points, every solution returned and labeled.",
    aliases: &["COGO intersection", "survey intersection calculator"],
    keywords: &[
        "intersection",
        "bearing",
        "distance",
        "COGO",
        "two solutions",
        "arc",
    ],
    inputs: INTERSECTION_INPUTS,
    outputs: &[
        Field::new(
            "solutions",
            "Solutions",
            "Every point that satisfies both",
            Kind::List {
                items: SOLUTION,
                min: 0,
                max: 2,
            },
        ),
        Field::new(
            "count",
            "Number of solutions",
            "0, 1, or 2",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(gp_base::tool::Precision::Decimals(0)),
        Field::new(
            "kind",
            "Intersection",
            "What met: two lines, a line and a circle, or two circles",
            Kind::Text { max_len: 40 },
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED"],
    when_to_use: "Use this to fix a point from two known points without measuring to it directly: two bearings (a line-line intersection), a bearing and a distance, or two distances. It gives every point that fits, labeled, so you can pick the one that matches the field, and refuses lines that are parallel or cross behind a point.",
    limitations: "The geometry is plane, so the coordinates must be grid or local plane coordinates, not latitude and longitude. A distance-distance or bearing-distance problem usually has two answers, and the tool cannot know which one you meant; a shallow crossing angle makes any of them sensitive to small errors in the directions.",
    model: "Plane coordinate geometry: two lines by their directions, a line and a circle, or two circles about the known points (Ghilani & Wolf 2021, ch. 11)",
    accuracy: "Exact for the geometry; a shallow crossing or a near-tangent meeting magnifies small errors in the inputs",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Two distances from a 400 ft baseline",
        input: r#"{"northing1":"1000 ft","easting1":"1000 ft","distance1":"300 ft","northing2":"1000 ft","easting2":"1400 ft","distance2":"250 ft"}"#,
        source: "add-survey-suite two-solution scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
        Related {
            id: "survey.cogo.resection",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.forward",
            reason: "alternative",
        },
    ],
    sentence: "The {kind} meet at {count} {plural count \"point\" \"points\"}.",
    limits: &[("batchRows", 10_000)],
    run: run_intersection,
    ..ToolDef::BLANK
};

macro_rules! intersection_method {
    ($name:ident, $run:ident, $id:literal, $title:literal, $summary:literal, $aliases:expr,
     $keywords:expr, $inputs:expr, $model:literal, $when:literal, $limits:literal,
     $example_title:literal, $example:literal, $source:literal, $sentence:literal,
     $stability:expr, $warnings:expr, $related:expr) => {
        pub static $name: ToolDef = ToolDef {
            id: concat!("survey.cogo.", $id),
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: $inputs,
            outputs: INTERSECTION.outputs,
            errors: INTERSECTION.errors,
            warnings: $warnings,
            model: $model,
            accuracy: INTERSECTION.accuracy,
            when_to_use: $when,
            limitations: $limits,
            references: INTERSECTION.references,
            examples: &[Example {
                id: "primary",
                title: $example_title,
                input: $example,
                source: $source,
            }],
            primary_example: "primary",
            visualization: INTERSECTION.visualization,
            related: $related,
            stability: $stability,
            sentence: $sentence,
            limits: INTERSECTION.limits,
            run: $run,
            ..ToolDef::BLANK
        };
    };
}

intersection_method!(
    BEARING_BEARING,
    run_bearing_bearing,
    "bearing-bearing",
    "Bearing-bearing intersection",
    "Finds the point where 2 forward bearings or azimuths from known points intersect.",
    &[
        "bearing bearing intersection",
        "direction direction intersection"
    ],
    &["intersection", "bearing", "azimuth", "COGO", "line line"],
    BEARING_BEARING_INPUTS,
    "Plane line-line intersection from 2 known points and 2 forward directions (Ghilani & Wolf 2021, ch. 11)",
    "Use this to fix an inaccessible point by observing or specifying a forward bearing from each of 2 known control points. Both directions must point toward the wanted point. The operation returns the single coordinate where those rays meet and refuses parallel rays or a crossing behind either point. Inspect the crossing geometry before accepting the coordinate, especially when the 2 sight lines meet at a narrow angle.",
    "The coordinates must be on one plane grid or local system, not latitude and longitude. A shallow crossing angle makes the result highly sensitive to small direction errors. This geometric intersection has no redundancy, uncertainty estimate, or blunder detection.",
    "Michigan DOT line-line intersection",
    r#"{"northing1":"4080.822 ft","easting1":"5447.330 ft","direction1":"334°48'47\"","northing2":"4377.864 ft","easting2":"5752.796 ft","direction2":"308°39'58\""}"#,
    "Michigan Department of Transportation survey manual, part III, section 3.6.1 and figure 3.26",
    "The 2 bearings meet at {count} point.",
    gp_base::tool::Stability::Stable,
    &["LEGACY_UNIT", "UNIT_ASSUMED"],
    &[
        Related {
            id: "survey.cogo.intersection",
            reason: "parent"
        },
        Related {
            id: "survey.cogo.bearing-distance",
            reason: "alternative"
        },
        Related {
            id: "survey.cogo.distance-distance",
            reason: "alternative"
        },
    ]
);

intersection_method!(
    BEARING_DISTANCE,
    run_bearing_distance,
    "bearing-distance",
    "Bearing-distance intersection",
    "Finds every forward point on a bearing or azimuth that lies at a given distance from a second known point.",
    &["bearing distance intersection", "line circle intersection"],
    &[
        "intersection",
        "bearing",
        "distance",
        "COGO",
        "line circle",
        "two solutions"
    ],
    BEARING_DISTANCE_INPUTS,
    "Plane line-circle intersection from a forward direction at the first point and a radius about the second point (Ghilani & Wolf 2021, ch. 11)",
    "Use this to fix a point that lies on a known forward bearing from one control point and at a measured distance from another. It returns every solution in front of the bearing point, nearest first, so field evidence can identify the intended point.",
    "The coordinates must be on one plane grid or local system. The geometry can have 0, 1, or 2 solutions; this operation cannot choose between 2 valid field positions. A near-tangent crossing is sensitive to small input errors and no redundant observation is available to detect a blunder. Check the selected point against the site sketch.",
    "A north line through a 50 m circle",
    r#"{"northing1":"0 m","easting1":"0 m","direction1":"0","northing2":"40 m","easting2":"30 m","distance2":"50 m"}"#,
    "add-survey-suite bearing-distance scenario",
    "The bearing and distance give {count} {plural count \"point\" \"points\"}.",
    gp_base::tool::Stability::Experimental,
    &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    &[
        Related {
            id: "survey.cogo.intersection",
            reason: "parent"
        },
        Related {
            id: "survey.cogo.bearing-bearing",
            reason: "alternative"
        },
        Related {
            id: "survey.cogo.distance-distance",
            reason: "alternative"
        },
    ]
);

intersection_method!(
    DISTANCE_DISTANCE,
    run_distance_distance,
    "distance-distance",
    "Distance-distance intersection",
    "Finds every point at the entered distances from 2 known points and labels each side of their baseline.",
    &[
        "distance distance intersection",
        "circle circle intersection",
        "two distance fix"
    ],
    &[
        "intersection",
        "distance",
        "COGO",
        "circle circle",
        "two solutions",
        "trilateration"
    ],
    DISTANCE_DISTANCE_INPUTS,
    "Plane circle-circle intersection from 2 known centers and their measured radii (Ghilani & Wolf 2021, ch. 11)",
    "Use this to fix a point from distances measured to 2 known control points. It returns both valid coordinates when they exist and labels them left or right of the directed baseline from the first control point to the second.",
    "The coordinates and distances must share one plane grid or local system. Two distances normally leave a left-right ambiguity that field evidence must resolve. Near-tangent circles are sensitive to small distance errors, and 2 distances alone provide no redundancy or blunder detection. A result also depends on the stated order of the control points because that order defines the left and right labels. Check both distances and the side against the field notes before using the coordinate.",
    "Tangent circles on a 100 ft baseline",
    r#"{"northing1":"0 ft","easting1":"0 ft","distance1":"60 ft","northing2":"0 ft","easting2":"100 ft","distance2":"40 ft"}"#,
    "add-survey-suite distance-distance geometry",
    "The 2 distances give {count} {plural count \"point\" \"points\"}.",
    gp_base::tool::Stability::Experimental,
    &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    &[
        Related {
            id: "survey.cogo.intersection",
            reason: "parent"
        },
        Related {
            id: "survey.cogo.bearing-bearing",
            reason: "alternative"
        },
        Related {
            id: "survey.cogo.bearing-distance",
            reason: "alternative"
        },
    ]
);

type P = (f64, f64); // (northing, easting)

#[derive(Clone, Copy)]
enum IntersectionMethod {
    BearingBearing,
    BearingDistance,
    DistanceDistance,
}

/// Which side of the directed baseline a point lies on, as a label.
fn side(a: P, b: P, p: P) -> &'static str {
    // East is x and north is y: a positive cross product is counterclockwise, the left.
    let cross = (b.1 - a.1) * (p.0 - a.0) - (b.0 - a.0) * (p.1 - a.1);
    if cross > 0.0 {
        "left of the baseline"
    } else if cross < 0.0 {
        "right of the baseline"
    } else {
        "on the baseline"
    }
}

fn run_intersection(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_intersection_with(ctx, None)
}

fn run_bearing_bearing(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_intersection_with(ctx, Some(IntersectionMethod::BearingBearing))
}

fn run_bearing_distance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_intersection_with(ctx, Some(IntersectionMethod::BearingDistance))
}

fn run_distance_distance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_intersection_with(ctx, Some(IntersectionMethod::DistanceDistance))
}

fn run_intersection_with(
    ctx: &mut Ctx,
    fixed_method: Option<IntersectionMethod>,
) -> Result<Json, ToolError> {
    let (n1, e1, n2, e2) = (
        ctx.req_quantity("northing1")?,
        ctx.req_quantity("easting1")?,
        ctx.req_quantity("northing2")?,
        ctx.req_quantity("easting2")?,
    );
    let (d1, d2) = match fixed_method {
        Some(IntersectionMethod::BearingBearing) => (None, None),
        Some(IntersectionMethod::BearingDistance) => (None, ctx.quantity("distance2")?),
        Some(IntersectionMethod::DistanceDistance) => {
            (ctx.quantity("distance1")?, ctx.quantity("distance2")?)
        }
        None => (ctx.quantity("distance1")?, ctx.quantity("distance2")?),
    };
    let mut named = vec![
        ("northing1", n1),
        ("easting1", e1),
        ("northing2", n2),
        ("easting2", e2),
    ];
    named.extend(d1.map(|q| ("distance1", q)));
    named.extend(d2.map(|q| ("distance2", q)));
    let u = common_unit(&named)?;
    let dir = |ctx: &Ctx, name: &str| -> Result<Option<f64>, ToolError> {
        ctx.text(name)?
            .map(|t| {
                direction::parse(&t)
                    .map(f64::to_radians)
                    .map_err(|m| ToolError::invalid(&format!("/{name}"), m))
            })
            .transpose()
    };
    let (t1, t2) = match fixed_method {
        Some(IntersectionMethod::BearingBearing) => {
            (dir(ctx, "direction1")?, dir(ctx, "direction2")?)
        }
        Some(IntersectionMethod::BearingDistance) => (dir(ctx, "direction1")?, None),
        Some(IntersectionMethod::DistanceDistance) => (None, None),
        None => (dir(ctx, "direction1")?, dir(ctx, "direction2")?),
    };
    let a: P = (n1.to(u), e1.to(u));
    let b: P = (n2.to(u), e2.to(u));
    let (r1, r2) = (d1.map(|q| q.to(u)), d2.map(|q| q.to(u)));
    for (r, f) in [(r1, "/distance1"), (r2, "/distance2")] {
        if r.is_some_and(|r| r <= 0.0) {
            return Err(ToolError::invalid(f, "A distance must be positive."));
        }
    }
    let one = |who: &str,
               t: Option<f64>,
               r: Option<f64>|
     -> Result<(Option<f64>, Option<f64>), ToolError> {
        match (t, r) {
            (Some(_), Some(_)) => Err(ToolError::invalid(
                &format!("/distance{who}"),
                "Give each point a direction or a distance, not both.",
            )),
            (None, None) => Err(ToolError::invalid(
                &format!("/direction{who}"),
                "Give each point a direction or a distance.",
            )),
            x => Ok(x),
        }
    };
    let (s1, s2) = (one("1", t1, r1)?, one("2", t2, r2)?);
    let base = hypot(b.0 - a.0, b.1 - a.1);
    let (kind, pts): (&str, Vec<P>) = match (s1, s2) {
        ((Some(t1), None), (Some(t2), None)) => {
            // a + s·(cos t1, sin t1) = b + w·(cos t2, sin t2)
            let det = cos(t1) * (-sin(t2)) - sin(t1) * (-cos(t2));
            if det.abs() < 1e-12 {
                return Err(ToolError::invalid(
                    "/direction2",
                    "The two directions are parallel, so the lines never cross.",
                ));
            }
            let s = ((b.0 - a.0) * (-sin(t2)) - (b.1 - a.1) * (-cos(t2))) / det;
            let w = (cos(t1) * (b.1 - a.1) - sin(t1) * (b.0 - a.0)) / det;
            if s < 0.0 || w < 0.0 {
                return Err(ToolError::invalid(
                    "/direction2",
                    "The lines cross behind one of the points: check the directions point toward each other.",
                ));
            }
            ("two lines", vec![(a.0 + s * cos(t1), a.1 + s * sin(t1))])
        }
        ((Some(t), None), (None, Some(r))) | ((None, Some(r)), (Some(t), None)) => {
            // The line from the point with the direction; the circle about the other.
            let (from, center) = if t1.is_some() { (a, b) } else { (b, a) };
            let (c, s) = (cos(t), sin(t));
            let (dx, dy) = (from.0 - center.0, from.1 - center.1);
            let bq = 2.0 * (c * dx + s * dy);
            let cq = dx * dx + dy * dy - r * r;
            let disc = bq * bq - 4.0 * cq;
            if disc < -1e-12 * (1.0 + r * r) {
                return Err(ToolError::invalid(
                    if t1.is_some() {
                        "/distance2"
                    } else {
                        "/distance1"
                    },
                    "The line passes the circle without meeting it.",
                ));
            }
            let root = sqrt(disc.max(0.0));
            let mut v: Vec<P> = [(-bq - root) / 2.0, (-bq + root) / 2.0]
                .into_iter()
                .filter(|k| *k >= -1e-9)
                .map(|k| (from.0 + k * c, from.1 + k * s))
                .collect();
            v.dedup_by(|p, q| hypot(p.0 - q.0, p.1 - q.1) < 1e-9 * (1.0 + r));
            if v.is_empty() {
                return Err(ToolError::invalid(
                    if t1.is_some() {
                        "/direction1"
                    } else {
                        "/direction2"
                    },
                    "The circle lies behind the direction given.",
                ));
            }
            ("line and circle", v)
        }
        ((None, Some(r1)), (None, Some(r2))) => {
            if base == 0.0 {
                return Err(ToolError::invalid(
                    "/northing2",
                    "The two points are the same point.",
                ));
            }
            if r1 + r2 < base * (1.0 - 1e-12) || (r1 - r2).abs() > base * (1.0 + 1e-12) {
                return Err(ToolError::invalid(
                    "/distance2",
                    "The two circles do not meet: the distances are too short, or one circle lies inside the other.",
                ));
            }
            // Along the baseline to the chord, then out either side.
            let x = (r1 * r1 - r2 * r2 + base * base) / (2.0 * base);
            let h = sqrt((r1 * r1 - x * x).max(0.0));
            let (ux, uy) = ((b.0 - a.0) / base, (b.1 - a.1) / base);
            let foot = (a.0 + x * ux, a.1 + x * uy);
            // Left of the baseline first, then right.
            let mut v = vec![
                (foot.0 + h * uy, foot.1 - h * ux),
                (foot.0 - h * uy, foot.1 + h * ux),
            ];
            if h < 1e-9 * (1.0 + base) {
                v.truncate(1);
            }
            ("two circles", v)
        }
        _ => unreachable!("each point has exactly one of direction and distance"),
    };
    let q = |x: f64, u: &'static Unit| Q { value: x, unit: u };
    let sols: Vec<Json> = pts
        .iter()
        .map(|&p| {
            Json::obj(vec![
                (
                    "label",
                    Json::Str(if pts.len() == 1 {
                        "the intersection".into()
                    } else {
                        side(a, b, p).to_owned()
                    }),
                ),
                ("northing", ctx.emit("northing", q(p.0, u), u)),
                ("easting", ctx.emit("easting", q(p.1, u), u)),
            ])
        })
        .collect();
    Ok(Json::obj(vec![
        ("solutions", Json::Arr(sols)),
        ("count", Json::Num(pts.len() as f64)),
        ("kind", Json::Str(kind.into())),
    ]))
}

const CONTROL: &[Field] = &[
    len("northing", "Northing", "Like 1000.00 ft").required(),
    len("easting", "Easting", "Like 1000.00 ft").required(),
];

pub static RESECTION: ToolDef = ToolDef {
    id: "survey.cogo.resection",
    title: "Three-point resection",
    summary: "The coordinates of an occupied station from the angles it turned between three known points, by Tienstra's method, with a warning when the station is near the danger circle.",
    aliases: &[
        "three point resection",
        "Tienstra",
        "free station",
        "resection",
    ],
    keywords: &[
        "resection",
        "Tienstra",
        "three point",
        "danger circle",
        "free station",
        "angles",
    ],
    inputs: &[
        Field::new(
            "control",
            "Control points A, B, C",
            "Three known points in the order you turned to them, one per line: northing, easting, like 1000, 1000",
            Kind::List {
                items: CONTROL,
                min: 3,
                max: 3,
            },
        )
        .required()
        .core(),
        Field::new(
            "angle_ab",
            "Angle A to B",
            "Turned clockwise at the station from A to B, like 37°51'16\"",
            Kind::Quantity {
                q: gp_base::units::Quantity::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded")
        .required()
        .core(),
        Field::new(
            "angle_bc",
            "Angle B to C",
            "Turned clockwise at the station from B to C, like 34°00'27\"",
            Kind::Quantity {
                q: gp_base::units::Quantity::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded")
        .required()
        .core(),
    ],
    outputs: &[
        len_out("northing", "Northing", "Of the occupied station"),
        len_out("easting", "Easting", "Of the occupied station"),
        Field::new(
            "danger_ratio",
            "Distance from the danger circle",
            "|distance to the circle's center − its radius| ÷ radius; near 0 is unstable",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(gp_base::tool::Precision::Decimals(4)),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &[
        "RESECTION_UNSTABLE",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Tienstra: with the triangle's angles A′, B′, C′ and the station's opposite angles α = ∠BPC, β = ∠CPA, γ = ∠APB, P = (K₁A + K₂B + K₃C)/(K₁ + K₂ + K₃), Kᵢ = 1/(cot of the triangle angle − cot of the station angle) (Ghilani & Wolf 2021, ch. 11)",
    accuracy: "Exact for the geometry; unstable near the circle through the three points, where every station on it sees the same angles",
    when_to_use: "Use this when you set up a total station on an unknown point and can see three known control points. Turn the clockwise angle from A to B and from B to C, enter the three points in that order, and it finds the station's northing and easting by Tienstra's method. That position then lets you orient the instrument and start staking or a traverse.",
    limitations: "Only two angles and three points go in, so there is no extra measurement to catch a mistake, and no error estimate comes out. If the station sits near the circle through the three points, the answer becomes unstable; the tool warns when it is close. Coordinates must be on a plane grid. Use a least-squares network when you have more shots.",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A station south of three control points",
        input: r#"{"control":[{"northing":"1000 ft","easting":"1000 ft"},{"northing":"1500 ft","easting":"1400 ft"},{"northing":"1100 ft","easting":"1800 ft"}],"angle_ab":"37.85442542467041 deg","angle_bc":"34.00749241973227 deg"}"#,
        source: "Angles computed from a station at (500, 1350)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.intersection",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
    ],
    sentence: "The station is at northing {northing}, easting {easting}.{warn RESECTION_UNSTABLE} It is close to the danger circle, so the position is weak.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_resection,
    ..ToolDef::BLANK
};

fn run_resection(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("control")?;
    let mut qs = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for f in ["northing", "easting"] {
            let q = ctx.row_quantity("control", i, row, f)?.expect("required");
            qs.push((f, q));
        }
    }
    let u = common_unit(&qs)?;
    let v: Vec<f64> = qs.iter().map(|(_, q)| q.to(u)).collect();
    let (pa, pb, pc): (P, P, P) = ((v[0], v[1]), (v[2], v[3]), (v[4], v[5]));
    let ab = gp_geo::point::plain_angle(ctx, "angle_ab")?.expect("required");
    let bc = gp_geo::point::plain_angle(ctx, "angle_bc")?.expect("required");
    if !(0.0 < ab && ab < 360.0 && 0.0 < bc && bc < 360.0) {
        return Err(ToolError::invalid(
            "/angle_bc",
            "Each angle is between 0° and 360°, turned clockwise.",
        ));
    }
    // The station's angles opposite each known point, clockwise. The third
    // closes the circle; past 360° it wraps, and Tienstra's cotangents do not mind.
    let (alpha, beta, gamma) = (bc, (720.0 - ab - bc) % 360.0, ab);
    // The triangle's interior angles at A, B, and C.
    let ang = |p: P, q: P, r: P| -> f64 {
        let (a1, a2) = (atan2(q.1 - p.1, q.0 - p.0), atan2(r.1 - p.1, r.0 - p.0));
        let mut d = (a2 - a1).abs();
        if d > core::f64::consts::PI {
            d = 2.0 * core::f64::consts::PI - d;
        }
        d
    };
    let (ta, tb, tc) = (ang(pa, pb, pc), ang(pb, pc, pa), ang(pc, pa, pb));
    if ta.min(tb).min(tc) < 1e-9 {
        return Err(ToolError::invalid(
            "/northing_c",
            "The three known points lie on a line, so they fix no circle and no resection.",
        ));
    }
    let cot = |x: f64| 1.0 / tan(x);
    let k = |t: f64, s: f64| 1.0 / (cot(t) - cot(s.to_radians()));
    let (k1, k2, k3) = (k(ta, alpha), k(tb, beta), k(tc, gamma));
    let ks = k1 + k2 + k3;
    // The circle through A, B, and C: every station on it sees the same angles.
    let (ax, ay, bx, by, cx, cy) = (pa.0, pa.1, pb.0, pb.1, pc.0, pc.1);
    let dd = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    let ox = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / dd;
    let oy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / dd;
    let radius = hypot(ax - ox, ay - oy);
    if !ks.is_finite() || ks.abs() < 1e-12 * (k1.abs() + k2.abs() + k3.abs()).max(1.0) {
        return Err(ToolError::invalid(
            "/angle_ab",
            "These angles put the station on the danger circle through the three points, where the resection has no single answer.",
        ));
    }
    let p: P = (
        (k1 * pa.0 + k2 * pb.0 + k3 * pc.0) / ks,
        (k1 * pa.1 + k2 * pb.1 + k3 * pc.1) / ks,
    );
    let ratio = (hypot(p.0 - ox, p.1 - oy) - radius).abs() / radius;
    if ratio < 0.05 {
        ctx.warnings.push(Warning::new(
            "RESECTION_UNSTABLE",
            "The station is within 5% of the danger circle through the three known points: small errors in the angles move it a long way. Choose a point off the circle, or add a fourth.",
        ));
    }
    let q = |x: f64| Q { value: x, unit: u };
    Ok(Json::obj(vec![
        ("northing", ctx.emit("northing", q(p.0), u)),
        ("easting", ctx.emit("easting", q(p.1), u)),
        ("danger_ratio", Json::Num(ratio)),
    ]))
}
