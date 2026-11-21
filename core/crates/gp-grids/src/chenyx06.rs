//! LV03 to LV95 by swisstopo's CHENyx06 (add-international-grids,
//! geodesy/national-grids, "Canada, Europe, and the other national paths").
//! The math is in gp_geo::chenyx06.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::chenyx06;
use gp_geo::hgrid::Grid;

pub const CHENYX06_ID: &str = "chenyx06";
pub const CHENYX06_VERSION: &str = "2009";
const FILE: &str = "chenyx06a.grid";

const NOTICE: &str = "Derived from work by swisstopo, released under CC0 1.0.";
const ACCURACY: &str = "swisstopo states the LV03 to LV95 difference as up to 1.6 m, which is what this removes; the grid is swisstopo's own approximation of its FINELTRA transformation, and it reproduces the five EUREF points swisstopo publishes within 8 mm, against 1.6 m for treating LV03 as LV95 less the round 2,000 km.";

const SWISSTOPO: Reference = Reference {
    title: "Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems",
    issuer: "Federal Office of Topography swisstopo",
    year: 2016,
    edition: "December 2016 edition",
    locator: "Sections 6.1 and 6.2, the strict transformations between LV03/LN02 and ETRS89, whose LV03 to LV95 step is \"the Fineltra algorithm (data set CHENyx06) or a grid approximation of the differences\"; section 7.1, the numerical example with the five EUREF points of Switzerland in LV03 and LV95",
    url: "https://www.swisstopo.admin.ch/dam/en/sd-web/c46Fz-MHIc3u/refsys-EN.pdf",
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

pub static CHENYX06: ToolDef = ToolDef {
    id: "geodesy.datum.chenyx06",
    stability: gp_base::tool::Stability::Stable,
    title: "LV03 to LV95 by swisstopo's CHENyx06",
    summary: "Moves Swiss coordinates between LV03 and LV95 by swisstopo's CHENyx06 grid, which removes the up to 1.6 m distortion of the 1903 triangulation that subtracting a round 2,000 km leaves behind.",
    aliases: &[
        "CHENyx06",
        "LV03 to LV95",
        "LV95 to LV03",
        "Swiss coordinate conversion",
        "CH1903 to CH1903+",
    ],
    keywords: &[
        "CHENyx06",
        "LV03",
        "LV95",
        "CH1903",
        "swisstopo",
        "Switzerland",
        "FINELTRA",
        "datum",
    ],
    inputs: &[
        length(
            "easting",
            "Easting",
            "On LV03 (around 600,000 m) or on LV95 when going the other way (around 2,600,000 m), like 602030.68",
        )
        .required()
        .core()
        .precision(Precision::Decimals(3)),
        length(
            "northing",
            "Northing",
            "On LV03 (around 200,000 m) or on LV95 when going the other way (around 1,200,000 m), like 191775.03",
        )
        .required()
        .core()
        .precision(Precision::Decimals(3)),
        Field::new(
            "direction",
            "Direction",
            "to-lv95 (the default) from LV03 coordinates, or to-lv03, like to-lv95",
            Kind::Choice(&["to-lv95", "to-lv03"]),
        ),
    ],
    outputs: &[
        length("easting", "Easting", "On the grid moved to").precision(Precision::Decimals(4)),
        length("northing", "Northing", "On the grid moved to").precision(Precision::Decimals(4)),
        length(
            "east_shift",
            "Easting distortion",
            "What CHENyx06 adds here beyond the round 2,000,000 m, LV03 to LV95",
        )
        .precision(Precision::Decimals(4)),
        length(
            "north_shift",
            "Northing distortion",
            "What CHENyx06 adds here beyond the round 1,000,000 m, LV03 to LV95",
        )
        .precision(Precision::Decimals(4)),
        length(
            "shift",
            "How far it moved",
            "The distortion itself, horizontally",
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "notice",
            "Notice",
            "The attribution swisstopo's licence asks for",
            Kind::Text { max_len: 80 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &[],
    model: "swisstopo's CHENyx06 grid, latitude and longitude offsets at 30-arcsecond nodes from 5°33′E 45°28′N to 11°03′E 48°04′N, interpolated bilinearly: the LV03 coordinates unprojected to CH1903 by the Swiss oblique Mercator, the offsets added to give CH1903+, and the result projected again to LV95; the reverse by iterating the grid, as PROJ's hgridshift does",
    accuracy: ACCURACY,
    when_to_use: "Use this whenever Swiss coordinates have to move between LV03 and LV95 and metres matter: cadastral work, survey control, or joining an older LV03 dataset to current LV95 mapping. Subtracting 2,000,000 m and 1,000,000 m is the one-metre-level shortcut swisstopo allows; this is the transformation it defines for everything finer.",
    limitations: "swisstopo transforms with FINELTRA, a triangulated affine transformation on the projection plane; this applies the grid approximation of the same differences that swisstopo's own documentation offers beside it, which lands within 8 mm of its published results. Outside the grid, 5°33′E to 11°03′E and 45°28′N to 48°04′N, there is no answer. Heights are untouched: LN02 to LHN95 is swisstopo's HTRANS, which is not here.",
    references: &[SWISSTOPO],
    examples: &[Example {
        id: "primary",
        title: "Zimmerwald, a swisstopo EUREF point",
        source: "swisstopo, Formulas and constants (December 2016), section 7.1: Zimmerwald at LV03 602030.680, 191775.030 is LV95 2602030.740, 1191775.030 by FINELTRA with CHENyx06",
        input: r#"{"easting":"602030.68 m","northing":"191775.03 m"}"#,
    }],
    primary_example: "primary",
    assets: &[CHENYX06_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.projection.lv95-inverse",
            reason: "next",
        },
        Related {
            id: "geodesy.projection.lv95-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "alternative",
        },
    ],
    sentence: "That point is E {easting}, N {northing}, {shift} from the round shift.",
    limits: &[("batchRows", 10_000)],
    run: run_chenyx06,
    ..ToolDef::BLANK
};

fn run_chenyx06(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mm = unit(QT::Length, "m");
    let east = ctx.req_quantity("easting")?.to(mm);
    let north = ctx.req_quantity("northing")?.to(mm);
    if !east.is_finite() || !north.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on LV03 or LV95.",
        ));
    }
    let to_lv95 = ctx.choice("direction")? != Some("to-lv03");
    let bytes = ctx.asset(CHENYX06_ID, CHENYX06_VERSION, FILE)?;
    let grid = Grid::parse(&bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The CHENyx06 grid could not be read: {e}."),
        )
    })?;
    let outside = || {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "The point is outside swisstopo's CHENyx06 grid, which covers 5°33′E to 11°03′E and 45°28′N to 48°04′N.",
        )
        .at("/easting")
    };
    let ((e, n), step) = if to_lv95 {
        chenyx06::to_lv95(&grid, east, north).ok_or_else(outside)?
    } else {
        chenyx06::to_lv03(&grid, east, north).ok_or_else(outside)?
    };
    // The distortion is what the grid adds beyond the round offsets, measured
    // LV03 to LV95 whichever way the tool was asked to run.
    let (de, dn) = if to_lv95 {
        (
            e - (east + chenyx06::LV03_EAST),
            n - (north + chenyx06::LV03_NORTH),
        )
    } else {
        (
            east - (e + chenyx06::LV03_EAST),
            north - (n + chenyx06::LV03_NORTH),
        )
    };
    let shift = de.hypot(dn);
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "Off the Swiss projection",
            "the Swiss oblique Mercator read backwards onto the Bessel ellipsoid, at LV95's false origin",
            format!("{east:.3} m, {north:.3} m"),
            format!("{:.9}°, {:.9}° on CH1903", step.from.0, step.from.1),
        );
        ctx.step(
            "The grid's offsets",
            "bilinear between the four 30-arcsecond nodes around the point",
            format!("{:.9}°, {:.9}°", step.from.0, step.from.1),
            format!("Δφ {:.6}\", Δλ {:.6}\"", step.offsets.0, step.offsets.1),
        );
        ctx.step(
            if to_lv95 {
                "Onto CH1903+"
            } else {
                "Back onto CH1903"
            },
            if to_lv95 {
                "the offsets added"
            } else {
                "the offsets taken away, iterated until the point settles"
            },
            format!("{:.9}°, {:.9}°", step.from.0, step.from.1),
            format!(
                "{:.9}°, {:.9}°{}",
                step.to.0,
                step.to.1,
                if to_lv95 {
                    String::new()
                } else {
                    format!(" after {} iterations", step.iterations)
                }
            ),
        );
        ctx.step(
            "Projected again",
            "the Swiss oblique Mercator forwards, then the round 2,000 km if the answer is LV03",
            format!("{:.9}°, {:.9}°", step.to.0, step.to.1),
            format!(
                "{} m",
                gp_base::display::number(e, Precision::Decimals(4), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(e))),
        ("northing", ctx.out("northing", m(n))),
        ("east_shift", ctx.out("east_shift", m(de))),
        ("north_shift", ctx.out("north_shift", m(dn))),
        ("shift", ctx.out("shift", m(shift))),
        ("notice", Json::str(NOTICE)),
    ]))
}
