//! Datum transformations (geodesy/datums-and-transformations spec): a
//! generic 7- or 14-parameter Helmert tool with an explicit convention, and
//! transformations between ITRF realizations (and the WGS 84 realizations
//! aligned with them) using the IERS ITRF2020 parameters.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ellipsoid::{CATALOG, Ellipsoid};
use gp_geo::frames as fr;
use gp_geo::helmert::{self, Convention, Helmert, Params};
use gp_geo::point;

const IOGP_7_2: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "§4.2.3 Helmert 7-parameter and §4.2.5 time-dependent transformations (EPSG methods 1032, 1033, 1053, 1056)",
    url: "https://www.iogp.org/wp-content/uploads/2019/09/373-07-02.pdf",
};
const IERS_ITRF2020: Reference = Reference {
    title: "ITRF2020: transformation parameters from ITRF2020 to past ITRFs",
    issuer: "IERS ITRF Center, IGN France",
    year: 2022,
    edition: "Transfo-ITRF2020_TRFs.txt",
    locator: "Parameters at epoch 2015.0 with rates, position-vector convention (formula 1)",
    url: "https://itrf.ign.fr/docs/solutions/itrf2020/Transfo-ITRF2020_TRFs.txt",
};
const NGA_WGS84: Reference = Reference {
    title: "Department of Defense World Geodetic System 1984, NGA.STND.0036",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2014,
    edition: "NGA.STND.0036_1.0.0_WGS84",
    locator: "Chapter 2: realizations of WGS 84 and their alignment with the ITRF",
    url: "https://earth-info.nga.mil/php/download.php?file=coord-wgs84",
};

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}
fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}
fn read(ctx: &mut Ctx, name: &str, q: QT, unit: &str) -> Result<Option<f64>, ToolError> {
    let u = units::by_symbol(q, unit).expect("registered unit");
    Ok(ctx.quantity(name)?.map(|v| v.to(u)))
}

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    unit: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit })
}
const fn mm_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(4))
}

/// The coordinate epoch: an ISO date or a decimal year.
fn epoch(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    match ctx.text(name)? {
        Some(s) => super::magnetic::parse_date(&s)
            .map(Some)
            .map_err(|e| ToolError::invalid(&format!("/{name}"), e)),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------- generic Helmert

const XYZ_IN: [Field; 3] = [
    qty(
        "x",
        "X",
        "Source ECEF X, like 3657660.66 m",
        QT::Length,
        "m",
    )
    .required()
    .core(),
    qty("y", "Y", "Source ECEF Y, like 255768.55 m", QT::Length, "m")
        .required()
        .core(),
    qty(
        "z",
        "Z",
        "Source ECEF Z, like 5201382.11 m",
        QT::Length,
        "m",
    )
    .required()
    .core(),
];

pub static HELMERT: ToolDef = ToolDef {
    id: "geodesy.datum.helmert",
    title: "Helmert transformation (7 or 14 parameters)",
    summary: "Applies a 7-parameter Helmert transformation, or a 14-parameter time-dependent one with rates, to ECEF coordinates, in the position-vector or coordinate-frame convention you name, forward or exactly reversed.",
    aliases: &["Bursa-Wolf", "7 parameter transformation", "14 parameter transformation", "datum shift parameters"],
    keywords: &["Helmert", "Bursa-Wolf", "datum", "transformation", "position vector", "coordinate frame", "time-dependent", "ECEF"],
    inputs: &[
        XYZ_IN[0],
        XYZ_IN[1],
        XYZ_IN[2],
        qty("tx", "Translation X", "Like 0 m", QT::Length, "m"),
        qty("ty", "Translation Y", "Like 0 m", QT::Length, "m"),
        qty("tz", "Translation Z", "Like 4.5 m", QT::Length, "m"),
        qty("rx", "Rotation X", "Arc-seconds, like 0", QT::Angle, "arcsec"),
        qty("ry", "Rotation Y", "Arc-seconds, like 0", QT::Angle, "arcsec"),
        qty("rz", "Rotation Z", "Arc-seconds, like 0.554", QT::Angle, "arcsec"),
        qty("scale", "Scale difference", "Parts per million, like 0.219", QT::Dimensionless, "ppm"),
        Field::new("convention", "Rotation convention", "position-vector (EPSG 1033, IERS) or coordinate-frame (EPSG 1032); required with rotations", Kind::Choice(&["position-vector", "coordinate-frame"])).core(),
        qty("tx_rate", "Translation X rate", "Per year, like 1.42 mm/yr", QT::Speed, "m/yr"),
        qty("ty_rate", "Translation Y rate", "Per year", QT::Speed, "m/yr"),
        qty("tz_rate", "Translation Z rate", "Per year", QT::Speed, "m/yr"),
        qty("rx_rate", "Rotation X rate", "Arc-seconds per year", QT::AngularRate, "arcsec/yr"),
        qty("ry_rate", "Rotation Y rate", "Arc-seconds per year", QT::AngularRate, "arcsec/yr"),
        qty("rz_rate", "Rotation Z rate", "Arc-seconds per year", QT::AngularRate, "arcsec/yr"),
        qty("scale_rate", "Scale rate", "Per year, like 0.000109 ppm/yr", QT::Frequency, "ppm/yr"),
        Field::new("reference_epoch", "Parameter reference epoch", "Decimal year of the parameters, like 1994.0", Kind::Number { min: 1900.0, max: 2200.0 }),
        Field::new("epoch", "Coordinate epoch", "The coordinates' epoch: a date (2013-11-24) or decimal year (2013.90)", Kind::Text { max_len: 40 }),
        Field::new("direction", "Direction", "forward (default) or reverse (the exact inverse)", Kind::Choice(&["forward", "reverse"])),
    ],
    outputs: &[
        mm_out("x", "X", "Target ECEF X"),
        mm_out("y", "Y", "Target ECEF Y"),
        mm_out("z", "Z", "Target ECEF Z"),
        mm_out("shift", "Shift", "Distance between source and target positions"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "V_T = (1 + dS) · R · V_S + T with the linearized rotation matrix of IOGP GN 7-2; time-dependent parameters p + ṗ (t − t0); reverse by exact inversion",
    accuracy: "Exact for the given parameters in double precision; matches both IOGP GN 7-2 worked examples",
    references: &[IOGP_7_2],
    examples: &[Example {
        id: "primary",
        title: "WGS 72 to WGS 84 (EPSG 1238), position vector",
        input: r#"{"x":3657660.66,"y":255768.55,"z":5201382.11,"tz":4.5,"rz":0.554,"scale":0.219,"convention":"position-vector"}"#,
        source: "IOGP GN 7-2 §4.2.3 example: 3 657 660.78, 255 778.43, 5 201 387.75 m (to the centimeter)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "geodesy.datum.itrf",
        reason: "alternative",
    }],
    sentence: "The transformed position is X {x}, Y {y}, Z {z}, {shift} from the original.",
    limits: &[("batchRows", 10_000)],
    run: run_helmert,
    ..ToolDef::BLANK
};

fn run_helmert(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut v = [0.0; 3];
    for (i, k) in ["x", "y", "z"].into_iter().enumerate() {
        v[i] = read(ctx, k, QT::Length, "m")?.expect("required");
    }
    let get = |ctx: &mut Ctx, k: &str, q: QT, u: &str| -> Result<f64, ToolError> {
        Ok(read(ctx, k, q, u)?.unwrap_or(0.0))
    };
    let p = Params {
        t: [
            get(ctx, "tx", QT::Length, "m")?,
            get(ctx, "ty", QT::Length, "m")?,
            get(ctx, "tz", QT::Length, "m")?,
        ],
        r: [
            get(ctx, "rx", QT::Angle, "arcsec")?,
            get(ctx, "ry", QT::Angle, "arcsec")?,
            get(ctx, "rz", QT::Angle, "arcsec")?,
        ],
        ds: get(ctx, "scale", QT::Dimensionless, "ppm")?,
    };
    let rate = Params {
        t: [
            get(ctx, "tx_rate", QT::Speed, "m/yr")?,
            get(ctx, "ty_rate", QT::Speed, "m/yr")?,
            get(ctx, "tz_rate", QT::Speed, "m/yr")?,
        ],
        r: [
            get(ctx, "rx_rate", QT::AngularRate, "arcsec/yr")?,
            get(ctx, "ry_rate", QT::AngularRate, "arcsec/yr")?,
            get(ctx, "rz_rate", QT::AngularRate, "arcsec/yr")?,
        ],
        ds: get(ctx, "scale_rate", QT::Frequency, "ppm/yr")?,
    };
    let rotates = p.r.iter().chain(&rate.r).any(|r| *r != 0.0);
    let convention = match ctx.choice("convention")? {
        Some("position-vector") => Convention::PositionVector,
        Some(_) => Convention::CoordinateFrame,
        None if rotates => {
            return Err(ToolError::invalid(
                "/convention",
                "Name the rotation convention: position-vector and coordinate-frame parameters differ in the sign of the rotations, so the same numbers give different results.",
            )
            .hint("IERS and EPSG method 1033 use position-vector; EPSG method 1032 and many national sets use coordinate-frame."));
        }
        None => Convention::PositionVector,
    };
    let has_rates = rate != Params::default();
    let (t, t0) = if has_rates {
        let t = epoch(ctx, "epoch")?.ok_or_else(|| {
            ToolError::invalid(
                "/epoch",
                "Rates need the coordinate epoch, like 2013.90 or 2013-11-24.",
            )
        })?;
        let t0 = ctx.number("reference_epoch")?.ok_or_else(|| {
            ToolError::invalid(
                "/reference_epoch",
                "Rates need the parameters' reference epoch, like 1994.0.",
            )
        })?;
        ctx.context.push(("epoch", Json::Num(t)));
        ctx.context.push(("referenceEpoch", Json::Num(t0)));
        (t, t0)
    } else {
        (0.0, 0.0)
    };
    let h = Helmert {
        p,
        rate,
        t0,
        convention,
    };
    let out = if ctx.choice("direction")? == Some("reverse") {
        h.reverse(v, t)
    } else {
        h.forward(v, t)
    };
    let shift =
        ((out[0] - v[0]).powi(2) + (out[1] - v[1]).powi(2) + (out[2] - v[2]).powi(2)).sqrt();
    Ok(Json::obj([
        ("x", ctx.out("x", m(out[0]))),
        ("y", ctx.out("y", m(out[1]))),
        ("z", ctx.out("z", m(out[2]))),
        ("shift", ctx.out("shift", m(shift))),
    ]))
}

// ---------------------------------------------------------------- ITRF and WGS 84 realizations

/// WGS 84 realizations and the ITRF each was aligned with (NGA).
const WGS84: &[(&str, &str)] = &[
    ("WGS84(G2296)", "ITRF2020"),
    ("WGS84(G2139)", "ITRF2014"),
    ("WGS84(G1762)", "ITRF2008"),
];

const FRAMES: &[&str] = &[
    "ITRF2020",
    "ITRF2014",
    "ITRF2008",
    "ITRF2005",
    "ITRF2000",
    "ITRF97",
    "ITRF96",
    "ITRF94",
    "ITRF93",
    "ITRF92",
    "ITRF91",
    "ITRF90",
    "ITRF89",
    "ITRF88",
    "WGS84",
    "WGS84(G2296)",
    "WGS84(G2139)",
    "WGS84(G1762)",
];

pub static ITRF: ToolDef = ToolDef {
    id: "geodesy.datum.itrf",
    title: "Transform between ITRF and WGS 84 realizations",
    summary: "Transforms a position between International Terrestrial Reference Frame realizations (ITRF2020 back to ITRF88) and the WGS 84 realizations aligned with them, at the coordinates' epoch, with the IERS parameters.",
    aliases: &[
        "ITRF2020 to ITRF2014",
        "ITRF converter",
        "WGS 84 realization",
        "reference frame transformation",
    ],
    keywords: &[
        "ITRF",
        "ITRF2020",
        "ITRF2014",
        "WGS 84",
        "G2296",
        "G2139",
        "reference frame",
        "epoch",
        "Helmert",
    ],
    inputs: &[
        Field::new(
            "from",
            "From frame",
            "Like ITRF2020 or WGS84(G2139)",
            Kind::Choice(FRAMES),
        )
        .required()
        .core(),
        Field::new("to", "To frame", "Like ITRF2014", Kind::Choice(FRAMES))
            .required()
            .core(),
        Field::new(
            "epoch",
            "Coordinate epoch",
            "The date the coordinates refer to: 2026-09-19 or 2026.72",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "Decimal degrees, like 40.446111",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]")
        .core(),
        Field::new(
            "lon",
            "Longitude",
            "Decimal degrees, like -79.982222",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-180,180)")
        .core(),
        qty(
            "height",
            "Ellipsoidal height",
            "Height above the GRS 80 ellipsoid, like 300 m",
            QT::Length,
            "m",
        ),
        qty(
            "x",
            "X",
            "Or give ECEF X instead of latitude and longitude",
            QT::Length,
            "m",
        ),
        qty("y", "Y", "ECEF Y", QT::Length, "m"),
        qty("z", "Z", "ECEF Z", QT::Length, "m"),
    ],
    outputs: &[
        mm_out(
            "shift",
            "Shift",
            "3D distance between the positions in the two frames",
        ),
        mm_out("east", "East shift", "In the local east-north-up frame"),
        mm_out("north", "North shift", "In the local east-north-up frame"),
        mm_out("up", "Up shift", "In the local east-north-up frame"),
        Field::new(
            "lat",
            "Latitude",
            "In the target frame (GRS 80)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(10))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "In the target frame (GRS 80)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(10))
        .angle_range("[-180,180)"),
        mm_out("height", "Ellipsoidal height", "In the target frame"),
        mm_out("x", "X", "Target ECEF X"),
        mm_out("y", "Y", "Target ECEF Y"),
        mm_out("z", "Z", "Target ECEF Z"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "REALIZATION_ASSUMED",
        "INPUT_NORMALIZED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "IERS ITRF2020 → past-ITRF Helmert parameters (epoch 2015.0, with rates), chained through ITRF2020; WGS 84 realizations taken as coincident with the ITRF each was aligned with",
    accuracy: "The IERS parameters; a few millimeters between ITRF2020, ITRF2014, and ITRF2008, and up to centimeters for older realizations",
    references: &[IERS_ITRF2020, NGA_WGS84],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh from ITRF2020 to ITRF2014 in 2026",
        input: r#"{"from":"ITRF2020","to":"ITRF2014","epoch":"2026.72","lat":40.446111,"lon":-79.982222,"height":300}"#,
        source: "PROJ +proj=helmert with its ITRF2020 parameter file",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.helmert",
            reason: "alternative",
        },
        Related {
            id: "geodesy.frame.to-local",
            reason: "next",
        },
    ],
    sentence: "The position moves {shift} between the frames: {east} east, {north} north, {up} up.",
    limits: &[("batchRows", 10_000)],
    run: run_itrf,
    ..ToolDef::BLANK
};

/// The ITRF a frame name resolves to, with the WGS 84 realization it came from.
fn resolve(ctx: &mut Ctx, name: &'static str, field: &str) -> &'static str {
    let name = if name == "WGS84" {
        ctx.warnings.push(Warning::new("REALIZATION_ASSUMED", format!("\"WGS 84\" names several realizations; {field} was taken as the current one, WGS 84 (G2296).")));
        "WGS84(G2296)"
    } else {
        name
    };
    WGS84
        .iter()
        .find(|(w, _)| *w == name)
        .map_or(name, |(_, itrf)| itrf)
}

fn run_itrf(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let from_name = ctx.choice("from")?.expect("required");
    let to_name = ctx.choice("to")?.expect("required");
    let t = epoch(ctx, "epoch")?.expect("required");
    if !(1980.0..=2100.0).contains(&t) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The epoch must be between 1980 and 2100.",
        )
        .at("/epoch"));
    }
    let from = resolve(ctx, from_name, "the source frame");
    let to = resolve(ctx, to_name, "the target frame");
    let grs80 = CATALOG
        .iter()
        .find(|e| e.id == "grs80")
        .copied()
        .unwrap_or(Ellipsoid {
            id: "grs80",
            name: "GRS 1980",
            a: 6_378_137.0,
            f: 1.0 / 298.257_222_101,
        });
    let by_xyz = ctx.is_set("x") || ctx.is_set("y") || ctx.is_set("z");
    let v = if by_xyz {
        let mut v = [0.0; 3];
        for (i, k) in ["x", "y", "z"].into_iter().enumerate() {
            v[i] = read(ctx, k, QT::Length, "m")?.ok_or_else(|| {
                ToolError::invalid(
                    &format!("/{k}"),
                    "Give all of x, y, and z, or latitude and longitude.",
                )
            })?;
        }
        v
    } else {
        if !ctx.is_set("lat") || !ctx.is_set("lon") {
            return Err(ToolError::invalid(
                "/lat",
                "Give latitude and longitude (and height), or ECEF x, y, and z.",
            ));
        }
        let (lat, lon) = point::read(ctx, "lat", "lon")?;
        let h = read(ctx, "height", QT::Length, "m")?.unwrap_or(0.0);
        let p = fr::to_ecef(&grs80, lat.to_radians(), lon.to_radians(), h);
        [p.0, p.1, p.2]
    };
    let out = helmert::itrf_transform(from, to, v, t).expect("frames from the choice list");
    ctx.context.push(("epoch", Json::Num(t)));
    ctx.context.push(("from", Json::str(from)));
    ctx.context.push(("to", Json::str(to)));
    if from == to && from_name != to_name {
        ctx.accuracy = Some(format!(
            "No change: {} and {} coincide by the alignment of the WGS 84 realization with its ITRF, which NGA states at the few-centimeter level.",
            from_name.replace("WGS84", "WGS 84 "),
            to_name.replace("WGS84", "WGS 84 ")
        ));
    }
    let (phi, lam, h) = fr::from_ecef(&grs80, out[0], out[1], out[2]).ok_or_else(|| {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "The position is at the Earth's center.",
        )
    })?;
    let enu = fr::ecef_to_enu((v[0], v[1], v[2]), phi, lam, (out[0], out[1], out[2]));
    let shift = enu.iter().map(|d| d * d).sum::<f64>().sqrt();
    Ok(Json::obj([
        ("shift", ctx.out("shift", m(shift))),
        ("east", ctx.out("east", m(enu[0]))),
        ("north", ctx.out("north", m(enu[1]))),
        ("up", ctx.out("up", m(enu[2]))),
        ("lat", ctx.out("lat", deg(phi.to_degrees()))),
        (
            "lon",
            ctx.out("lon", deg(gp_base::angle::wrap_lon(lam.to_degrees()))),
        ),
        ("height", ctx.out("height", m(h))),
        ("x", ctx.out("x", m(out[0]))),
        ("y", ctx.out("y", m(out[1]))),
        ("z", ctx.out("z", m(out[2]))),
    ]))
}

// ---------------------------------------------------------------- plate motion

const ITRF2020_PMM: Reference = Reference {
    title: "ITRF2020 Plate Motion Model, Geophysical Research Letters 50",
    issuer: "Altamimi, Z., Métivier, L., Rebischung, P., Collilieux, X., Chanard, K., and Barnéoud, J., American Geophysical Union",
    year: 2023,
    edition: "e2023GL106373",
    locator: "Table 1 (plate rotation poles) and Table 2 (origin rate bias)",
    url: "https://doi.org/10.1029/2023GL106373",
};
const PB2002: Reference = Reference {
    title: "An updated digital model of plate boundaries, Geochemistry, Geophysics, Geosystems 4(3)",
    issuer: "Bird, P., American Geophysical Union",
    year: 2003,
    edition: "PB2002, 1027 (orogens as GeoJSON by Ahlenius, ODC-BY 1.0)",
    locator: "PB2002_orogens: the 13 zones of distributed deformation",
    url: "https://doi.org/10.1029/2001GC000252",
};

const PLATE_CODES: &[&str] = &[
    "AMUR", "ANTA", "ARAB", "AUST", "CARB", "EURA", "INDI", "NAZC", "NOAM", "NUBI", "PCFC", "SOAM",
    "SOMA",
];

pub static PLATE_MOTION: ToolDef = ToolDef {
    id: "geodesy.datum.plate-motion",
    title: "Move a position between epochs (ITRF2020 plate motion)",
    summary: "Propagates an ITRF2020 position from one epoch to another with the rigid-plate velocity of its tectonic plate, or with a site velocity you give, and flags zones where plates deform and rigid motion does not apply.",
    aliases: &[
        "epoch propagation",
        "plate motion model",
        "tectonic plate velocity",
        "coordinate epoch update",
    ],
    keywords: &[
        "plate motion",
        "ITRF2020",
        "epoch",
        "velocity",
        "tectonic",
        "propagation",
        "PMM",
        "site velocity",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "Decimal degrees, like 39",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "Decimal degrees, like -98",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
        qty(
            "height",
            "Ellipsoidal height",
            "Height above the GRS 80 ellipsoid, like 300 m",
            QT::Length,
            "m",
        ),
        Field::new(
            "from_epoch",
            "From epoch",
            "Epoch of the coordinates: 2010-01-01 or 2010.0",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "to_epoch",
            "To epoch",
            "Epoch wanted: 2026-09-19 or 2026.72",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "plate",
            "Plate",
            "NOAM (North American), PCFC (Pacific), EURA, AUST, SOAM, NUBI, and others; required unless you give a site velocity",
            Kind::Choice(PLATE_CODES),
        ),
        Field::new(
            "origin_rate",
            "Origin rate bias",
            "yes (default) adds the ITRF2020 origin rate bias to the plate velocity; no leaves it out",
            Kind::Choice(&["yes", "no"]),
        ),
        qty(
            "v_east",
            "Site velocity east",
            "Overrides the model, like 12.3 mm/yr",
            QT::Speed,
            "mm/yr",
        ),
        qty(
            "v_north",
            "Site velocity north",
            "Like -4.1 mm/yr",
            QT::Speed,
            "mm/yr",
        ),
        qty(
            "v_up",
            "Site velocity up",
            "Like 0.8 mm/yr",
            QT::Speed,
            "mm/yr",
        ),
    ],
    outputs: &[
        mm_out(
            "displacement",
            "Horizontal displacement",
            "Between the two epochs",
        ),
        mm_out("east", "East displacement", "Between the two epochs"),
        mm_out("north", "North displacement", "Between the two epochs"),
        mm_out("up", "Up displacement", "Between the two epochs"),
        qty(
            "v_east",
            "Velocity east",
            "The velocity used",
            QT::Speed,
            "mm/yr",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "v_north",
            "Velocity north",
            "The velocity used",
            QT::Speed,
            "mm/yr",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "v_up",
            "Velocity up",
            "The velocity used",
            QT::Speed,
            "mm/yr",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "lat",
            "Latitude",
            "At the new epoch",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(10))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "At the new epoch",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(10))
        .angle_range("[-180,180)"),
        mm_out("height", "Ellipsoidal height", "At the new epoch"),
        mm_out("x", "X", "ECEF at the new epoch"),
        mm_out("y", "Y", "ECEF at the new epoch"),
        mm_out("z", "Z", "ECEF at the new epoch"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["DEFORMATION_ZONE", "INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "ITRF2020 plate motion model: v = ω × X plus the origin rate bias, X(t2) = X(t1) + v (t2 − t1); a site velocity replaces the model",
    accuracy: "About 0.2 mm/yr on stable plate interiors (the model's fit); rigid-plate velocities can be wrong by centimeters per year in deforming zones",
    references: &[ITRF2020_PMM, PB2002],
    examples: &[Example {
        id: "primary",
        title: "Kansas on the North American plate, 2010 to 2026.7",
        input: r#"{"lat":38.5,"lon":-98,"height":500,"from_epoch":"2010.0","to_epoch":"2026.7","plate":"NOAM"}"#,
        source: "PROJ +proj=helmert with the NOAM_T rates of its data/ITRF2020",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[Related {
        id: "geodesy.datum.itrf",
        reason: "next",
    }],
    sentence: "The position moves {displacement} horizontally: {east} east and {north} north.{warn DEFORMATION_ZONE} It lies in a deforming zone, so use a site velocity from a nearby station.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_plate_motion,
    ..ToolDef::BLANK
};

fn run_plate_motion(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = read(ctx, "height", QT::Length, "m")?.unwrap_or(0.0);
    let t1 = epoch(ctx, "from_epoch")?.expect("required");
    let t2 = epoch(ctx, "to_epoch")?.expect("required");
    for (t, f) in [(t1, "/from_epoch"), (t2, "/to_epoch")] {
        if !(1980.0..=2100.0).contains(&t) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Epochs must be between 1980 and 2100.",
            )
            .at(f));
        }
    }
    let grs80 = CATALOG
        .iter()
        .find(|e| e.id == "grs80")
        .copied()
        .expect("GRS 80 in the catalog");
    let (phi, lam) = (lat.to_radians(), lon.to_radians());
    let p = fr::to_ecef(&grs80, phi, lam, h);
    let site = ["v_east", "v_north", "v_up"].map(|k| read(ctx, k, QT::Speed, "m/yr"));
    let site: Vec<Option<f64>> = site.into_iter().collect::<Result<_, _>>()?;
    let v_ecef = if site.iter().any(Option::is_some) {
        let enu = [0, 1, 2].map(|i| site[i].unwrap_or(0.0));
        let o = fr::enu_to_ecef((0.0, 0.0, 0.0), phi, lam, enu);
        ctx.model = Some("Site velocity given by the user, X(t2) = X(t1) + v (t2 − t1)".into());
        [o.0, o.1, o.2]
    } else {
        let plate = ctx
            .choice("plate")?
            .ok_or_else(|| ToolError::invalid("/plate", "Name the tectonic plate, or give a site velocity.").hint("Most of the United States and Canada is NOAM; coastal California west of the San Andreas fault is PCFC."))?;
        let orb = ctx.choice("origin_rate")? != Some("no");
        if let Some(zone) = gp_geo::plates::deformation_zone(lat, lon) {
            ctx.warnings.push(Warning::new(
                "DEFORMATION_ZONE",
                format!("The point is in the {zone} deformation zone, where rigid-plate velocities do not apply. Use a site velocity from a nearby CORS station (NGS publishes them)."),
            ));
        }
        ctx.context.push(("plate", Json::str(plate)));
        gp_geo::plates::velocity(plate, [p.0, p.1, p.2], orb).expect("plate from the choice list")
    };
    let dt = t2 - t1;
    let q = (
        p.0 + v_ecef[0] * dt,
        p.1 + v_ecef[1] * dt,
        p.2 + v_ecef[2] * dt,
    );
    let v_enu = fr::ecef_to_enu((0.0, 0.0, 0.0), phi, lam, (v_ecef[0], v_ecef[1], v_ecef[2]));
    let d = v_enu.map(|v| v * dt);
    let (phi2, lam2, h2) = fr::from_ecef(&grs80, q.0, q.1, q.2).expect("not the center");
    ctx.context.push(("fromEpoch", Json::Num(t1)));
    ctx.context.push(("toEpoch", Json::Num(t2)));
    let mmyr = |v: f64| Q {
        value: v * 1000.0,
        unit: units::by_symbol(QT::Speed, "mm/yr").expect("mm/yr"),
    };
    Ok(Json::obj([
        ("displacement", ctx.out("displacement", m(d[0].hypot(d[1])))),
        ("east", ctx.out("east", m(d[0]))),
        ("north", ctx.out("north", m(d[1]))),
        ("up", ctx.out("up", m(d[2]))),
        ("v_east", ctx.out("v_east", mmyr(v_enu[0]))),
        ("v_north", ctx.out("v_north", mmyr(v_enu[1]))),
        ("v_up", ctx.out("v_up", mmyr(v_enu[2]))),
        ("lat", ctx.out("lat", deg(phi2.to_degrees()))),
        (
            "lon",
            ctx.out("lon", deg(gp_base::angle::wrap_lon(lam2.to_degrees()))),
        ),
        ("height", ctx.out("height", m(h2))),
        ("x", ctx.out("x", m(q.0))),
        ("y", ctx.out("y", m(q.1))),
        ("z", ctx.out("z", m(q.2))),
    ]))
}
