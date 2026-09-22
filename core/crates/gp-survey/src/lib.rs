//! Survey: COGO, traverse closure and adjustment, area by coordinates,
//! horizontal and vertical curves, earthwork volumes, and the combined scale
//! factor (add-survey-suite). Results keep the linear unit the surveyor entered.

pub mod borrow;
pub mod closure;
pub mod direction;
pub mod grade;
pub mod intersect;
pub mod land;
pub mod layout;
pub mod leveling;
pub mod profile;
pub mod reduction;
pub mod section;
pub mod sight;
pub mod spiral;
pub mod staking;
pub mod stationing;
pub mod stockpile;
pub mod vcurve;

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, Slot, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use libm::{acos, asin, atan, atan2, cos, hypot, sin, tan};
use serde_json::{Map, Value};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapters 10 (traverse computations), 12 (area), 24 (horizontal curves), 25 (vertical curves), 26 (volumes)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003237",
};
const AASHTO_GREEN_BOOK: Reference = Reference {
    title: "A Policy on Geometric Design of Highways and Streets (the Green Book)",
    issuer: "American Association of State Highway and Transportation Officials",
    year: 2018,
    edition: "7th edition",
    locator: "Table 3-34 (design controls for crest vertical curves) and Table 3-36 (sag), by design speed",
    url: "https://store.transportation.org/Item/CollectionDetail/180",
};
const STEM: Reference = Reference {
    title: "State Plane Coordinate System of 1983, NOAA Manual NOS NGS 5",
    issuer: "Stem, J. E., National Geodetic Survey",
    year: 1990,
    edition: "Reprint with corrections",
    locator: "Section 4 (grid scale factor, elevation factor, combined factor; R = 20,906,000 ft)",
    url: "https://geodesy.noaa.gov/library/pdfs/NOAA_Manual_NOS_NGS_0005.pdf",
};

pub(crate) fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

pub(crate) const fn len(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
}

pub(crate) const fn len_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
    .precision(Precision::Decimals(3))
}

/// The common linear unit of several values. US survey feet and international
/// feet may not be mixed (the 2 ppm trap); other units convert to the first.
pub(crate) fn common_unit(values: &[(&str, Q)]) -> Result<&'static Unit, ToolError> {
    let first = values
        .first()
        .map(|(_, q)| q.unit)
        .unwrap_or_else(|| unit(QT::Length, "ft"));
    let has = |s: &str| values.iter().any(|(_, q)| q.unit.symbol == s);
    if has("ft") && has("ftUS") {
        let field = values
            .iter()
            .find(|(_, q)| q.unit.symbol != first.symbol)
            .map_or("", |(f, _)| *f);
        return Err(ToolError::new(
            ErrorCode::UnitMismatch,
            "This mixes US survey feet and international feet, which differ by 2 parts per million (about 1 ft in 94 miles). Convert one explicitly.",
        )
        .at(field)
        .hint("Use units.length.ftus-to-ft to convert, then enter every value in one kind of foot."));
    }
    Ok(first)
}

// ---------------------------------------------------------------- inverse / forward

pub static INVERSE: ToolDef = ToolDef {
    id: "survey.cogo.inverse",
    title: "Inverse between two coordinates",
    summary: "The bearing, azimuth, and horizontal distance between two plane-survey coordinates (northing, easting).",
    aliases: &["COGO inverse", "bearing and distance between points"],
    keywords: &[
        "inverse", "bearing", "distance", "COGO", "northing", "easting",
    ],
    inputs: &[
        len("northing1", "From northing", "Like 1000.000 ft")
            .required()
            .core(),
        len("easting1", "From easting", "Like 1000.000 ft")
            .required()
            .core(),
        len("northing2", "To northing", "Like 1100.000 ft")
            .required()
            .core(),
        len("easting2", "To easting", "Like 1100.000 ft")
            .required()
            .core(),
    ],
    outputs: &[
        Field::new(
            "bearing",
            "Bearing",
            "Quadrant bearing",
            Kind::Text { max_len: 24 },
        ),
        len_out(
            "distance",
            "Distance",
            "Horizontal distance, in the input unit",
        ),
        Field::new(
            "azimuth",
            "Azimuth",
            "North azimuth",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .angle_range("[0,360)"),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &[
        "LEGACY_UNIT",
        "AZIMUTH_UNDEFINED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Plane coordinate geometry: azimuth = atan2(ΔE, ΔN), distance = √(ΔN² + ΔE²)",
    accuracy: "Exact for the plane (grid or assumed) coordinates given",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "From (1,000, 1,000) to (1,100, 1,100)",
        input: r#"{"northing1":1000,"easting1":1000,"northing2":1100,"easting2":1100}"#,
        source: "add-survey-suite scenario: N 45°00'00\" E, 141.421",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("distance", "distance")],
    }],
    related: &[Related {
        id: "survey.cogo.forward",
        reason: "inverse",
    }],
    sentence: "The bearing is {bearing} and the distance is {distance}.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let names = ["northing1", "easting1", "northing2", "easting2"];
    let mut qs = Vec::new();
    for n in names {
        qs.push((n, ctx.req_quantity(n)?));
    }
    let u = common_unit(&qs.iter().map(|(n, q)| (*n, *q)).collect::<Vec<_>>())?;
    let v: Vec<f64> = qs.iter().map(|(_, q)| q.to(u)).collect();
    let (dn, de) = (v[2] - v[0], v[3] - v[1]);
    let d = hypot(dn, de);
    if d == 0.0 {
        ctx.warnings.push(Warning::new(
            "AZIMUTH_UNDEFINED",
            "The points coincide, so there is no direction between them.",
        ));
    }
    let az = gp_base::angle::wrap_azimuth(atan2(de, dn).to_degrees());
    Ok(Json::obj([
        ("bearing", Json::str(direction::bearing(az, 0))),
        ("distance", ctx.emit("distance", Q { value: d, unit: u }, u)),
        ("azimuth", ctx.out("azimuth", deg(az))),
    ]))
}

pub static FORWARD: ToolDef = ToolDef {
    id: "survey.cogo.forward",
    title: "Coordinates from a bearing and distance",
    summary: "The coordinates of a point from a known point, a bearing or azimuth, and a horizontal distance (a radial sideshot).",
    aliases: &["COGO forward", "traverse point", "radial sideshot"],
    keywords: &["forward", "sideshot", "bearing", "distance", "COGO"],
    inputs: &[
        len("northing", "From northing", "Like 1000.000 ft")
            .required()
            .core(),
        len("easting", "From easting", "Like 1000.000 ft")
            .required()
            .core(),
        Field::new(
            "direction",
            "Bearing or azimuth",
            "Like N 45°00'00\" E, S44-30-00W, or 224.5",
            Kind::Text { max_len: 32 },
        )
        .required()
        .core(),
        len("distance", "Horizontal distance", "Like 141.421 ft")
            .required()
            .core(),
    ],
    outputs: &[
        len_out("northing", "Northing", "Of the new point"),
        len_out("easting", "Easting", "Of the new point"),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "N2 = N1 + d·cos(az), E2 = E1 + d·sin(az)",
    accuracy: "Exact for plane coordinates",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "N 45° E for 141.421 ft from (1,000, 1,000)",
        input: r#"{"northing":1000,"easting":1000,"direction":"N 45°00'00\" E","distance":141.421356237}"#,
        source: "Inverse of the add-survey-suite inverse scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("northing", "northing"), ("easting", "easting")],
    }],
    related: &[Related {
        id: "survey.cogo.inverse",
        reason: "inverse",
    }],
    sentence: "The new point is at northing {northing}, easting {easting}.",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let n = ctx.req_quantity("northing")?;
    let e = ctx.req_quantity("easting")?;
    let d = ctx.req_quantity("distance")?;
    let u = common_unit(&[("/northing", n), ("/easting", e), ("/distance", d)])?;
    let dir = ctx.text("direction")?.expect("required");
    let az = direction::parse(&dir)
        .map_err(|m| ToolError::invalid("/direction", m))?
        .to_radians();
    let (n2, e2) = (n.to(u) + d.to(u) * cos(az), e.to(u) + d.to(u) * sin(az));
    Ok(Json::obj([
        (
            "northing",
            ctx.emit("northing", Q { value: n2, unit: u }, u),
        ),
        ("easting", ctx.emit("easting", Q { value: e2, unit: u }, u)),
    ]))
}

// ---------------------------------------------------------------- traverse

const COURSE: &[Field] = &[
    Field::new(
        "direction",
        "Bearing or azimuth",
        "Like N 45°30'15\" E or 90",
        Kind::Text { max_len: 32 },
    )
    .required(),
    len(
        "distance",
        "Distance",
        "Horizontal distance, like 300.00 ft",
    )
    .required(),
];
const ADJUSTED: &[Field] = &[
    Field::new(
        "point",
        "Point",
        "1 is the start",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    len_out("northing", "Northing", "Adjusted"),
    len_out("easting", "Easting", "Adjusted"),
    Field::new(
        "bearing",
        "Adjusted bearing",
        "Of the course ending here",
        Kind::Text { max_len: 24 },
    ),
    len_out("distance", "Adjusted distance", "Of the course ending here"),
];

pub static TRAVERSE: ToolDef = ToolDef {
    id: "survey.cogo.traverse-closure",
    stability: gp_base::tool::Stability::Stable,
    title: "Traverse closure and adjustment",
    summary: "Latitudes, departures, linear misclosure, and precision ratio for a closed loop traverse, adjusted by the compass (Bowditch) or transit rule.",
    aliases: &[
        "traverse closure calculator",
        "Bowditch adjustment",
        "compass rule",
    ],
    keywords: &[
        "traverse",
        "closure",
        "misclosure",
        "precision",
        "Bowditch",
        "compass rule",
        "transit rule",
        "latitudes",
        "departures",
    ],
    inputs: &[
        Field::new(
            "courses",
            "Courses",
            "Direction and distance for each leg, in order around the loop, like N 0°00'00\" E and 300 ft",
            Kind::List {
                items: COURSE,
                min: 2,
                max: 2000,
            },
        )
        .required()
        .core(),
        Field::new(
            "adjustment",
            "Adjustment",
            "none, compass (Bowditch, default), or transit",
            Kind::Choice(&["none", "compass", "transit"]),
        )
        .core(),
        len("start_northing", "Start northing", "Default 5000"),
        len("start_easting", "Start easting", "Default 5000"),
    ],
    outputs: &[
        Field::new(
            "precision",
            "Precision",
            "1 : total length / misclosure",
            Kind::Text { max_len: 24 },
        ),
        len_out("misclosure", "Linear misclosure", "√(ΣLat² + ΣDep²)"),
        Field::new(
            "misclosure_bearing",
            "Misclosure direction",
            "Bearing of the error of closure",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "sum_latitudes",
            "Σ latitudes",
            "Sum of north-south components",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "sum_departures",
            "Σ departures",
            "Sum of east-west components",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(4)),
        len_out("total_length", "Total length", "Perimeter of the traverse"),
        Field::new(
            "precision_ratio",
            "Precision ratio",
            "Total length / misclosure (absent when the closure is perfect)",
            Kind::Number {
                min: 0.0,
                max: 1e18,
            },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new("note", "Note", "Method notes", Kind::Text { max_len: 200 }).optional(),
        Field::new(
            "adjusted",
            "Adjusted points",
            "Coordinates, bearings, and distances after adjustment",
            Kind::List {
                items: ADJUSTED,
                min: 0,
                max: 2001,
            },
        )
        .optional(),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &[
        "PERFECT_CLOSURE",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Latitudes and departures; compass rule corrections ∝ course length; transit rule ∝ |latitude| and |departure|",
    accuracy: "Exact arithmetic; the adjusted traverse closes to within 1e-9 of the unit",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A four-course loop",
        input: r#"{"courses":[{"direction":"0","distance":300.00},{"direction":"90","distance":400.02},{"direction":"180","distance":299.95},{"direction":"270.01","distance":400.00}]}"#,
        source: "add-survey-suite scenario: ΣLat 0.1198, ΣDep 0.0200, misclosure 0.1215, precision about 1:11,525",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("ring", "adjusted")],
    }],
    related: &[
        Related {
            id: "survey.cogo.area-by-coordinates",
            reason: "next",
        },
        Related {
            id: "survey.reduction.combined-factor",
            reason: "next",
        },
        Related {
            id: "survey.land.deed-plot",
            reason: "alternative",
        },
    ],
    sentence: "The traverse precision is {precision}, with a misclosure of {misclosure} over {total_length}.{warn PERFECT_CLOSURE} The closure is perfect, which is unusual for field data.{/warn}",
    limits: &[("batchRows", 1_000)],
    run: run_traverse,
    ..ToolDef::BLANK
};

fn run_traverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows("courses")?;
    let mut legs = Vec::with_capacity(rows.len());
    let mut qs = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let dir = ctx
            .row_text("courses", i, row, "direction")?
            .expect("required");
        let az = direction::parse(&dir)
            .map_err(|m| ToolError::invalid(&format!("/courses/{i}/direction"), m))?;
        let d = ctx
            .row_quantity("courses", i, row, "distance")?
            .expect("required");
        qs.push(d);
        legs.push(az);
    }
    let pointers: Vec<String> = (0..qs.len())
        .map(|i| format!("/courses/{i}/distance"))
        .collect();
    let tagged: Vec<(&str, Q)> = pointers
        .iter()
        .map(String::as_str)
        .zip(qs.iter().copied())
        .collect();
    let u = common_unit(&tagged)?;
    let dists: Vec<f64> = qs.iter().map(|q| q.to(u)).collect();
    if dists.iter().any(|d| *d <= 0.0) {
        return Err(ToolError::invalid(
            "/courses",
            "Every course needs a positive distance.",
        ));
    }
    let lat: Vec<f64> = legs
        .iter()
        .zip(&dists)
        .map(|(az, d)| d * cos(az.to_radians()))
        .collect();
    let dep: Vec<f64> = legs
        .iter()
        .zip(&dists)
        .map(|(az, d)| d * sin(az.to_radians()))
        .collect();
    let (sl, sd): (f64, f64) = (lat.iter().sum(), dep.iter().sum());
    let total: f64 = dists.iter().sum();
    let mis = hypot(sl, sd);
    let perfect = mis <= 1e-12 * total;
    let q = |v: f64| Q { value: v, unit: u };
    let mut out: Vec<(&str, Json)> = Vec::new();
    let ratio = total / mis;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Sum of latitudes",
            "ΣL = Σ distance × cos(bearing)",
            format!("{} courses", lat.len()),
            format!("{} {}", n(sl, 4), u.symbol),
        );
        ctx.step(
            "Sum of departures",
            "ΣD = Σ distance × sin(bearing)",
            format!("{} courses", dep.len()),
            format!("{} {}", n(sd, 4), u.symbol),
        );
        ctx.step(
            "Error of closure",
            "e = √(ΣL² + ΣD²)",
            format!("√({}² + {}²)", n(sl, 4), n(sd, 4)),
            format!("{} {}", n(mis, 4), u.symbol),
        );
        ctx.step(
            "Precision",
            "precision = perimeter / error of closure",
            format!("{} {} / {} {}", n(total, 3), u.symbol, n(mis, 4), u.symbol),
            if perfect {
                "perfect".to_owned()
            } else {
                format!("1:{}", n(ratio, 0))
            },
        );
    }
    out.push((
        "precision",
        Json::str(if perfect {
            "perfect".to_owned()
        } else {
            format!(
                "1:{}",
                display::number(ratio, Precision::Decimals(0), ctx.options.format)
            )
        }),
    ));
    out.push(("misclosure", ctx.emit("misclosure", q(mis), u)));
    // The error of closure points from the true start to where the traverse ends.
    let mis_az = gp_base::angle::wrap_azimuth(atan2(sd, sl).to_degrees());
    out.push((
        "misclosure_bearing",
        Json::str(if perfect {
            "none".to_owned()
        } else {
            direction::bearing(mis_az, 0)
        }),
    ));
    out.push(("sum_latitudes", ctx.emit("sum_latitudes", q(sl), u)));
    out.push(("sum_departures", ctx.emit("sum_departures", q(sd), u)));
    out.push(("total_length", ctx.emit("total_length", q(total), u)));
    if perfect {
        ctx.warnings.push(Warning::new("PERFECT_CLOSURE", "The traverse closes exactly; the precision ratio is undefined and no adjustment is applied."));
    } else {
        out.push(("precision_ratio", Json::Num(ratio)));
    }
    let method = ctx.choice("adjustment")?.unwrap_or("compass");
    if method == "transit" {
        out.push(("note", Json::str("Transit-rule corrections depend on the orientation of the coordinate axes; rotate the grid and the adjustment changes.")));
    }
    if method != "none" {
        let (sum_abs_lat, sum_abs_dep): (f64, f64) = (
            lat.iter().map(|x| x.abs()).sum(),
            dep.iter().map(|x| x.abs()).sum(),
        );
        let n0 = ctx.quantity("start_northing")?.map_or(5000.0, |x| x.to(u));
        let e0 = ctx.quantity("start_easting")?.map_or(5000.0, |x| x.to(u));
        let (mut n, mut e) = (n0, e0);
        let mut rows_out = vec![Json::obj([
            ("point", Json::Num(1.0)),
            ("northing", ctx.emit("northing", q(n), u)),
            ("easting", ctx.emit("easting", q(e), u)),
        ])];
        for i in 0..lat.len() {
            let (cl, cd) = if perfect {
                (0.0, 0.0)
            } else if method == "transit" {
                (
                    if sum_abs_lat == 0.0 {
                        0.0
                    } else {
                        -sl * lat[i].abs() / sum_abs_lat
                    },
                    if sum_abs_dep == 0.0 {
                        0.0
                    } else {
                        -sd * dep[i].abs() / sum_abs_dep
                    },
                )
            } else {
                (-sl * dists[i] / total, -sd * dists[i] / total)
            };
            let (al, ad) = (lat[i] + cl, dep[i] + cd);
            n += al;
            e += ad;
            // Close the last point onto the start exactly (removes round-off).
            if i + 1 == lat.len() {
                n = n0;
                e = e0;
            }
            let az = gp_base::angle::wrap_azimuth(atan2(ad, al).to_degrees());
            rows_out.push(Json::obj([
                ("point", Json::Num((i + 2) as f64)),
                ("northing", ctx.emit("northing", q(n), u)),
                ("easting", ctx.emit("easting", q(e), u)),
                ("bearing", Json::str(direction::bearing(az, 0))),
                ("distance", ctx.emit("distance", q(hypot(al, ad)), u)),
            ]));
        }
        out.push(("adjusted", Json::Arr(rows_out)));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- area

const VERTEX: &[Field] = &[
    len("northing", "Northing", "Like 0").required(),
    len("easting", "Easting", "Like 100").required(),
];

pub static AREA: ToolDef = ToolDef {
    id: "survey.cogo.area-by-coordinates",
    stability: gp_base::tool::Stability::Stable,
    title: "Area by coordinates (acreage)",
    summary: "The area enclosed by plane-survey coordinates by the coordinate (shoelace) method, in square units, acres, and hectares, with the perimeter.",
    aliases: &[
        "acreage calculator",
        "area from coordinates",
        "shoelace area",
    ],
    keywords: &[
        "area",
        "acreage",
        "acres",
        "hectares",
        "shoelace",
        "parcel",
        "coordinates",
    ],
    inputs: &[Field::new(
        "points",
        "Points",
        "Northing and easting of each corner, in order, like 0, 0",
        Kind::List {
            items: VERTEX,
            min: 3,
            max: 100_000,
        },
    )
    .required()
    .core()],
    outputs: &[
        Field::new(
            "acres",
            "Area (acres)",
            "US survey acres when the coordinates are in US survey feet",
            Kind::Quantity {
                q: QT::Area,
                unit: "ac",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "area",
            "Area",
            "In the square of the coordinate unit",
            Kind::Quantity {
                q: QT::Area,
                unit: "ft2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "hectares",
            "Area (hectares)",
            "Hectares",
            Kind::Quantity {
                q: QT::Area,
                unit: "ha",
            },
        )
        .precision(Precision::Decimals(4)),
        len_out("perimeter", "Perimeter", "Total boundary length"),
        Field::new(
            "orientation",
            "Orientation",
            "clockwise or counterclockwise",
            Kind::Text { max_len: 20 },
        ),
    ],
    errors: &[ErrorCode::UnitMismatch, ErrorCode::DegenerateGeometry],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Coordinate (shoelace) method on plane coordinates",
    accuracy: "Exact for plane coordinates; for latitude and longitude use a geodesic area tool",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A 100 × 50 ft rectangle",
        input: r#"{"points":[{"northing":0,"easting":0},{"northing":0,"easting":100},{"northing":50,"easting":100},{"northing":50,"easting":0}]}"#,
        source: "add-survey-suite scenario: 5,000 ft², 0.1148 ac",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("ring", "area")],
    }],
    related: &[
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "parent",
        },
        Related {
            id: "survey.land.deed-plot",
            reason: "alternative",
        },
        Related {
            id: "survey.reduction.combined-factor",
            reason: "next",
        },
    ],
    sentence: "The area is {acres} ({area}), with a perimeter of {perimeter}.",
    limits: &[("batchRows", 1_000)],
    run: run_area,
    ..ToolDef::BLANK
};

fn run_area(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("points")?;
    let mut pts = Vec::new();
    let mut tagged = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let n = ctx
            .row_quantity("points", i, r, "northing")?
            .expect("required");
        let e = ctx
            .row_quantity("points", i, r, "easting")?
            .expect("required");
        tagged.push(n);
        tagged.push(e);
        pts.push((n, e));
    }
    let ptrs: Vec<String> = (0..tagged.len())
        .map(|k| {
            format!(
                "/points/{}/{}",
                k / 2,
                if k % 2 == 0 { "northing" } else { "easting" }
            )
        })
        .collect();
    let u = common_unit(
        &ptrs
            .iter()
            .map(String::as_str)
            .zip(tagged.iter().copied())
            .collect::<Vec<_>>(),
    )?;
    let xy: Vec<(f64, f64)> = pts.iter().map(|(n, e)| (e.to(u), n.to(u))).collect();
    // Shoelace about the first vertex to keep large coordinates from losing digits.
    let (x0, y0) = xy[0];
    let mut twice = 0.0;
    let mut per = 0.0;
    for i in 0..xy.len() {
        let (a, b) = (xy[i], xy[(i + 1) % xy.len()]);
        twice += (a.0 - x0) * (b.1 - y0) - (b.0 - x0) * (a.1 - y0);
        per += hypot(b.0 - a.0, b.1 - a.1);
    }
    let area_u2 = twice.abs() / 2.0;
    if area_u2 == 0.0 {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The points enclose no area (they are collinear or repeated).",
        )
        .at("/points"));
    }
    // Square of the coordinate unit, in m², then report in the matching area units.
    let m = unit(QT::Length, "m");
    let to_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(m);
    let area_m2 = area_u2 * to_m * to_m;
    let m2 = unit(QT::Area, "m2");
    let (area_unit, acre_unit) = match u.symbol {
        "ft" => ("ft2", "ac"),
        "ftUS" => ("ftUS2", "acUS"),
        "m" | "km" | "cm" | "mm" => ("m2", "ac"),
        _ => ("m2", "ac"),
    };
    let area_q = Q {
        value: area_m2,
        unit: m2,
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let acres = Q {
            value: area_m2,
            unit: m2,
        }
        .to(unit(QT::Area, acre_unit));
        ctx.step(
            "Twice the area",
            "2A = Σ (xᵢ − x₀)(yᵢ₊₁ − y₀) − (xᵢ₊₁ − x₀)(yᵢ − y₀)",
            format!(
                "the shoelace sum over {} corners, about the first",
                xy.len()
            ),
            format!("{} {}²", n(twice.abs(), 3), u.symbol),
        );
        ctx.step(
            "Area",
            "A = |2A| / 2",
            format!("{} {}² / 2", n(twice.abs(), 3), u.symbol),
            format!("{} {}²", n(area_u2, 3), u.symbol),
        );
        ctx.step(
            "In acres",
            "acres = A converted from the coordinate unit",
            format!("{} {}²", n(area_u2, 3), u.symbol),
            format!("{} {}", n(acres, 4), acre_unit),
        );
    }
    Ok(Json::obj([
        (
            "acres",
            ctx.emit("acres", area_q, unit(QT::Area, acre_unit)),
        ),
        ("area", ctx.emit("area", area_q, unit(QT::Area, area_unit))),
        (
            "hectares",
            ctx.emit("hectares", area_q, unit(QT::Area, "ha")),
        ),
        (
            "perimeter",
            ctx.emit(
                "perimeter",
                Q {
                    value: per,
                    unit: u,
                },
                u,
            ),
        ),
        (
            "orientation",
            Json::str(if twice > 0.0 {
                "counterclockwise"
            } else {
                "clockwise"
            }),
        ),
    ]))
}

// ---------------------------------------------------------------- stations

/// Reads a station: `12+34.56`, `1+234.567`, or a plain number (in `u`).
pub(crate) fn station(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    match ctx.raw(name).cloned() {
        None => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64().filter(|x| x.is_finite())),
        Some(Value::String(s)) => {
            let t = s.trim();
            let parse = |x: &str| x.trim().parse::<f64>().ok();
            let v = match t.split_once('+') {
                Some((a, b)) => {
                    let (a, b) = (parse(a), parse(b));
                    match (a, b) {
                        (Some(a), Some(b)) if a.fract() == 0.0 && b >= 0.0 => {
                            let digits = t
                                .split_once('+')
                                .map(|(_, r)| r.split('.').next().unwrap_or("").len())
                                .unwrap_or(2);
                            Some(a * 10f64.powi(digits as i32) + b)
                        }
                        _ => None,
                    }
                }
                None => parse(t),
            };
            v.filter(|x| x.is_finite()).map(Some).ok_or_else(|| {
                ToolError::invalid(
                    &format!("/{name}"),
                    format!("\"{s}\" is not a station like 10+00 or 1+234.567."),
                )
            })
        }
        Some(_) => Err(ToolError::invalid(
            &format!("/{name}"),
            "A station is text like 10+00 or a number.",
        )),
    }
}

/// Formats a station: `12+34.56` (per 100 units, feet) or `1+234.567` (per 1,000, meters).
pub(crate) fn fmt_station(v: f64, metric: bool) -> String {
    let (per, dec) = if metric { (1000.0, 3) } else { (100.0, 2) };
    let neg = v < 0.0;
    let a = v.abs();
    let scale = 10f64.powi(dec);
    let total = (a * scale).round();
    let whole = (total / (per * scale)).floor();
    let rest = (total - whole * per * scale) / scale;
    let width = if metric { 7 } else { 5 };
    format!(
        "{}{}+{:0width$.prec$}",
        if neg { "-" } else { "" },
        whole,
        rest,
        width = width,
        prec = dec as usize
    )
}

pub(crate) fn is_metric(u: &Unit) -> bool {
    matches!(u.symbol, "m" | "km" | "cm" | "mm")
}

// ---------------------------------------------------------------- circular curve

const CURVE_ELEMENTS: [&str; 9] = [
    "radius",
    "delta",
    "tangent",
    "length",
    "chord",
    "external",
    "middle_ordinate",
    "degree",
    "degree_chord",
];

pub static CIRCULAR_CURVE: ToolDef = ToolDef {
    id: "survey.curves.circular-curve",
    stability: gp_base::tool::Stability::Stable,
    title: "Horizontal circular curve",
    summary: "All elements of a simple circular curve (radius, deflection, tangent, length, chord, external, middle ordinate, degree of curve) from any two, with PC and PT stations.",
    aliases: &[
        "horizontal curve calculator",
        "curve table",
        "degree of curve",
    ],
    keywords: &[
        "horizontal curve",
        "radius",
        "tangent",
        "chord",
        "external",
        "middle ordinate",
        "PC",
        "PT",
        "stationing",
    ],
    inputs: &[
        len("radius", "Radius R", "Like 500 ft").core(),
        Field::new(
            "delta",
            "Deflection Δ",
            "Like 30°00'00\" or 30 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("unbounded"),
        len("tangent", "Tangent T", "Distance PI to PC, like 250 ft").core(),
        len("length", "Curve length L", "Along the arc, like 480 ft"),
        len("chord", "Long chord C", "PC to PT, like 470 ft"),
        len(
            "external",
            "External E",
            "PI to the curve midpoint, like 30 ft",
        ),
        len(
            "middle_ordinate",
            "Middle ordinate M",
            "Chord midpoint to the curve midpoint, like 28 ft",
        ),
        Field::new(
            "degree",
            "Degree of curve (arc)",
            "Per 100 ft of arc, like 11.4592 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded"),
        Field::new(
            "degree_chord",
            "Degree of curve (chord)",
            "Per 100 ft chord, like 15 deg (railroads and older highway plans)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded"),
        Field::new(
            "pi_station",
            "PI station",
            "Like 12+34.56",
            Kind::Text { max_len: 20 },
        )
        .core(),
    ],
    outputs: &[
        len_out("radius", "Radius R", "Radius"),
        Field::new(
            "delta",
            "Deflection Δ",
            "Central angle",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .angle_range("unbounded"),
        len_out("tangent", "Tangent T", "PI to PC"),
        len_out("length", "Curve length L", "Along the arc, like 480 ft"),
        len_out("chord", "Long chord C", "PC to PT, like 470 ft"),
        len_out("external", "External E", "PI to curve midpoint"),
        len_out(
            "middle_ordinate",
            "Middle ordinate M",
            "Chord midpoint to curve midpoint",
        ),
        Field::new(
            "degree_arc",
            "Degree of curve (arc)",
            "Central angle per 100 ft of arc",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
        Field::new(
            "degree_chord",
            "Degree of curve (chord)",
            "Central angle for a 100 ft chord",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded")
        .optional(),
        Field::new(
            "pc_station",
            "PC station",
            "Point of curvature",
            Kind::Text { max_len: 20 },
        )
        .optional(),
        Field::new(
            "pt_station",
            "PT station",
            "Point of tangency",
            Kind::Text { max_len: 20 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &[
        "AMBIGUOUS_INPUT",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "T = R·tan(Δ/2), L = R·Δ, C = 2R·sin(Δ/2), E = R(sec(Δ/2) − 1), M = R(1 − cos(Δ/2)); D(arc) = 5,729.578/R ft, sin(D(chord)/2) = 50/R ft; PT station = PC + L along the arc",
    accuracy: "Exact; two given elements other than R and Δ are solved by bisection to 1e-15 relative",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "R = 500 ft, Δ = 30°",
        input: r#"{"radius":"500 ft","delta":"30 deg","pi_station":"12+34.56"}"#,
        source: "add-survey-suite scenario: T 133.975, L 261.799, C 258.819, E 17.638, M 17.037 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("radius", "radius"), ("angle", "delta")],
    }],
    related: &[
        Related {
            id: "survey.curves.vertical-curve",
            reason: "next",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "next",
        },
        Related {
            id: "survey.earthwork.average-end-area",
            reason: "next",
        },
    ],
    sentence: "The curve has tangent {tangent}, length {length}, and long chord {chord}.",
    limits: &[("batchRows", 10_000)],
    run: run_circular,
    // "radius 500 ft delta 30": delta is a deflection angle in degrees.
    slots: &[
        Slot::new("radius", &["radius", "r"]),
        Slot::new("delta", &["delta", "deflection", "central", "intersection"]).range(0.0, 180.0),
        Slot::new("tangent", &["tangent", "t"]),
        Slot::new("length", &["length", "l", "arc"]),
        Slot::new("chord", &["chord", "c"]),
        Slot::new("external", &["external", "e"]),
        Slot::new("middle_ordinate", &["ordinate", "m"]),
        Slot::new("degree", &["degree", "d"]),
        Slot::new("degree_chord", &["dc"]),
        Slot::new("pi_station", &["pi", "station"]),
    ],
    ..ToolDef::BLANK
};

/// Element as a function of the half-angle θ = Δ/2, per unit radius.
fn element(name: &str, th: f64) -> f64 {
    match name {
        "tangent" => tan(th),
        "length" => 2.0 * th,
        "chord" => 2.0 * sin(th),
        // Written without cancellation, so tiny angles stay accurate.
        "external" => 2.0 * sin(th / 2.0) * sin(th / 2.0) / cos(th),
        _ => 2.0 * sin(th / 2.0) * sin(th / 2.0), // middle ordinate
    }
}

fn run_circular(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let given: Vec<&str> = CURVE_ELEMENTS
        .iter()
        .copied()
        .filter(|n| ctx.is_set(n))
        .collect();
    // R and each degree of curve fix the same element, the radius.
    let radii = given
        .iter()
        .filter(|n| matches!(**n, "radius" | "degree" | "degree_chord"))
        .count();
    if given.len() != 2 || radii > 1 {
        return Err(ToolError::invalid(
            "/radius",
            "Give exactly two independent elements, such as R and Δ, or T and L. More than two can be inconsistent.",
        ));
    }
    let mut lens: Vec<(&str, Q)> = Vec::new();
    for n in given
        .iter()
        .filter(|n| !matches!(**n, "delta" | "degree" | "degree_chord"))
    {
        lens.push((*n, ctx.req_quantity(n)?));
    }
    let u = if lens.is_empty() {
        unit(QT::Length, "ft")
    } else {
        common_unit(&lens)?
    };
    let get = |n: &str| lens.iter().find(|(k, _)| *k == n).map(|(_, q)| q.to(u));
    let ft = unit(QT::Length, "ft");
    let to_ft = Q {
        value: 1.0,
        unit: u,
    }
    .to(ft);
    let mut r = get("radius");
    if let Some(d) = gp_geo::point::plain_angle(ctx, "degree")? {
        if d <= 0.0 || d >= 180.0 {
            return Err(ToolError::invalid(
                "/degree",
                "The degree of curve must be between 0° and 180°.",
            ));
        }
        r = Some((18_000.0 / core::f64::consts::PI) / d / to_ft);
    }
    if let Some(d) = gp_geo::point::plain_angle(ctx, "degree_chord")? {
        if d <= 0.0 || d >= 180.0 {
            return Err(ToolError::invalid(
                "/degree_chord",
                "The degree of curve must be between 0° and 180°.",
            ));
        }
        r = Some(50.0 / sin(d.to_radians() / 2.0) / to_ft);
    }
    let delta = gp_geo::point::plain_angle(ctx, "delta")?.map(f64::to_radians);
    if let Some(d) = delta
        && (d <= 0.0 || d >= core::f64::consts::TAU)
    {
        return Err(ToolError::invalid(
            "/delta",
            "The deflection must be between 0° and 360°.",
        ));
    }
    let others: Vec<(&str, f64)> = lens
        .iter()
        .filter(|(k, _)| *k != "radius")
        .map(|(k, q)| (*k, q.to(u)))
        .collect();
    // Point at the element that is not positive.
    let bad = others
        .iter()
        .find(|(_, v)| *v <= 0.0)
        .map(|(k, _)| *k)
        .or(r.filter(|r| *r <= 0.0).map(|_| "radius"));
    if let Some(bad) = bad {
        return Err(ToolError::invalid(
            &format!("/{bad}"),
            "Curve elements must be positive.",
        ));
    }
    let bad = || {
        ToolError::invalid(
            "/radius",
            "These two elements do not form a circular curve; check the values.",
        )
    };
    let (r, th) = match (r, delta) {
        (Some(r), Some(d)) => (r, d / 2.0),
        (Some(r), None) => {
            let (k, v) = others[0];
            let x = v / r;
            let th = match k {
                "tangent" => atan(x),
                "length" => x / 2.0,
                "chord" if x <= 2.0 => asin(x / 2.0),
                "external" => acos(1.0 / (1.0 + x)),
                "middle_ordinate" if x < 1.0 => acos(1.0 - x),
                _ => return Err(bad()),
            };
            (r, th)
        }
        (None, Some(d)) => {
            let (k, v) = others[0];
            (v / element(k, d / 2.0), d / 2.0)
        }
        (None, None) => {
            let ((ka, va), (kb, vb)) = (others[0], others[1]);
            // Solve element(a)/element(b) = va/vb for the half-angle. Most
            // ratios are monotonic, but tangent over middle ordinate is not,
            // so scan (0°, 90°) for every sign change and bisect each one.
            let f = |th: f64| element(ka, th) / element(kb, th) - va / vb;
            let n = 720;
            let edge = 1e-7;
            let at = |i: usize| {
                (core::f64::consts::FRAC_PI_2 * i as f64 / n as f64)
                    .clamp(edge, core::f64::consts::FRAC_PI_2 - edge)
            };
            let mut roots = Vec::new();
            for i in 0..n {
                let (mut lo, mut hi) = (at(i), at(i + 1));
                if (f(lo) < 0.0) == (f(hi) < 0.0) {
                    continue;
                }
                let rising = f(hi) > f(lo);
                for _ in 0..100 {
                    let mid = (lo + hi) / 2.0;
                    if (f(mid) < 0.0) == rising {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                roots.push((lo + hi) / 2.0);
            }
            let Some(&th) = roots.first() else {
                return Err(bad());
            };
            if let Some(&other) = roots.get(1) {
                ctx.warnings.push(Warning::new(
                    "AMBIGUOUS_INPUT",
                    format!(
                        "Two curves fit these elements; this is the one with Δ = {:.4}°. The other has Δ = {:.4}°.",
                        (2.0 * th).to_degrees(),
                        (2.0 * other).to_degrees()
                    ),
                ));
            }
            (va / element(ka, th), th)
        }
    };
    let q = |v: f64| Q { value: v, unit: u };
    let r_ft = r * to_ft;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let delta_deg = (2.0 * th).to_degrees();
        ctx.step(
            "Radius",
            "R = the radius the two given elements fix",
            format!("from {}", given.join(" and ")),
            format!("{} {}", n(r, 3), u.symbol),
        );
        ctx.step(
            "Tangent",
            "T = R × tan(Δ/2)",
            format!("{} {} × tan({}° / 2)", n(r, 3), u.symbol, n(delta_deg, 4)),
            format!("{} {}", n(r * tan(th), 3), u.symbol),
        );
        ctx.step(
            "Curve length",
            "L = R × Δ in radians",
            format!("{} {} × {} rad", n(r, 3), u.symbol, n(2.0 * th, 6)),
            format!("{} {}", n(2.0 * r * th, 3), u.symbol),
        );
    }
    let mut out = vec![
        ("radius", ctx.emit("radius", q(r), u)),
        ("delta", ctx.out("delta", deg((2.0 * th).to_degrees()))),
        ("tangent", ctx.emit("tangent", q(r * tan(th)), u)),
        ("length", ctx.emit("length", q(2.0 * r * th), u)),
        ("chord", ctx.emit("chord", q(2.0 * r * sin(th)), u)),
        (
            "external",
            ctx.emit("external", q(r * (1.0 / cos(th) - 1.0)), u),
        ),
        (
            "middle_ordinate",
            ctx.emit("middle_ordinate", q(r * (1.0 - cos(th))), u),
        ),
        (
            "degree_arc",
            ctx.out("degree_arc", deg(18_000.0 / core::f64::consts::PI / r_ft)),
        ),
    ];
    if r_ft >= 50.0 {
        out.push((
            "degree_chord",
            ctx.out("degree_chord", deg(2.0 * asin(50.0 / r_ft).to_degrees())),
        ));
    }
    if let Some(pi) = station(ctx, "pi_station")? {
        let metric = is_metric(u);
        let pc = pi - r * tan(th);
        out.push(("pc_station", Json::str(fmt_station(pc, metric))));
        out.push((
            "pt_station",
            Json::str(fmt_station(pc + 2.0 * r * th, metric)),
        ));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- vertical curve

pub static VERTICAL_CURVE: ToolDef = ToolDef {
    id: "survey.curves.vertical-curve",
    stability: gp_base::tool::Stability::Stable,
    title: "Vertical curve",
    summary: "PVC and PVT stations and elevations, the high or low point, and the K value of a symmetric parabolic vertical curve.",
    aliases: &[
        "vertical curve calculator",
        "high point of a vertical curve",
        "K value",
    ],
    keywords: &[
        "vertical curve",
        "PVC",
        "PVI",
        "PVT",
        "grade",
        "high point",
        "low point",
        "K value",
        "crest",
        "sag",
    ],
    inputs: &[
        Field::new(
            "g1",
            "Incoming grade g1",
            "Percent, like +2",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "g2",
            "Outgoing grade g2",
            "Percent, like -3",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .required()
        .core(),
        len("length", "Curve length L", "Horizontal length, like 600 ft").core(),
        Field::new(
            "k",
            "K value",
            "Instead of L: length per percent of grade change, like 40 \u{2014} look your design speed up in AASHTO Green Book Table 3-34",
            Kind::Number { min: 0.0, max: 1e6 },
        ),
        Field::new(
            "pvi_station",
            "PVI station",
            "Like 10+00",
            Kind::Text { max_len: 20 },
        )
        .required()
        .core(),
        len("pvi_elevation", "PVI elevation", "Like 100.00 ft")
            .required()
            .core(),
    ],
    outputs: &[
        Field::new(
            "turning_station",
            "High or low point station",
            "Where the grade is zero, if inside the curve",
            Kind::Text { max_len: 20 },
        )
        .optional(),
        Field::new(
            "turning_elevation",
            "High or low point elevation",
            "Elevation there",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "pvc_station",
            "PVC station",
            "Start of the curve",
            Kind::Text { max_len: 20 },
        ),
        Field::new(
            "pvc_elevation",
            "PVC elevation",
            "Start of the curve",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "pvt_station",
            "PVT station",
            "End of the curve",
            Kind::Text { max_len: 20 },
        ),
        Field::new(
            "pvt_elevation",
            "PVT elevation",
            "End of the curve",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "k",
            "K value",
            "L / |g2 − g1|, with grades in percent",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Significant(4)),
        Field::new(
            "curve_type",
            "Turning point",
            "high (crest curve) or low (sag curve)",
            Kind::Text { max_len: 8 },
        ),
        Field::new(
            "turning_note",
            "High or low point",
            "Explanation when none is inside the curve",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Symmetric parabola: y(x) = y_PVC + g1·x + (g2 − g1)·x² / (2L)",
    accuracy: "Exact",
    references: &[GHILANI, AASHTO_GREEN_BOOK],
    examples: &[Example {
        id: "primary",
        title: "A crest curve: +2% to −3% over 600 ft, PVI 10+00 at 100.00 ft",
        input: r#"{"g1":2,"g2":-3,"length":"600 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
        source: "add-survey-suite scenario: PVC 7+00 at 94.00 ft; high point 9+40 at 96.40 ft; K = 120",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "pvc_elevation")],
    }],
    related: &[
        Related {
            id: "survey.curves.circular-curve",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.average-end-area",
            reason: "next",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "alternative",
        },
    ],
    sentence: "The curve starts at {pvc_station} and ends at {pvt_station}, with K = {k}.{if turning_elevation > -1e12} The {curve_type} point is at {turning_station}, elevation {turning_elevation}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_vertical,
    ..ToolDef::BLANK
};

fn run_vertical(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g1 = ctx.number("g1")?.expect("required");
    let g2 = ctx.number("g2")?.expect("required");
    let pvi_e = ctx.req_quantity("pvi_elevation")?;
    let (l, u) = match (ctx.quantity("length")?, ctx.number("k")?) {
        (Some(l), None) => {
            let u = common_unit(&[("/length", l), ("/pvi_elevation", pvi_e)])?;
            (l.to(u), u)
        }
        (None, Some(k)) => (k * (g2 - g1).abs(), pvi_e.unit),
        _ => {
            return Err(ToolError::invalid(
                "/length",
                "Give the curve length or the K value, not both.",
            ));
        }
    };
    if l <= 0.0 || g1 == g2 {
        return Err(ToolError::invalid(
            "/length",
            "The curve needs a positive length and two different grades.",
        ));
    }
    let pvi_s = station(ctx, "pvi_station")?.expect("required");
    let y_pvi = pvi_e.to(u);
    let (a1, a2) = (g1 / 100.0, g2 / 100.0);
    let pvc_s = pvi_s - l / 2.0;
    let y_pvc = y_pvi - a1 * l / 2.0;
    let y_pvt = y_pvi + a2 * l / 2.0;
    let metric = is_metric(u);
    let q = |v: f64| Q { value: v, unit: u };
    let crest = a2 < a1;
    let mut out: Vec<(&str, Json)> = Vec::new();
    let x = -a1 * l / (a2 - a1);
    let turning =
        (x > 0.0 && x < l).then(|| (pvc_s + x, y_pvc + a1 * x + (a2 - a1) * x * x / (2.0 * l)));
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |v: f64, d: u8| display::number(v, Precision::Decimals(d), fmt);
        let a = g2 - g1;
        ctx.step(
            "Algebraic difference",
            "A = g2 − g1, in percent",
            format!("{}% − {}%", n(g2, 3), n(g1, 3)),
            format!("{}%", n(a, 3)),
        );
        ctx.step(
            "Rate of change",
            "K = L / |A|, the length per percent of grade change",
            format!("{} {} / {}%", n(l, 3), u.symbol, n(a.abs(), 3)),
            format!("{} {} per %", n(l / a.abs(), 3), u.symbol),
        );
        match turning {
            Some((st, _)) => ctx.step(
                if crest { "High point" } else { "Low point" },
                "x = −g1 × L / A from the PVC, where the grade reaches zero",
                format!(
                    "−{}% × {} {} / {}%, from station {}",
                    n(g1, 3),
                    n(l, 3),
                    u.symbol,
                    n(a, 3),
                    fmt_station(pvc_s, metric)
                ),
                fmt_station(st, metric),
            ),
            None => ctx.step(
                "No high or low point",
                "both grades run the same way, so the curve has none inside it",
                format!("g1 {}% and g2 {}%", n(g1, 3), n(g2, 3)),
                fmt_station(pvc_s, metric),
            ),
        }
    }
    if let Some((s, y)) = turning {
        out.push(("turning_station", Json::str(fmt_station(s, metric))));
        out.push(("turning_elevation", ctx.emit("turning_elevation", q(y), u)));
    }
    out.push(("pvc_station", Json::str(fmt_station(pvc_s, metric))));
    out.push(("pvc_elevation", ctx.emit("pvc_elevation", q(y_pvc), u)));
    out.push(("pvt_station", Json::str(fmt_station(pvc_s + l, metric))));
    out.push(("pvt_elevation", ctx.emit("pvt_elevation", q(y_pvt), u)));
    out.push(("k", Json::Num(l / (g2 - g1).abs())));
    out.push(("curve_type", Json::str(if crest { "high" } else { "low" })));
    if turning.is_none() {
        out.push(("turning_note", Json::str("No high or low point lies within the curve: both grades have the same sign, so the highest or lowest point is at an end.")));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- earthwork

const fn area_in(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Area,
            unit: "ft2",
        },
    )
}

const VOLUME_OUT: [Field; 2] = [
    Field::new(
        "volume",
        "Volume",
        "Volume between the sections",
        Kind::Quantity {
            q: QT::Volume,
            unit: "yd3",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "volume_ft3",
        "Volume (ft³)",
        "In cubic feet",
        Kind::Quantity {
            q: QT::Volume,
            unit: "ft3",
        },
    )
    .precision(Precision::Decimals(1)),
];

pub static AVERAGE_END_AREA: ToolDef = ToolDef {
    id: "survey.earthwork.average-end-area",
    title: "Average end area volume",
    summary: "Earthwork volume between two cross-sections by the average end area method, in cubic yards and cubic feet.",
    aliases: &["cubic yards calculator", "end area volume"],
    keywords: &[
        "earthwork",
        "cut",
        "fill",
        "cross-section",
        "cubic yards",
        "average end area",
    ],
    inputs: &[
        area_in("area1", "End area A1", "Like 120 ft²")
            .required()
            .core(),
        area_in("area2", "End area A2", "Like 180 ft²")
            .required()
            .core(),
        len("length", "Distance between sections", "Like 100 ft")
            .required()
            .core(),
    ],
    outputs: &[VOLUME_OUT[0], VOLUME_OUT[1]],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "V = L·(A1 + A2)/2",
    accuracy: "Exact for the method; overestimates for pyramidal sections compared with the prismoidal formula",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "120 and 180 ft² sections 100 ft apart",
        input: r#"{"area1":"120 ft2","area2":"180 ft2","length":"100 ft"}"#,
        source: "add-survey-suite scenario: 15,000 ft³ = 555.56 yd³",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.earthwork.prismoidal",
        reason: "alternative",
    }],
    sentence: "The volume is {volume} ({volume_ft3}).",
    limits: &[("batchRows", 10_000)],
    run: run_aea,
    ..ToolDef::BLANK
};

fn areas_len(ctx: &mut Ctx) -> Result<(f64, f64, f64), ToolError> {
    let m2 = unit(QT::Area, "m2");
    let a1 = ctx.req_quantity("area1")?.to(m2);
    let a2 = ctx.req_quantity("area2")?.to(m2);
    let l = ctx.req_quantity("length")?.to(unit(QT::Length, "m"));
    if a1 < 0.0 || a2 < 0.0 || l <= 0.0 {
        return Err(ToolError::invalid(
            "/length",
            "Areas cannot be negative and the length must be positive.",
        ));
    }
    Ok((a1, a2, l))
}

fn volume_out(ctx: &mut Ctx, v_m3: f64) -> Vec<(&'static str, Json)> {
    let q = Q {
        value: v_m3,
        unit: unit(QT::Volume, "m3"),
    };
    vec![
        ("volume", ctx.out("volume", q)),
        (
            "volume_ft3",
            ctx.emit("volume_ft3", q, unit(QT::Volume, "ft3")),
        ),
    ]
}

fn run_aea(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (a1, a2, l) = areas_len(ctx)?;
    Ok(Json::obj(volume_out(ctx, l * (a1 + a2) / 2.0)))
}

pub static PRISMOIDAL: ToolDef = ToolDef {
    id: "survey.earthwork.prismoidal",
    title: "Prismoidal volume",
    summary: "Earthwork volume by the prismoidal formula with a measured middle section, and its difference from the average end area method.",
    aliases: &["prismoidal formula"],
    keywords: &[
        "prismoidal",
        "earthwork",
        "volume",
        "middle area",
        "cut",
        "fill",
    ],
    inputs: &[
        area_in("area1", "End area A1", "Like 120 ft²")
            .required()
            .core(),
        area_in("area2", "End area A2", "Like 180 ft²")
            .required()
            .core(),
        area_in(
            "area_middle",
            "Measured middle area Am",
            "Measured at the midpoint, like 148 ft²",
        )
        .required()
        .core(),
        len("length", "Distance between end sections", "Like 100 ft")
            .required()
            .core(),
    ],
    outputs: &[
        VOLUME_OUT[0],
        VOLUME_OUT[1],
        Field::new(
            "difference",
            "Average end area minus prismoidal",
            "Overestimate by the simpler method",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(2)),
    ],
    warnings: &[
        "PRISMOIDAL_MIDDLE_AREA_AVERAGED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "V = L/6·(A1 + 4·Am + A2)",
    accuracy: "Exact for prismoids; needs a measured middle area",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Middle area 148 ft² measured",
        input: r#"{"area1":"120 ft2","area2":"180 ft2","area_middle":"148 ft2","length":"100 ft"}"#,
        source: "add-survey-suite scenario: 14,866.7 ft³",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.earthwork.average-end-area",
        reason: "alternative",
    }],
    sentence: "The prismoidal volume is {volume} ({volume_ft3}), {abs(difference)} less than average end areas.{warn PRISMOIDAL_MIDDLE_AREA_AVERAGED} The middle area looks averaged, not measured.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_prismoidal,
    ..ToolDef::BLANK
};

fn run_prismoidal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (a1, a2, l) = areas_len(ctx)?;
    let am = ctx.req_quantity("area_middle")?.to(unit(QT::Area, "m2"));
    let mean = (a1 + a2) / 2.0;
    if (am - mean).abs() <= 1e-9 * mean.max(1e-12) {
        ctx.warnings.push(
            Warning::new(
                "PRISMOIDAL_MIDDLE_AREA_AVERAGED",
                "The middle area equals the average of the end areas, which makes the prismoidal formula the same as average end area. Measure the middle section.",
            )
            .at("/area_middle"),
        );
    }
    let v = l / 6.0 * (a1 + 4.0 * am + a2);
    let mut out = volume_out(ctx, v);
    let d = Q {
        value: l * mean - v,
        unit: unit(QT::Volume, "m3"),
    };
    out.push(("difference", ctx.out("difference", d)));
    Ok(Json::obj(out))
}

pub static SHRINK_SWELL: ToolDef = ToolDef {
    id: "survey.earthwork.shrink-swell",
    title: "Swell, shrink, and truck loads",
    summary: "Loose and compacted volumes from a bank volume with your swell and shrink factors, and the truck loads to haul it.",
    aliases: &[
        "truck loads calculator",
        "swell factor",
        "bank to loose yards",
    ],
    keywords: &[
        "swell",
        "shrink",
        "bank",
        "loose",
        "compacted",
        "haul",
        "truck loads",
    ],
    inputs: &[
        Field::new(
            "bank_volume",
            "Bank volume",
            "In place, like 1000 yd3",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .required()
        .core(),
        Field::new(
            "swell",
            "Swell",
            "Percent, like 25 (soil-specific; typical values are reference only)",
            Kind::Number {
                min: 0.0,
                max: 200.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "shrink",
            "Shrink",
            "Percent, like 10",
            Kind::Number {
                min: 0.0,
                max: 90.0,
            },
        )
        .core(),
        Field::new(
            "truck_capacity",
            "Truck capacity",
            "Loose volume per load, like 12 yd3",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "loose_volume",
            "Loose volume",
            "After excavation",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "compacted_volume",
            "Compacted volume",
            "After placement",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "loads",
            "Truck loads",
            "Rounded up",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(0))
        .optional(),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Loose = bank × (1 + swell); compacted = bank × (1 − shrink)",
    accuracy: "Exact for the factors given; real factors vary with soil and moisture",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "1,000 bank yd³ at 25% swell in 12 yd³ trucks",
        input: r#"{"bank_volume":"1000 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
        source: "add-survey-suite scenario: 1,250 loose yd³, 105 loads",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.earthwork.average-end-area",
        reason: "parent",
    }],
    sentence: "That is {loose_volume} loose{if loads > 0}, or {loads} truck loads{/if}.",
    limits: &[("batchRows", 10_000)],
    run: run_shrink_swell,
    ..ToolDef::BLANK
};

fn run_shrink_swell(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let bank = ctx.req_quantity("bank_volume")?;
    let m3 = unit(QT::Volume, "m3");
    let b = bank.to(m3);
    if b < 0.0 {
        return Err(ToolError::invalid(
            "/bank_volume",
            "The volume cannot be negative.",
        ));
    }
    let swell = ctx.number("swell")?.expect("required") / 100.0;
    let loose = b * (1.0 + swell);
    let mut out = vec![(
        "loose_volume",
        ctx.out(
            "loose_volume",
            Q {
                value: loose,
                unit: m3,
            },
        ),
    )];
    if let Some(sh) = ctx.number("shrink")? {
        out.push((
            "compacted_volume",
            ctx.out(
                "compacted_volume",
                Q {
                    value: b * (1.0 - sh / 100.0),
                    unit: m3,
                },
            ),
        ));
    }
    if let Some(cap) = ctx.quantity("truck_capacity")? {
        let c = cap.to(m3);
        if c <= 0.0 {
            return Err(ToolError::invalid(
                "/truck_capacity",
                "Truck capacity must be positive.",
            ));
        }
        // Round up, allowing for binary round-off on exact multiples.
        let loads = (loose / c * (1.0 - 1e-12)).ceil();
        out.push(("loads", Json::Num(loads)));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- reductions

pub static COMBINED_FACTOR: ToolDef = ToolDef {
    id: "survey.reduction.combined-factor",
    stability: gp_base::tool::Stability::Stable,
    title: "Combined scale factor (grid and ground)",
    summary: "The elevation factor and combined factor from a grid scale factor and height, and conversion of a distance between grid and ground.",
    aliases: &[
        "grid to ground",
        "ground to grid",
        "combined factor calculator",
    ],
    keywords: &[
        "combined factor",
        "grid factor",
        "elevation factor",
        "grid to ground",
        "state plane",
        "scale factor",
    ],
    inputs: &[
        Field::new(
            "grid_scale",
            "Grid scale factor",
            "Point scale factor from the projection, like 0.99991",
            Kind::Number { min: 0.9, max: 1.1 },
        )
        .required()
        .core(),
        len(
            "ellipsoid_height",
            "Ellipsoid height",
            "Height above the ellipsoid, like 1,500 ft (or give elevation and geoid height)",
        )
        .core(),
        len("elevation", "Orthometric elevation", "Like 1,530 ft"),
        len(
            "geoid_height",
            "Geoid height N",
            "Like -30 ft (negative in the conterminous US)",
        ),
        len(
            "ground_distance",
            "Ground distance",
            "To convert to grid, like 1000 ft",
        )
        .core(),
        len(
            "grid_distance",
            "Grid distance",
            "To convert to ground, like 1000 ft",
        ),
        len(
            "radius",
            "Earth radius",
            "Default 20,906,000 ft (6,372,000 m), as in NOAA Manual NOS NGS 5",
        ),
    ],
    outputs: &[
        Field::new(
            "combined_factor",
            "Combined factor",
            "Grid factor × elevation factor",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(8)),
        Field::new(
            "elevation_factor",
            "Elevation factor",
            "R / (R + h)",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(8)),
        len_out("grid_distance", "Grid distance", "Ground × combined factor").optional(),
        len_out(
            "ground_distance",
            "Ground distance",
            "Grid ÷ combined factor",
        )
        .optional(),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Elevation factor = R/(R + h) with h the ellipsoid height; combined factor = grid factor × elevation factor (Stem 1990)",
    accuracy: "About 1 ppm with the mean radius; better with a local radius of curvature",
    references: &[STEM],
    examples: &[Example {
        id: "primary",
        title: "Grid factor 0.99991 at 1,500 ft ellipsoid height",
        input: r#"{"grid_scale":0.99991,"ellipsoid_height":"1500 ft","ground_distance":"1000 ft"}"#,
        source: "NOAA Manual NOS NGS 5 formula with R = 20,906,000 ft",
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
            id: "geodesy.spcs.spcs83-forward",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "next",
        },
    ],
    sentence: "The combined factor is {combined_factor}.{if grid_distance > 0} A ground distance of {ground_distance} is {grid_distance} on the grid.{/if}{warn ORTHOMETRIC_AS_ELLIPSOIDAL} The elevation was used as the ellipsoid height.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_combined,
    ..ToolDef::BLANK
};

fn run_combined(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let k = ctx.number("grid_scale")?.expect("required");
    let m = unit(QT::Length, "m");
    let r = ctx.quantity("radius")?.map_or(6_372_000.0, |q| q.to(m));
    if !(1_000_000.0..=10_000_000.0).contains(&r) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The Earth radius must be between 1,000 km and 10,000 km.",
        )
        .at("/radius"));
    }
    let h = match (
        ctx.quantity("ellipsoid_height")?,
        ctx.quantity("elevation")?,
        ctx.quantity("geoid_height")?,
    ) {
        (Some(h), None, None) => h.to(m),
        (None, Some(e), Some(n)) => e.to(m) + n.to(m),
        (None, Some(e), None) => {
            ctx.warnings.push(Warning::new(
                "ORTHOMETRIC_AS_ELLIPSOIDAL",
                "No geoid height was given, so the elevation was used as the ellipsoid height. In the conterminous US this makes the elevation factor too small by up to about 5 ppm.",
            ));
            e.to(m)
        }
        _ => {
            return Err(ToolError::invalid(
                "/ellipsoid_height",
                "Give the ellipsoid height, or the elevation (with the geoid height).",
            ));
        }
    };
    let ef = r / (r + h);
    let cf = k * ef;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Elevation factor",
            "EF = R / (R + h)",
            format!("{} m / ({} m + {} m)", n(r, 0), n(r, 0), n(h, 3)),
            n(ef, 8),
        );
        ctx.step(
            "Combined factor",
            "CF = grid scale factor × elevation factor",
            format!("{} × {}", n(k, 8), n(ef, 8)),
            n(cf, 8),
        );
    }
    let mut out = vec![
        ("combined_factor", Json::Num(cf)),
        ("elevation_factor", Json::Num(ef)),
    ];
    match (
        ctx.quantity("ground_distance")?,
        ctx.quantity("grid_distance")?,
    ) {
        (Some(g), None) => {
            out.push((
                "grid_distance",
                ctx.emit(
                    "grid_distance",
                    Q {
                        value: g.value * cf,
                        unit: g.unit,
                    },
                    g.unit,
                ),
            ));
            out.push(("ground_distance", ctx.emit("ground_distance", g, g.unit)));
        }
        (None, Some(g)) => {
            out.push(("grid_distance", ctx.emit("grid_distance", g, g.unit)));
            out.push((
                "ground_distance",
                ctx.emit(
                    "ground_distance",
                    Q {
                        value: g.value / cf,
                        unit: g.unit,
                    },
                    g.unit,
                ),
            ));
        }
        (None, None) => {}
        _ => {
            return Err(ToolError::invalid(
                "/grid_distance",
                "Give a ground distance or a grid distance, not both.",
            ));
        }
    }
    Ok(Json::obj(out))
}

pub static TOOLS: &[&ToolDef] = &[
    &INVERSE,
    &FORWARD,
    &TRAVERSE,
    &AREA,
    &CIRCULAR_CURVE,
    &VERTICAL_CURVE,
    &layout::CURVE_LAYOUT,
    &vcurve::UNEQUAL,
    &spiral::SPIRAL,
    &sight::SIGHT_DISTANCE,
    &profile::PROFILE_GRADES,
    &AVERAGE_END_AREA,
    &PRISMOIDAL,
    &SHRINK_SWELL,
    &grade::GRADE,
    &staking::SLOPE_STAKE,
    &section::SECTION_AREA,
    &borrow::BORROW_PIT,
    &stockpile::STOCKPILE,
    &stockpile::SOLID,
    &COMBINED_FACTOR,
    &reduction::SLOPE,
    &reduction::CURVATURE,
    &reduction::STADIA,
    &reduction::INACCESSIBLE,
    &reduction::OFFSET,
    &intersect::INTERSECTION,
    &intersect::RESECTION,
    &closure::ANGULAR,
    &stationing::STATION_OFFSET,
    &leveling::LEVEL_RUN,
    &land::LEGACY_UNITS,
    &land::DEED_PARSE,
    &land::DEED_PLOT,
    &land::PLSS_PARSE,
    &land::BASIS_ROTATION,
];

pub static REGISTRY: Registry = Registry {
    module: "survey",
    tools: TOOLS,
};

gp_base::export_module!("survey", REGISTRY);
