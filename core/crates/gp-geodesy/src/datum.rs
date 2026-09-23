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
    stability: gp_base::tool::Stability::Stable,
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
        qty("ty_rate", "Translation Y rate", "Per year, like 0.0001 m/yr", QT::Speed, "m/yr"),
        qty("tz_rate", "Translation Z rate", "Per year, like 0.0001 m/yr", QT::Speed, "m/yr"),
        qty("rx_rate", "Rotation X rate", "Arc-seconds per year, like 0.00008 arcsec/yr", QT::AngularRate, "arcsec/yr"),
        qty("ry_rate", "Rotation Y rate", "Arc-seconds per year, like 0.00008 arcsec/yr", QT::AngularRate, "arcsec/yr"),
        qty("rz_rate", "Rotation Z rate", "Arc-seconds per year, like 0.00008 arcsec/yr", QT::AngularRate, "arcsec/yr"),
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
    warnings: &[],
    when_to_use: "Use this when you have the parameters of a datum transformation and need them applied to geocentric coordinates: a frame tie published by IERS or EPSG, a legacy datum's shift onto WGS 84, or the fourteen-parameter form that carries a position between epochs as the plates move. It runs either way round, and the reverse is the exact inverse rather than the negated parameters.",
    limitations: "A parameter set belongs to one pair of frames, and often to one epoch and one region; applied to any other pair it gives an answer that looks reasonable and is wrong, so the set has to come from the authority for those frames. The convention must travel with the parameters, since position-vector and coordinate-frame differ in the sign of the rotations and the same seven numbers give different results under each. The rotations are linearized, as the EPSG methods define them, which is what published parameters are fitted against. A similarity transformation cannot model the distortion of a legacy survey network, which is why a grid-based transformation exists for the datums that have one.",
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
    related: &[
        Related {
            id: "geodesy.datum.itrf",
            reason: "alternative",
        },
        Related {
            id: "geodesy.frame.geodetic-to-ecef",
            reason: "parent",
        },
        Related {
            id: "geodesy.frame.ecef-to-geodetic",
            reason: "next",
        },
        Related {
            id: "geodesy.datum.plate-motion",
            reason: "alternative",
        },
    ],
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
    stability: gp_base::tool::Stability::Stable,
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
            "Or give ECEF X instead of latitude and longitude, like -1248000 m",
            QT::Length,
            "m",
        ),
        qty("y", "Y", "ECEF Y, like -4819000 m", QT::Length, "m"),
        qty("z", "Z", "ECEF Z, like 3976000 m", QT::Length, "m"),
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
    warnings: &["REALIZATION_ASSUMED", "INPUT_NORMALIZED"],
    model: "IERS ITRF2020 → past-ITRF Helmert parameters (epoch 2015.0, with rates), chained through ITRF2020; WGS 84 realizations taken as coincident with the ITRF each was aligned with",
    accuracy: "The IERS parameters; a few millimeters between ITRF2020, ITRF2014, and ITRF2008, and up to centimeters for older realizations",
    when_to_use: "Use this when two positions are quoted in different realizations of what people loosely call the same system. A GNSS network gives coordinates in a particular ITRF, a receiver may report WGS 84 without saying which realization, and archived survey data carries whichever was current when it was observed; combining them without this step buries a shift of centimeters to decimeters in the result. Give the epoch the coordinates belong to, because the frames move relative to each other over time and the parameters carry rates. It reports the shift in meters and as east, north, and up, so the size of what would otherwise be silent is visible.",
    limitations: "This changes the frame, not the epoch: it does not move a position forward or back in time along its plate's motion, which is the neighbouring plate-motion tool, and the two are usually needed together. Unqualified WGS 84 is treated as the realization aligned with the ITRF of its day and says so in a warning, because a bare WGS 84 label does not identify a realization. The parameters are the published IERS values, so the answer is as good as they are — a few millimeters among ITRF2020, ITRF2014, and ITRF2008, and up to centimeters for the older realizations — and none of that accounts for the accuracy of the coordinates themselves. It is a rigid transformation of the whole Earth and knows nothing about local deformation.",
    references: &[IERS_ITRF2020, NGA_WGS84],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh from ITRF2020 to ITRF2014 in 2026",
        input: r#"{"from":"ITRF2020","to":"ITRF2014","epoch":"2026.72","lat":40.446111,"lon":-79.982222,"height":300}"#,
        source: "PROJ 9.3.0 through pyproj, transforming EPSG:9988 (ITRF2020) to EPSG:7912 (ITRF2014) with PROJ's own EPSG-sourced parameters at epoch 2026.72: latitude 40.44611101524038 and longitude -79.98222202049854 identical in every digit, height 300.0011211372912 m against 300.0011211390832 m, a difference of 1.8 nanometres",
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
        Related {
            id: "geodesy.datum.plate-motion",
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
    stability: gp_base::tool::Stability::Stable,
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
    warnings: &["DEFORMATION_ZONE", "INPUT_NORMALIZED"],
    model: "ITRF2020 plate motion model: v = ω × X plus the origin rate bias, X(t2) = X(t1) + v (t2 − t1); a site velocity replaces the model",
    accuracy: "About 0.2 mm/yr on stable plate interiors (the model's fit); rigid-plate velocities can be wrong by centimeters per year in deforming zones",
    when_to_use: "Use this to bring a position to the epoch you need it at. A coordinate in a modern reference frame is only meaningful with the date attached, because the ground it sits on is moving — 15 mm a year in Kansas, more than 50 on the Pacific plate — so a position observed in 2010 and one observed today are not the same number even for a mark that has not shifted an inch relative to its neighbours. Give the plate, or a site velocity from a nearby continuously operating station, which is better wherever one is available. It is the companion to changing frames: frames and epochs are separate steps and both are usually needed.",
    limitations: "It moves a position in time, not between frames — that is the ITRF or NAD 83 tool — and the two are easy to confuse because both report a shift in meters. The model is rigid-plate rotation, so it is only as good as that assumption: on a stable plate interior it fits to about 0.2 mm a year, but in a deforming zone it can be wrong by centimeters a year, and those zones are flagged from Bird's PB2002 orogens with the advice to use a site velocity instead. It also cannot know about anything episodic — an earthquake, subsidence, a landslide — which moves a mark without warning and leaves this answer confidently wrong. Epochs are accepted between 1980 and 2100; further out the linear velocity is extrapolation rather than model.",
    references: &[ITRF2020_PMM, PB2002],
    examples: &[Example {
        id: "primary",
        title: "Kansas on the North American plate, 2010 to 2026.7",
        input: r#"{"lat":38.5,"lon":-98,"height":500,"from_epoch":"2010.0","to_epoch":"2026.7","plate":"NOAM"}"#,
        source: "PROJ 9.3.0 given the Altamimi 2023 rotation pole and origin rate bias and left to do the cross product itself: it lands within 4 nanometres in latitude, 1.2 in longitude and 2.4 in height, and 14 mm away if the origin rate bias is omitted, so the check distinguishes that term",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.itrf",
            reason: "next",
        },
        Related {
            id: "geodesy.datum.nad83",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.helmert",
            reason: "alternative",
        },
    ],
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

// ---------------------------------------------------------------- NAD 83 (HTDP)

const NGS_HTDP: Reference = Reference {
    title: "Horizontal Time-Dependent Positioning (HTDP) software, version 3.6.0",
    issuer: "National Geodetic Survey, NOAA",
    year: 2025,
    edition: "HTDP 3.6.0 (2025-04-07)",
    locator: "Subroutine SETTP: transformation parameters from ITRF94 to NAD 83 and the ITRF and WGS 84 realizations",
    url: "https://geodesy.noaa.gov/TOOLS/Htdp/Htdp.shtml",
};

const NAD83_FRAMES: &[&str] = &[
    "NAD83(2011)",
    "NAD83(PA11)",
    "NAD83(MA11)",
    "ITRF2020",
    "ITRF2014",
    "ITRF2008",
    "ITRF2005",
    "ITRF2000",
    "WGS84",
    "WGS84(G2296)",
    "WGS84(G2139)",
    "WGS84(G1762)",
    "WGS84(G1674)",
    "WGS84(G1150)",
];

pub static NAD83: ToolDef = ToolDef {
    id: "geodesy.datum.nad83",
    stability: gp_base::tool::Stability::Stable,
    title: "Transform between NAD 83 and ITRF or WGS 84",
    summary: "Transforms a position between NAD 83 (2011, PA11, or MA11) and ITRF2020, ITRF2014, ITRF2008, ITRF2005, ITRF2000, or WGS 84 at one epoch, with the NGS HTDP parameters, and shows the meter-level difference between them.",
    aliases: &[
        "NAD83 to WGS84",
        "WGS84 to NAD83",
        "NAD83(2011) to ITRF2020",
        "HTDP transformation",
    ],
    keywords: &[
        "NAD 83",
        "NAD83(2011)",
        "HTDP",
        "ITRF",
        "WGS 84",
        "datum",
        "PA11",
        "MA11",
        "NGS",
    ],
    inputs: &[
        Field::new(
            "from",
            "From frame",
            "Like WGS84(G2296) or NAD83(2011)",
            Kind::Choice(NAD83_FRAMES),
        )
        .required()
        .core(),
        Field::new(
            "to",
            "To frame",
            "Like NAD83(2011)",
            Kind::Choice(NAD83_FRAMES),
        )
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
            "Decimal degrees, like 38.5",
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
            "Height above the GRS 80 ellipsoid, like 500 m",
            QT::Length,
            "m",
        ),
    ],
    outputs: &[
        mm_out(
            "shift",
            "Horizontal shift",
            "Between the positions in the two frames",
        ),
        mm_out("east", "East shift", "In the local east-north-up frame"),
        mm_out("north", "North shift", "In the local east-north-up frame"),
        mm_out("up", "Up shift", "In the local east-north-up frame"),
        Field::new(
            "azimuth",
            "Shift direction",
            "Degrees from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        Field::new(
            "lat",
            "Latitude",
            "In the target frame",
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
            "In the target frame",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(10))
        .angle_range("[-180,180)"),
        mm_out("height", "Ellipsoidal height", "In the target frame"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "INPUT_NORMALIZED"],
    model: "NGS HTDP 3.6.0 14-parameter transformations through ITRF94, applied as HTDP does; WGS 84 (G2296, G2139, G1762) taken as ITRF2020, ITRF2014, and ITRF2008",
    accuracy: "Matches NGS HTDP 3.6.0 within 1 mm; the transformation itself is good to about 1–2 cm in the conterminous United States",
    when_to_use: "Use this whenever survey or mapping data in NAD 83 has to meet a GNSS position. Published US control, state plane coordinates, parcel data, and almost every official map are on NAD 83, while a receiver gives ITRF or WGS 84; the two differ by more than a meter in the conterminous United States, which is far too much to ignore and small enough to go unnoticed. Give the epoch the coordinates belong to. The answer includes the difference as a distance and an azimuth, so the size and direction of the discrepancy are visible rather than buried in the new numbers.",
    limitations: "The published transformation is itself good to about one to two centimeters in the conterminous United States, so the result cannot be better than that however many digits it carries; the arithmetic matches HTDP to a millimeter, which is a different claim. It changes the frame, not the epoch, and does not move a position along its plate's motion — that is the plate-motion tool, and the two are usually wanted together. NAD 83 (2011), (PA11) and (MA11) cover different plates and are not interchangeable: using the wrong one puts the answer out by meters. Unqualified WGS 84 is taken as the realization aligned with the ITRF of its day and says so, because a bare WGS 84 label does not identify a realization. Heights are ellipsoidal throughout, not orthometric, and nothing here touches a vertical datum.",
    references: &[NGS_HTDP],
    examples: &[Example {
        id: "primary",
        title: "Kansas: WGS 84 (G2296) to NAD 83 (2011) in 2026.7",
        input: r#"{"from":"WGS84(G2296)","to":"NAD83(2011)","epoch":"2026.7","lat":38.5,"lon":-98,"height":500}"#,
        source: "NGS HTDP 3.6.0 compiled from htdp.f, menu option 4, and independently PROJ 9.3.0 through pyproj on EPSG:9988 to EPSG:6319, which agrees with the tool to 9.5 nanometres in latitude, 1.2 in longitude and 0.2 in height",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.itrf",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.plate-motion",
            reason: "next",
        },
        Related {
            id: "geodesy.datum.nadcon5",
            reason: "alternative",
        },
    ],
    sentence: "The frames differ here by {shift} horizontally, toward {azimuth}.",
    limits: &[("batchRows", 10_000)],
    run: run_nad83,
    ..ToolDef::BLANK
};

fn run_nad83(ctx: &mut Ctx) -> Result<Json, ToolError> {
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
    // WGS 84 realizations aligned with an ITRF use that ITRF's parameters.
    let from = resolve(ctx, from_name, "the source frame");
    let to = resolve(ctx, to_name, "the target frame");
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = read(ctx, "height", QT::Length, "m")?.unwrap_or(0.0);
    let grs80 = CATALOG
        .iter()
        .find(|e| e.id == "grs80")
        .copied()
        .expect("GRS 80");
    let (phi, lam) = (lat.to_radians(), lon.to_radians());
    let p = fr::to_ecef(&grs80, phi, lam, h);
    let q =
        gp_geo::htdp::transform(from, to, [p.0, p.1, p.2], t).expect("frames from the choice list");
    let enu = fr::ecef_to_enu(p, phi, lam, (q[0], q[1], q[2]));
    let (phi2, lam2, h2) = fr::from_ecef(&grs80, q[0], q[1], q[2]).expect("not the center");
    ctx.context.push(("epoch", Json::Num(t)));
    if from.starts_with("NAD83") || to.starts_with("NAD83") {
        ctx.context.push(("nad83ReferenceEpoch", Json::Num(2010.0)));
    }
    let az = gp_base::angle::wrap_azimuth(enu[0].atan2(enu[1]).to_degrees());
    Ok(Json::obj([
        ("shift", ctx.out("shift", m(enu[0].hypot(enu[1])))),
        ("east", ctx.out("east", m(enu[0]))),
        ("north", ctx.out("north", m(enu[1]))),
        ("up", ctx.out("up", m(enu[2]))),
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("lat", ctx.out("lat", deg(phi2.to_degrees()))),
        (
            "lon",
            ctx.out("lon", deg(gp_base::angle::wrap_lon(lam2.to_degrees()))),
        ),
        ("height", ctx.out("height", m(h2))),
    ]))
}

// ---------------------------------------------------------------- legacy datums

const EPSG_DATASET: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset",
    issuer: "IOGP Geomatics Committee",
    year: 2025,
    edition: "v10.094, as shipped in PROJ 9.3.0",
    locator: "Transformations 1108, 1122, 1133, 1173, 1267, 1305, 1314, 1864 to WGS 84",
    url: "https://epsg.org/search/by-name",
};

/// A published transformation from a legacy datum to WGS 84.
struct Legacy {
    id: &'static str,
    name: &'static str,
    epsg: u32,
    /// Semi-major axis and inverse flattening of the datum's ellipsoid.
    a: f64,
    rf: f64,
    params: Params,
    convention: Convention,
    /// Stated accuracy (m).
    accuracy: f64,
    /// Area of use: west, south, east, north (degrees; east < west crosses 180°).
    area: [f64; 4],
    area_name: &'static str,
}

const fn t3(tx: f64, ty: f64, tz: f64) -> Params {
    Params {
        t: [tx, ty, tz],
        r: [0.0; 3],
        ds: 0.0,
    }
}

const LEGACY: &[Legacy] = &[
    Legacy {
        id: "ED50",
        name: "ED50",
        epsg: 1133,
        a: 6_378_388.0,
        rf: 297.0,
        params: t3(-87.0, -98.0, -121.0),
        convention: Convention::PositionVector,
        accuracy: 10.0,
        area: [-9.56, 34.88, 31.59, 71.24],
        area_name: "western Europe",
    },
    Legacy {
        id: "NAD27",
        name: "NAD27",
        epsg: 1173,
        a: 6_378_206.4,
        rf: 294.978_698_213_898_2,
        params: t3(-8.0, 160.0, 176.0),
        convention: Convention::PositionVector,
        accuracy: 10.0,
        area: [-124.79, 24.41, -66.91, 49.38],
        area_name: "the conterminous United States",
    },
    Legacy {
        id: "OSGB36",
        name: "OSGB36",
        epsg: 1314,
        a: 6_377_563.396,
        rf: 299.324_964_6,
        params: Params {
            t: [446.448, -125.157, 542.06],
            r: [0.15, 0.247, 0.842],
            ds: -20.489,
        },
        convention: Convention::PositionVector,
        accuracy: 2.0,
        area: [-8.82, 49.79, 1.92, 60.94],
        area_name: "Great Britain",
    },
    Legacy {
        id: "Tokyo",
        name: "Tokyo (South Korea)",
        epsg: 1305,
        a: 6_377_397.155,
        rf: 299.152_812_8,
        params: t3(-147.0, 506.0, 687.0),
        convention: Convention::PositionVector,
        accuracy: 4.0,
        area: [124.53, 33.14, 131.01, 38.64],
        area_name: "South Korea",
    },
    Legacy {
        id: "AGD66",
        name: "AGD66",
        epsg: 1108,
        a: 6_378_160.0,
        rf: 298.25,
        params: t3(-133.0, -48.0, 148.0),
        convention: Convention::PositionVector,
        accuracy: 6.0,
        area: [112.85, -43.7, 153.69, -9.86],
        area_name: "Australia",
    },
    Legacy {
        id: "Pulkovo1942",
        name: "Pulkovo 1942",
        epsg: 1267,
        a: 6_378_245.0,
        rf: 298.3,
        params: Params {
            t: [23.92, -141.27, -80.9],
            r: [0.0, -0.35, -0.82],
            ds: -0.12,
        },
        convention: Convention::CoordinateFrame,
        accuracy: 4.0,
        area: [19.58, 41.19, -168.97, 81.91],
        area_name: "Russia",
    },
    Legacy {
        id: "SAD69",
        name: "SAD69",
        epsg: 1864,
        a: 6_378_160.0,
        rf: 298.25,
        params: t3(-57.0, 1.0, -41.0),
        convention: Convention::PositionVector,
        accuracy: 19.0,
        area: [-81.41, -45.0, -34.74, 12.52],
        area_name: "South America north of 45° S",
    },
    Legacy {
        id: "Arc1960",
        name: "Arc 1960",
        epsg: 1122,
        a: 6_378_249.145,
        rf: 293.465,
        params: t3(-160.0, -6.0, -302.0),
        convention: Convention::PositionVector,
        accuracy: 35.0,
        area: [29.34, -11.75, 41.91, 4.63],
        area_name: "Kenya and Tanzania",
    },
];

const LEGACY_IDS: &[&str] = &[
    "ED50",
    "NAD27",
    "OSGB36",
    "Tokyo",
    "AGD66",
    "Pulkovo1942",
    "SAD69",
    "Arc1960",
];

pub static LEGACY_SHIFT: ToolDef = ToolDef {
    id: "geodesy.datum.legacy",
    stability: gp_base::tool::Stability::Stable,
    title: "Legacy datum to or from WGS 84",
    summary: "Converts a latitude and longitude between a legacy datum (ED50, NAD27, OSGB36, Tokyo, AGD66, Pulkovo 1942, SAD69, or Arc 1960) and WGS 84 with the published EPSG transformation, and states its accuracy, which is meters, not centimeters.",
    aliases: &[
        "ED50 to WGS84",
        "NAD27 to WGS84",
        "OSGB36 to WGS84",
        "old datum conversion",
        "Molodensky shift",
    ],
    keywords: &[
        "datum", "ED50", "NAD27", "OSGB36", "Pulkovo", "Tokyo", "AGD66", "SAD69", "Arc 1960",
        "WGS 84", "EPSG", "shift",
    ],
    inputs: &[
        Field::new(
            "datum",
            "Legacy datum",
            "ED50, NAD27, OSGB36, Tokyo, AGD66, Pulkovo1942, SAD69, or Arc1960",
            Kind::Choice(LEGACY_IDS),
        )
        .required()
        .core(),
        Field::new(
            "direction",
            "Direction",
            "to-wgs84 (default) or from-wgs84",
            Kind::Choice(&["to-wgs84", "from-wgs84"]),
        )
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "Decimal degrees on the source datum, like 48.85",
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
            "Decimal degrees on the source datum, like 2.35",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On the target datum",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "On the target datum",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7))
        .angle_range("[-180,180)"),
        mm_out(
            "shift",
            "Horizontal shift",
            "Between the two datums' coordinates of the point",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "azimuth",
            "Shift direction",
            "Degrees from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(0))
        .angle_range("[0,360)"),
        mm_out(
            "accuracy",
            "Stated accuracy",
            "The EPSG accuracy of this transformation",
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[
        "LOW_ACCURACY_TRANSFORM",
        "OUTSIDE_AREA_OF_USE",
        "INPUT_NORMALIZED",
    ],
    model: "EPSG geocentric translation or 7-parameter Helmert (geog2D domain): geographic → ECEF on the datum's ellipsoid → Helmert → WGS 84",
    accuracy: "As stated by EPSG for each transformation: 2 m (OSGB36) to 35 m (Arc 1960); matches PROJ's implementation of the same EPSG operation to 1e-9°",
    when_to_use: "Use this to place an old map, chart, or survey record against a modern position. Coordinates printed before satellite positioning are on a datum fitted to one region — ED50 across western Europe, NAD27 in North America, Tokyo in Japan and Korea, Arc 1960 in east Africa — and reading them as WGS 84 puts a point tens to hundreds of meters from where it belongs, far enough to move a boundary onto the wrong side of a road. The answer reports how far the point moves and in which direction, so the size of the correction is visible, along with what the transformation is actually worth.",
    limitations: "This is meters of accuracy, not centimeters, and the stated figure is the point: from 2 m for OSGB36 to 35 m for Arc 1960. A single set of parameters is being asked to stand for a whole continental datum that was never that consistent, so the error varies across the region and no single number describes any particular point. Each transformation has a published area of use and the answer is flagged outside it, where it is extrapolation rather than transformation. Where a national grid exists — the NADCON5 grids for North America, OSTN15 for Great Britain — that grid is more accurate than this and should be preferred; the neighbouring NADCON5 tool covers the American cases. Only latitude and longitude are converted; heights are untouched, and nothing here concerns a vertical datum.",
    references: &[EPSG_DATASET, IOGP_7_2],
    examples: &[Example {
        id: "primary",
        title: "Paris on ED50 to WGS 84",
        input: r#"{"datum":"ED50","lat":48.8566,"lon":2.3522}"#,
        source: "PROJ 9.3.0 asked for EPSG:1133 (ED50 to WGS 84 (1)) by code alone, through pyproj, returns 48.85568546266484 and 2.350914333016232 — identical to the tool in every digit a double carries",
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
            id: "geodesy.datum.nadcon5",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.itrf",
            reason: "next",
        },
    ],
    sentence: "The point shifts {shift} toward {azimuth}.{warn LOW_ACCURACY_TRANSFORM} This shift is good only to about {accuracy}.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_legacy,
    ..ToolDef::BLANK
};

fn in_area(area: [f64; 4], lat: f64, lon: f64) -> bool {
    let [w, s, e, n] = area;
    let lon_ok = if w <= e {
        (w..=e).contains(&lon)
    } else {
        lon >= w || lon <= e
    };
    lon_ok && (s..=n).contains(&lat)
}

fn run_legacy(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let id = ctx.choice("datum")?.expect("required");
    let d = LEGACY
        .iter()
        .find(|d| d.id == id)
        .expect("choice lists the table");
    let to_wgs = ctx.choice("direction")? != Some("from-wgs84");
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let legacy = Ellipsoid {
        id: "legacy",
        name: d.name,
        a: d.a,
        f: 1.0 / d.rf,
    };
    let wgs = CATALOG[0];
    let h = Helmert {
        p: d.params,
        rate: Params::default(),
        t0: 0.0,
        convention: d.convention,
    };
    let (src, dst) = if to_wgs {
        (&legacy, &wgs)
    } else {
        (&wgs, &legacy)
    };
    let (phi, lam) = (lat.to_radians(), lon.to_radians());
    let p = fr::to_ecef(src, phi, lam, 0.0);
    let q = if to_wgs {
        h.forward([p.0, p.1, p.2], 0.0)
    } else {
        h.reverse([p.0, p.1, p.2], 0.0)
    };
    let (phi2, lam2, _) = fr::from_ecef(dst, q[0], q[1], q[2]).expect("not the center");
    let (lat2, lon2) = (
        phi2.to_degrees(),
        gp_base::angle::wrap_lon(lam2.to_degrees()),
    );
    // The horizontal shift on the ground: the east-north offset between the two
    // coordinates on WGS 84 (under 1 km, the chord equals the arc to 1e-7 m).
    let a = fr::to_ecef(&wgs, phi, lam, 0.0);
    let b = fr::to_ecef(&wgs, phi2, lam2, 0.0);
    let enu = fr::ecef_to_enu(a, phi, lam, b);
    let s12 = enu[0].hypot(enu[1]);
    let az = if s12 == 0.0 {
        0.0
    } else {
        gp_base::angle::wrap_azimuth(enu[0].atan2(enu[1]).to_degrees())
    };
    ctx.accuracy = Some(format!(
        "EPSG {} ({} to WGS 84): about {} m, for {}.",
        d.epsg, d.name, d.accuracy, d.area_name
    ));
    ctx.context.push(("epsg", Json::Num(f64::from(d.epsg))));
    if d.accuracy >= 1.0 {
        ctx.warnings.push(Warning::new("LOW_ACCURACY_TRANSFORM", format!("{} to WGS 84 by EPSG {} is good to about {} m, not the centimeters of a modern frame.", d.name, d.epsg, d.accuracy)));
    }
    if !in_area(d.area, lat, lon) {
        ctx.warnings.push(Warning::new(
            "OUTSIDE_AREA_OF_USE",
            format!(
                "EPSG {} was derived for {}; outside it the error can be far larger than {} m.",
                d.epsg, d.area_name, d.accuracy
            ),
        ));
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat2))),
        ("lon", ctx.out("lon", deg(lon2))),
        ("shift", ctx.out("shift", m(s12))),
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("accuracy", ctx.out("accuracy", m(d.accuracy))),
    ]))
}

// ---------------------------------------------------------------- NADCON5

const NADCON5_REF: Reference = Reference {
    title: "NADCON 5.0: Geometric Transformation Tool for Points in the National Spatial Reference System, NOAA Technical Report NOS NGS 63",
    issuer: "Smith, D. A., and Bilich, A. L., National Geodetic Survey",
    year: 2017,
    edition: "NOAA TR NOS NGS 63 (revised August 31, 2020); grids release 20160901",
    locator: "The transformation grids by region and how they are applied",
    url: "https://geodesy.noaa.gov/library/pdfs/NOAA_TR_NOS_NGS_0063.pdf",
};
/// The interpolation NADCON5 applies between grid nodes (NGS's `qterp`).
const BIQUADRATIC_REF: Reference = Reference {
    title: "Biquadratic Interpolation, NOAA Technical Memorandum NOS NGS 84",
    issuer: "Smith, D. A., National Geodetic Survey",
    year: 2022,
    edition: "NOAA TM NOS NGS 84 (January 25, 2022)",
    locator: "Section 2: quadratic interpolation over the nearest 3 × 3 grid nodes",
    url: "https://geodesy.noaa.gov/library/pdfs/NOAA_TM_NOS_NGS_0084.pdf",
};
pub const NADCON5_ID: &str = "nadcon5";
pub const NADCON5_VERSION: &str = "20160901";

/// A NADCON5 first-step grid: region, old datum, target, file, and bounds
/// (west, south, east, north, with east beyond 180 when it crosses it).
struct Nc5Region {
    id: &'static str,
    name: &'static str,
    datum: &'static str,
    target: &'static str,
    file: &'static str,
    bounds: [f64; 4],
}

/// Checked in this order, so the small islands win over the big grids around them.
const NC5_REGIONS: &[Nc5Region] = &[
    Nc5Region {
        id: "stpaul",
        name: "St. Paul Island",
        datum: "St. Paul 1952",
        target: "NAD 83 (1986)",
        file: "sp1952_nad83_1986_stpaul.grid",
        bounds: [-170.7, 56.9, -169.6, 57.4],
    },
    Nc5Region {
        id: "hawaii",
        name: "Hawaii",
        datum: "Old Hawaiian",
        target: "NAD 83 (1986)",
        file: "ohd_nad83_1986_hawaii.grid",
        bounds: [-161.0, 18.0, -154.0, 23.0],
    },
    Nc5Region {
        id: "prvi",
        name: "Puerto Rico and the Virgin Islands",
        datum: "Puerto Rico 1940",
        target: "NAD 83 (1986)",
        file: "pr40_nad83_1986_prvi.grid",
        bounds: [-69.0, 17.0, -64.0, 19.0],
    },
    Nc5Region {
        id: "samoa",
        name: "American Samoa",
        datum: "American Samoa 1962",
        target: "NAD 83 (1993)",
        file: "as62_nad83_1993_as.grid",
        bounds: [-172.0, -16.0, -167.0, -13.0],
    },
    Nc5Region {
        id: "guam",
        name: "Guam and the Northern Mariana Islands",
        datum: "Guam 1963",
        target: "NAD 83 (1993)",
        file: "gu63_nad83_1993_guamcnmi.grid",
        bounds: [143.0, 12.0, 147.0, 22.0],
    },
    Nc5Region {
        id: "conus",
        name: "the conterminous United States",
        datum: "NAD 27",
        target: "NAD 83 (1986)",
        file: "nad27_nad83_1986_conus.grid",
        bounds: [-125.0, 24.0, -66.0, 50.0],
    },
    Nc5Region {
        id: "alaska",
        name: "Alaska",
        datum: "NAD 27",
        target: "NAD 83 (1986)",
        file: "nad27_nad83_1986_alaska.grid",
        bounds: [172.0, 50.0, 232.0, 73.0],
    },
];

fn nc5_covers(r: &Nc5Region, lat: f64, lon: f64) -> bool {
    let [w, s, e, n] = r.bounds;
    let lon = if lon < w { lon + 360.0 } else { lon };
    (s..=n).contains(&lat) && (w..=e).contains(&lon)
}

pub static NADCON5: ToolDef = ToolDef {
    id: "geodesy.datum.nadcon5",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Old US datums to NAD 83 (NADCON5)",
    summary: "Converts a latitude and longitude from a region's old datum to NAD 83 with the NGS NADCON5 grids, or back: NAD 27 in the conterminous US and Alaska, Old Hawaiian, Puerto Rico 1940, St. Paul 1952, American Samoa 1962, and Guam 1963.",
    aliases: &[
        "NAD27 to NAD83",
        "NADCON",
        "NAD83 to NAD27",
        "old survey coordinates to NAD83",
        "Old Hawaiian to NAD83",
        "Puerto Rico 1940 to NAD83",
    ],
    keywords: &[
        "NADCON5",
        "NADCON",
        "NAD 27",
        "NAD 83",
        "datum shift",
        "grid",
        "NGS",
        "survey",
        "Old Hawaiian",
        "Alaska",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "Decimal degrees, like 39.5",
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
            "Decimal degrees, like -98.25",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
        Field::new(
            "direction",
            "Direction",
            "to-nad83 (default) or from-nad83 (nad27-to-nad83 and nad83-to-nad27 also work)",
            Kind::Choice(&["to-nad83", "from-nad83", "nad27-to-nad83", "nad83-to-nad27"]),
        )
        .core(),
        Field::new(
            "region",
            "Region",
            "Chosen from the point unless set: conus, alaska, hawaii, prvi, stpaul, samoa, or guam",
            Kind::Choice(&[
                "conus", "alaska", "hawaii", "prvi", "stpaul", "samoa", "guam",
            ]),
        ),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On the target datum",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "On the target datum",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        mm_out(
            "shift",
            "Horizontal shift",
            "Between the old-datum and NAD 83 coordinates of the point",
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "azimuth",
            "Shift direction",
            "Degrees from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        qty(
            "dlat",
            "Latitude shift",
            "NAD 83 minus the old datum",
            QT::Angle,
            "arcsec",
        )
        .precision(Precision::Decimals(5)),
        qty(
            "dlon",
            "Longitude shift",
            "NAD 83 minus the old datum, east positive",
            QT::Angle,
            "arcsec",
        )
        .precision(Precision::Decimals(5)),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
    ],
    warnings: &["INPUT_NORMALIZED"],
    model: "NADCON5 first-step grid for the region, biquadratic interpolation (NGS qterp); the reverse by iteration",
    accuracy: "Matches PROJ's NADCON5 transformations to 1e-9°; NGS states each first step itself at the decimeter level (about 0.15 m, 1σ, for NAD 27 in CONUS)",
    when_to_use: "Use this for old American coordinates when the answer has to be better than a meter. NAD 27 and the other regional datums — Old Hawaiian, Puerto Rico 1940, St. Paul 1952, American Samoa 1962, Guam 1963 — differ from NAD 83 by an amount that changes from place to place across the country, and the NGS grids model that variation directly instead of averaging it into one set of parameters. This is the transformation NGS itself publishes for the job, so it is what a survey, a parcel record, or a republished map should be brought forward with.",
    limitations: "It only covers what the grids cover: the conterminous United States, Alaska, Hawaii, Puerto Rico and the Virgin Islands, St. Paul Island, American Samoa, and Guam and the Northern Marianas. A point outside all of them is refused rather than extrapolated, and the regional helmert transformation is the fallback for anywhere else. This is the first step of NADCON5 — the one that carries the old datum to NAD 83 (1986) — and NGS states it at the decimeter level, about 0.15 m for NAD 27 in the conterminous states, so the result is decimeters, not centimeters. Later NAD 83 realizations are a separate step, which the NAD 83 tool handles. Only latitude and longitude are converted; the vertical grids are not part of this.",
    references: &[NADCON5_REF, BIQUADRATIC_REF],
    examples: &[Example {
        id: "primary",
        title: "Central Kansas (Meades Ranch, the NAD 27 origin)",
        input: r#"{"lat":39.224,"lon":-98.542}"#,
        source: "PROJ 9.3 applying the NADCON5 grid us_noaa_nadcon5_nad27_nad83_1986_conus.tif through pyproj, which the 34 grid vectors come from across all seven regions; the grid moves this point 29.815 m where the single-parameter EPSG helmert moves it 31.793 m, the 2.5 m between them being the regional variation a grid carries and a helmert cannot",
    }],
    primary_example: "primary",
    assets: &[NADCON5_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.legacy",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.nad83",
            reason: "next",
        },
        Related {
            id: "geodesy.datum.helmert",
            reason: "alternative",
        },
    ],
    sentence: "The point moves {shift} toward {azimuth} between the old datum and NAD 83.",
    limits: &[("batchRows", 10_000)],
    run: run_nadcon5,
    ..ToolDef::BLANK
};

fn run_nadcon5(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let to83 = !matches!(
        ctx.choice("direction")?,
        Some("from-nad83" | "nad83-to-nad27")
    );
    let region = match ctx.choice("region")? {
        Some(id) => NC5_REGIONS.iter().find(|r| r.id == id).expect("choice lists the table"),
        None => NC5_REGIONS.iter().find(|r| nc5_covers(r, lat, lon)).ok_or_else(|| {
            ToolError::new(
                ErrorCode::OutOfDomain,
                "The point is outside every NADCON5 grid: the conterminous US, Alaska, Hawaii, Puerto Rico and the Virgin Islands, St. Paul Island, American Samoa, and Guam.",
            )
            .at("/lat")
            .hint("For NAD 27 elsewhere, the legacy EPSG shift works at meter accuracy.")
        })?,
    };
    let bytes = ctx.asset(NADCON5_ID, NADCON5_VERSION, region.file)?;
    let grid = gp_geo::nadcon5::Grid::parse(&bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The NADCON5 grid could not be read: {e}."),
        )
    })?;
    let got = if to83 {
        grid.forward(lat, lon)
    } else {
        grid.reverse(lat, lon)
    };
    let Some((lat2, lon2)) = got else {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!("The point is outside the NADCON5 grid for {}.", region.name),
        )
        .at("/lat"));
    };
    ctx.model = Some(format!(
        "NADCON5 {} to {} for {}, biquadratic interpolation (NGS qterp)",
        region.datum, region.target, region.name
    ));
    ctx.context.push(("region", Json::str(region.id)));
    ctx.context.push((
        "from",
        Json::str(if to83 { region.datum } else { region.target }),
    ));
    ctx.context.push((
        "to",
        Json::str(if to83 { region.target } else { region.datum }),
    ));
    ctx.context.push((
        "grids",
        Json::Arr(vec![Json::str(region.file.trim_end_matches(".grid"))]),
    ));
    let (old, new) = if to83 {
        ((lat, lon), (lat2, lon2))
    } else {
        ((lat2, lon2), (lat, lon))
    };
    let dlon = gp_base::angle::wrap_lon(new.1 - old.1);
    let (dlat, dlon) = ((new.0 - old.0) * 3600.0, dlon * 3600.0);
    // On the ground: the east-north offset on GRS 80 (sub-kilometer shifts).
    let grs80 = CATALOG
        .iter()
        .find(|e| e.id == "grs80")
        .copied()
        .expect("GRS 80");
    let (p1, p2) = (
        (old.0.to_radians(), old.1.to_radians()),
        (new.0.to_radians(), new.1.to_radians()),
    );
    let a = fr::to_ecef(&grs80, p1.0, p1.1, 0.0);
    let b = fr::to_ecef(&grs80, p2.0, p2.1, 0.0);
    let enu = fr::ecef_to_enu(a, p1.0, p1.1, b);
    let shift = enu[0].hypot(enu[1]);
    let az = gp_base::angle::wrap_azimuth(enu[0].atan2(enu[1]).to_degrees());
    let arcsec = |v: f64| Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "arcsec").expect("arcsec"),
    };
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat2))),
        ("lon", ctx.out("lon", deg(lon2))),
        ("shift", ctx.out("shift", m(shift))),
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("dlat", ctx.out("dlat", arcsec(dlat))),
        ("dlon", ctx.out("dlon", arcsec(dlon))),
    ]))
}
