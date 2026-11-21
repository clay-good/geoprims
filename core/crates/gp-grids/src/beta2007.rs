//! Germany's DHDN Gauss-Krüger to ETRS89 UTM and back by AdV's BeTA2007
//! (add-international-grids, geodesy/national-grids, "Canada, Europe, and the
//! other national paths"). The math is in gp_geo::beta2007.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::beta2007;
use gp_geo::hgrid::Grid;

pub const BETA2007_ID: &str = "beta2007";
pub const BETA2007_VERSION: &str = "2009";
const FILE: &str = "beta2007.grid";

const NOTICE: &str = "Derived from work by AdV. Free redistribution is allowed and welcome.";
const ACCURACY: &str = "AdV states that BeTA2007 reproduces the state transformations it unified within 0.02 m inside Germany; this tool reproduces all six transformation records AdV publishes in its own documentation within 2 micrometres, and agrees with PROJ 9.9.0 applying the same grid within 0.01 mm.";
const FRAME: &str = "The Gauss-Krüger coordinates are taken to be on DHDN90, the realization BeTA2007 is defined from, and the UTM coordinates on ETRS89. A WGS 84 or ITRF position from a consumer receiver is about 0.8 m from ETRS89 in Germany today.";

const ADV: Reference = Reference {
    title: "Bundeseinheitliche Transformation für ATKIS (BeTA2007)",
    issuer: "Arbeitsgemeinschaft der Vermessungsverwaltungen der Länder der Bundesrepublik Deutschland (AdV)",
    year: 2009,
    edition: "Documentation version 1.5, grid BETA2007.gsb of 6 November 2009, read 2026-11-21",
    locator: "Section 3, the NTv2 grid and its bilinear interpolation; section 4.2, the 31-point test data set, of which the documentation prints the first six records: for each of two points the DHDN to ETRS89 shift in degrees and the point on two Gauss-Krüger zones carried to ETRS89 UTM zone 32; section 6.1.2.3, the Gauss-Krüger formulas, whose easting the test data carries with the zone number in front of it",
    url: "https://crs.bkg.bund.de/crs/descrtrans/BeTA/BETA2007dokumentationV15.pdf",
};

fn unit(q: QT, s: &str) -> &'static gp_base::units::Unit {
    units::by_symbol(q, s).expect("registered unit")
}
fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}
fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}
const fn length(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}

const CONVERGENCE: Field = Field::new(
    "convergence",
    "Grid convergence",
    "Grid north minus true north at the point, on the grid answered in",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(6))
.angle_range("[-90,90]");

const SCALE: Field = Field::new(
    "scale_factor",
    "Point scale factor",
    "Grid distance over ground distance at the point, on the grid answered in",
    Kind::Number { min: 0.0, max: 2.0 },
)
.precision(Precision::Decimals(9));

const SHIFT: Field = length(
    "shift",
    "How far the datum moved it",
    "On the ground, between DHDN90 and ETRS89 at this point",
)
.precision(Precision::Decimals(3));

const NOTICE_OUT: Field = Field::new(
    "notice",
    "Notice",
    "The attribution AdV asks for",
    Kind::Text { max_len: 80 },
);

const ZONE_OUT: Field = Field::new(
    "zone",
    "Zone",
    "The zone of the answer",
    Kind::Number {
        min: 2.0,
        max: 60.0,
    },
)
.precision(Precision::Decimals(0));

fn grid<'a>(bytes: &'a [u8]) -> Result<Grid<'a>, ToolError> {
    Grid::parse(bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The BeTA2007 grid could not be read: {e}."),
        )
    })
}

fn outside(at: &str) -> ToolError {
    ToolError::new(
        ErrorCode::OutOfDomain,
        "The point is outside AdV's BeTA2007 grid, which covers Germany: 5°30′E to 15°40′E and 47°N to 55°18′N.",
    )
    .at(at)
}

fn plane(ctx: &mut Ctx) -> Result<(f64, f64), ToolError> {
    let mm = unit(QT::Length, "m");
    let e = ctx.req_quantity("easting")?.to(mm);
    let n = ctx.req_quantity("northing")?.to(mm);
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on the grid named.",
        ));
    }
    Ok((e, n))
}

pub static BETA2007_FORWARD: ToolDef = ToolDef {
    id: "geodesy.datum.beta2007",
    stability: gp_base::tool::Stability::Stable,
    title: "Gauss-Krüger to ETRS89 UTM by BeTA2007",
    summary: "Turns a German DHDN Gauss-Krüger easting and northing into ETRS89 UTM by AdV's BeTA2007 grid, the one national transformation that replaced the state-by-state ones, with the UTM zone picked for you.",
    aliases: &[
        "BeTA2007",
        "Gauss-Kruger to UTM",
        "DHDN to ETRS89",
        "German coordinate conversion",
        "GK to UTM32",
    ],
    keywords: &[
        "BeTA2007",
        "DHDN",
        "Gauss-Krüger",
        "ETRS89",
        "UTM",
        "Germany",
        "AdV",
        "ATKIS",
    ],
    inputs: &[
        length(
            "easting",
            "Gauss-Krüger easting",
            "With the zone number in front of it, as German data carries it, like 3399371.19",
        )
        .required()
        .core()
        .precision(Precision::Decimals(3)),
        length(
            "northing",
            "Gauss-Krüger northing",
            "From the equator, like 5930724.531",
        )
        .required()
        .core()
        .precision(Precision::Decimals(3)),
        Field::new(
            "utm_zone",
            "UTM zone",
            "Left out, the zone the point falls in; name one to force it, like 32",
            Kind::Number {
                min: 31.0,
                max: 33.0,
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    outputs: &[
        length("easting", "UTM easting", "On ETRS89 UTM, in the zone named")
            .precision(Precision::Decimals(4)),
        length("northing", "UTM northing", "On ETRS89 UTM").precision(Precision::Decimals(4)),
        ZONE_OUT,
        SHIFT,
        CONVERGENCE,
        SCALE,
        NOTICE_OUT,
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED", "NONSTANDARD_ZONE"],
    model: "AdV's BeTA2007 NTv2 grid, latitude and longitude offsets at 10 arcminutes of longitude and 6 of latitude across Germany, interpolated bilinearly: the Gauss-Krüger coordinates unprojected on the three-degree zone their easting names, on Bessel 1841, the offsets added to give ETRS89, and the result projected on UTM, on GRS 80, by Karney's series",
    accuracy: ACCURACY,
    when_to_use: "Use this to bring older German data onto current mapping: Gauss-Krüger on DHDN is what state cadastral and topographic data was published in before the ETRS89 UTM changeover, and BeTA2007 is the transformation AdV defines for the move, so it is the one that matches official conversions. The easting already names its Gauss-Krüger zone, so there is no zone to look up, and the UTM zone is picked from where the point lands.",
    limitations: "The input is taken to be DHDN90 and the answer ETRS89; a WGS 84 position from a consumer receiver is about 0.8 m from ETRS89, which geodesy.datum.etrf removes. Outside Germany, 5°30′E to 15°40′E and 47°N to 55°18′N, there is no answer. AdV states 0.02 m against the state transformations BeTA2007 unified, and some states publish their own finer grids for cadastral work. Heights are untouched: the German quasi-geoid GCG2016 is a separate asset.",
    references: &[ADV],
    examples: &[Example {
        id: "primary",
        title: "AdV's first test point, on zone 3",
        source: "AdV, BeTA2007 documentation v1.5, section 4.2, point 1 record 3: DE_DHDN_3GK3 3399371.190396, 5930724.531323 gives ETRS89_UTM32 399340.601862, 5928794.177992",
        input: r#"{"easting":"3399371.190396 m","northing":"5930724.531323 m"}"#,
    }],
    primary_example: "primary",
    assets: &[BETA2007_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.beta2007-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on UTM zone {zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (east, north) = plane(ctx)?;
    let gk = beta2007::gk_zone_of(east).ok_or_else(|| {
        ToolError::invalid(
            "/easting",
            "A German Gauss-Krüger easting carries its zone in front of it, so it reads like 3,399,371 m on zone 3: the zone digit must be 2, 3, 4, or 5, and the rest must be within 300 km of that zone's central meridian.",
        )
    })?;
    let bytes = ctx.asset(BETA2007_ID, BETA2007_VERSION, FILE)?;
    let grid = grid(&bytes)?;
    // The zone the point lands in, which needs the ETRS89 longitude, so the
    // transformation runs once on the standard zone and again if one is forced.
    let (_, _, probe) =
        beta2007::to_utm(&grid, gk, 32, east, north).ok_or_else(|| outside("/easting"))?;
    let standard = beta2007::utm_zone_of(probe.etrs89.1);
    let zone = match ctx.number("utm_zone")? {
        Some(z) => {
            let z = z.round() as u8;
            if z != standard {
                ctx.warnings.push(Warning::new(
                    "NONSTANDARD_ZONE",
                    format!(
                        "The longitude {:.6}° falls in UTM zone {standard}, and the answer is on zone {z} as asked. Coordinates on a forced zone are further from its central meridian, so the point scale factor is further from 0.9996.",
                        probe.etrs89.1
                    ),
                ));
            }
            z
        }
        None => standard,
    };
    let ((e, n), p, step) =
        beta2007::to_utm(&grid, gk, zone, east, north).ok_or_else(|| outside("/easting"))?;
    let (north, east) = beta2007::grs80().ground_offset(
        step.dhdn.0,
        step.etrs89.0 - step.dhdn.0,
        step.etrs89.1 - step.dhdn.1,
    );
    let shift = north.hypot(east);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "The Gauss-Krüger zone",
            "the digits in front of the 500,000 m false easting name it",
            format!("{east:.3} m"),
            format!("zone {gk}, central meridian {}°E", 3 * i32::from(gk)),
        );
        ctx.step(
            "Off that zone",
            "transverse Mercator on Bessel 1841, scale 1, read backwards",
            format!("{east:.3} m, {north:.3} m"),
            format!("{:.9}°, {:.9}° on DHDN90", step.dhdn.0, step.dhdn.1),
        );
        ctx.step(
            "The grid's offsets",
            "bilinear between the four nodes around the point, 10 arcminutes of longitude by 6 of latitude",
            format!("{:.9}°, {:.9}°", step.dhdn.0, step.dhdn.1),
            format!("Δφ {:.6}\", Δλ {:.6}\"", step.offsets.0, step.offsets.1),
        );
        ctx.step(
            "Onto UTM",
            format!(
                "the offsets added to reach ETRS89, then transverse Mercator on GRS 80, scale 0.9996, on zone {zone}"
            ),
            format!("{:.9}°, {:.9}°", step.etrs89.0, step.etrs89.1),
            format!(
                "{} m",
                gp_base::display::number(e, Precision::Decimals(4), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(e))),
        ("northing", ctx.out("northing", m(n))),
        ("zone", Json::Num(f64::from(zone))),
        ("shift", ctx.out("shift", m(shift))),
        ("convergence", ctx.out("convergence", deg(p.convergence))),
        ("scale_factor", Json::Num(p.k)),
        ("notice", Json::str(NOTICE)),
    ]))
}

pub static BETA2007_INVERSE: ToolDef = ToolDef {
    id: "geodesy.datum.beta2007-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "ETRS89 UTM to Gauss-Krüger by BeTA2007",
    summary: "Turns a German ETRS89 UTM easting and northing back into DHDN Gauss-Krüger by AdV's BeTA2007 grid, with the three-degree zone picked for you and its number in front of the easting as German data carries it.",
    aliases: &[
        "UTM to Gauss-Kruger",
        "ETRS89 to DHDN",
        "BeTA2007 inverse",
        "UTM32 to GK",
        "ETRS89 UTM to Gauss-Krüger",
    ],
    keywords: &[
        "BeTA2007",
        "DHDN",
        "Gauss-Krüger",
        "ETRS89",
        "UTM",
        "Germany",
        "AdV",
        "inverse",
    ],
    inputs: &[
        length("easting", "UTM easting", "On ETRS89 UTM, like 399340.602")
            .required()
            .core()
            .precision(Precision::Decimals(3)),
        length(
            "northing",
            "UTM northing",
            "On ETRS89 UTM, like 5928794.178",
        )
        .required()
        .core()
        .precision(Precision::Decimals(3)),
        Field::new(
            "utm_zone",
            "UTM zone",
            "The zone the easting and northing are on, 31, 32, or 33 in Germany, like 32",
            Kind::Number {
                min: 31.0,
                max: 33.0,
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(0)),
        Field::new(
            "gk_zone",
            "Gauss-Krüger zone",
            "Left out, the three-degree zone the point falls in; name one to force it, like 3",
            Kind::Number { min: 2.0, max: 5.0 },
        )
        .precision(Precision::Decimals(0)),
    ],
    outputs: &[
        length(
            "easting",
            "Gauss-Krüger easting",
            "With the zone number in front of it, as German data carries it",
        )
        .precision(Precision::Decimals(4)),
        length("northing", "Gauss-Krüger northing", "From the equator")
            .precision(Precision::Decimals(4)),
        ZONE_OUT,
        SHIFT,
        CONVERGENCE,
        SCALE,
        NOTICE_OUT,
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED", "NONSTANDARD_ZONE"],
    model: "AdV's BeTA2007 NTv2 grid in reverse: the UTM coordinates unprojected on their zone, on GRS 80, the grid iterated until the DHDN90 point whose offsets land on that ETRS89 position settles, and the result projected on the three-degree Gauss-Krüger zone, on Bessel 1841, with the zone number put in front of the easting",
    accuracy: ACCURACY,
    when_to_use: "Use this to give a position in the Gauss-Krüger coordinates older German records, plans, and datasets are kept in, when the job itself is in ETRS89 UTM. The answer carries the zone number in front of the easting, which is how German data writes it.",
    limitations: "The input is taken to be ETRS89 and the answer DHDN90. Outside Germany, 5°30′E to 15°40′E and 47°N to 55°18′N, there is no answer. AdV states 0.02 m against the state transformations BeTA2007 unified. Heights are untouched. Gauss-Krüger zones overlap by half a degree either side of their edges, so a point near one can legitimately be given on either zone, and naming a zone is how to choose.",
    references: &[ADV],
    examples: &[Example {
        id: "primary",
        title: "AdV's first test point, read back",
        source: "AdV, BeTA2007 documentation v1.5, section 4.2, point 1 record 3 the other way: ETRS89_UTM32 399340.601862, 5928794.177992 gives DE_DHDN_3GK3 3399371.190396, 5930724.531323",
        input: r#"{"easting":"399340.601862 m","northing":"5928794.177992 m","utm_zone":32}"#,
    }],
    primary_example: "primary",
    assets: &[BETA2007_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.beta2007",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on Gauss-Krüger zone {zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (east, north) = plane(ctx)?;
    let utm = ctx
        .number("utm_zone")?
        .ok_or_else(|| ToolError::invalid("/utm_zone", "A UTM easting does not say which zone it is on, so the zone is needed: 31, 32, or 33 in Germany."))?
        .round() as u8;
    let bytes = ctx.asset(BETA2007_ID, BETA2007_VERSION, FILE)?;
    let grid = grid(&bytes)?;
    // The three-degree zone the point lands in, which needs the DHDN
    // longitude, so the transformation runs once on a standard zone first.
    let (_, _, probe) =
        beta2007::to_gauss_kruger(&grid, utm, 3, east, north).ok_or_else(|| outside("/easting"))?;
    let standard = beta2007::gk_zone_for(probe.dhdn.1);
    let gk = match ctx.number("gk_zone")? {
        Some(z) => {
            let z = z.round() as u8;
            if z != standard {
                ctx.warnings.push(Warning::new(
                    "NONSTANDARD_ZONE",
                    format!(
                        "The longitude {:.6}° falls in Gauss-Krüger zone {standard}, and the answer is on zone {z} as asked. Zones overlap, so this is a legitimate choice, but coordinates on a forced zone are further from its central meridian.",
                        probe.dhdn.1
                    ),
                ));
            }
            z
        }
        None => standard,
    };
    let ((e, n), p, step) = beta2007::to_gauss_kruger(&grid, utm, gk, east, north)
        .ok_or_else(|| outside("/easting"))?;
    let (north, east) = beta2007::grs80().ground_offset(
        step.dhdn.0,
        step.etrs89.0 - step.dhdn.0,
        step.etrs89.1 - step.dhdn.1,
    );
    let shift = north.hypot(east);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "Off UTM",
            "transverse Mercator on GRS 80, scale 0.9996, read backwards on the zone given",
            format!("zone {utm}, {east:.3} m, {north:.3} m"),
            format!("{:.9}°, {:.9}° on ETRS89", step.etrs89.0, step.etrs89.1),
        );
        ctx.step(
            "Back onto DHDN90",
            "the grid read backwards: the point whose offsets land on this ETRS89 position, iterated until it settles",
            format!("{:.9}°, {:.9}°", step.etrs89.0, step.etrs89.1),
            format!(
                "{:.9}°, {:.9}° after {} iterations",
                step.dhdn.0, step.dhdn.1, step.iterations
            ),
        );
        ctx.step(
            "Onto the Gauss-Krüger zone",
            format!(
                "transverse Mercator on Bessel 1841, scale 1, on zone {gk}, with the zone number in front of the easting"
            ),
            format!("{:.9}°, {:.9}°", step.dhdn.0, step.dhdn.1),
            format!(
                "{} m",
                gp_base::display::number(e, Precision::Decimals(4), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(e))),
        ("northing", ctx.out("northing", m(n))),
        ("zone", Json::Num(f64::from(gk))),
        ("shift", ctx.out("shift", m(shift))),
        ("convergence", ctx.out("convergence", deg(p.convergence))),
        ("scale_factor", Json::Num(p.k)),
        ("notice", Json::str(NOTICE)),
    ]))
}
