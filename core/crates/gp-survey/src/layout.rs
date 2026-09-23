//! Curve stationing and layout (add-survey-suite, survey/alignment-curves,
//! "Stationing and layout"): PC and PT stations for a circular curve, and a
//! layout table at a chosen interval with deflections from the PC, chords, and
//! coordinates when the PI's position and the back tangent are known.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point::plain_angle;
use libm::{cos, sin, tan};

use crate::{common_unit, direction, fmt_station, is_metric, len, len_out, station};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 24 (horizontal curves: stationing, deflection angles, chords, and layout by coordinates)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const ROW: &[Field] = &[
    Field::new(
        "station",
        "Station",
        "Along the curve",
        Kind::Text { max_len: 16 },
    ),
    Field::new(
        "deflection",
        "Deflection from the PC",
        "Arc from the PC ÷ 2R",
        Kind::Text { max_len: 24 },
    ),
    len_out("chord_from_pc", "Chord from the PC", "2R sin(deflection)"),
    len_out("chord", "Chord from the last point", "Full or sub-chord"),
    len_out(
        "northing",
        "Northing",
        "When the PI and back tangent are given",
    )
    .optional(),
    len_out(
        "easting",
        "Easting",
        "When the PI and back tangent are given",
    )
    .optional(),
];

pub static CURVE_LAYOUT: ToolDef = ToolDef {
    id: "survey.curves.curve-layout",
    title: "Circular curve layout table",
    summary: "PC and PT stations for a circular curve and a layout table at a chosen interval: station, deflection from the PC, chords, and coordinates when the PI position and back tangent are given.",
    aliases: &[
        "curve layout",
        "deflection table",
        "curve staking",
        "stationing",
    ],
    keywords: &[
        "layout",
        "deflection",
        "chord",
        "station",
        "PC",
        "PT",
        "PI",
        "staking",
        "horizontal curve",
    ],
    inputs: &[
        len("radius", "Radius R", "Like 500 ft").required().core(),
        Field::new(
            "delta",
            "Deflection Δ",
            "Like 30°00'00\"",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded")
        .required()
        .core(),
        Field::new(
            "pi_station",
            "PI station",
            "Like 12+34.56 (feet) or 1+234.567 (meters)",
            Kind::Text { max_len: 16 },
        )
        .required()
        .core(),
        len("interval", "Station interval", "Like 50 ft")
            .required()
            .core(),
        Field::new(
            "turn",
            "Turn",
            "right (default) or left",
            Kind::Choice(&["right", "left"]),
        )
        .core(),
        len(
            "pi_northing",
            "PI northing",
            "For coordinates, like 5000.00 ft",
        ),
        len(
            "pi_easting",
            "PI easting",
            "For coordinates, like 5000.00 ft",
        ),
        Field::new(
            "back_azimuth",
            "Back tangent direction",
            "Direction of travel into the PI, like N 45°00'00\" E",
            Kind::Text { max_len: 32 },
        ),
    ],
    outputs: &[
        Field::new(
            "pc_station",
            "PC station",
            "PI − T",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "pt_station",
            "PT station",
            "PC + L",
            Kind::Text { max_len: 16 },
        ),
        len_out("tangent", "Tangent T", "R tan(Δ/2)"),
        len_out("length", "Curve length L", "R Δ in radians"),
        Field::new(
            "rows",
            "Layout",
            "One row per station",
            Kind::List {
                items: ROW,
                min: 2,
                max: 10_000,
            },
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Arc definition: T = R tan(Δ/2), L = RΔ; deflection from the PC δ = s/(2R) for arc s; chord 2R sin δ; coordinates from the PC along the back tangent rotated by δ (Ghilani & Wolf 2021, ch. 24)",
    accuracy: "Exact for the elements given",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "R = 500 ft, Δ = 30°, PI at 12+34.56, staked every 50 ft",
        input: r#"{"radius":"500 ft","delta":"30°00'00\"","pi_station":"12+34.56","interval":"50 ft"}"#,
        source: "add-survey-suite layout-table scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.curves.circular-curve",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.forward",
            reason: "next",
        },
    ],
    sentence: "The curve runs from PC {pc_station} to PT {pt_station}, {length} of arc.",
    limits: &[("batchRows", 1_000)],
    run: run_layout,
    ..ToolDef::BLANK
};

fn run_layout(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = ctx.req_quantity("radius")?;
    let step = ctx.req_quantity("interval")?;
    let pin = ctx.quantity("pi_northing")?;
    let pie = ctx.quantity("pi_easting")?;
    let mut qs = vec![("radius", r), ("interval", step)];
    qs.extend(pin.map(|q| ("pi_northing", q)));
    qs.extend(pie.map(|q| ("pi_easting", q)));
    let u = common_unit(&qs)?;
    let (r, step) = (r.to(u), step.to(u));
    if r <= 0.0 || step <= 0.0 {
        return Err(ToolError::invalid(
            "/radius",
            "The radius and the interval must be positive.",
        ));
    }
    let delta = plain_angle(ctx, "delta")?.expect("required");
    if !(0.0 < delta && delta < 180.0) {
        return Err(ToolError::invalid(
            "/delta",
            "The deflection is between 0° and 180°.",
        ));
    }
    let pi = station(ctx, "pi_station")?.expect("required");
    let dr = delta.to_radians();
    let t = r * tan(dr / 2.0);
    let l = r * dr;
    let (pc, pt) = (pi - t, pi - t + l);
    if (pt - pc) / step > 10_000.0 {
        return Err(ToolError::invalid(
            "/interval",
            "That interval gives more than 10,000 rows.",
        ));
    }
    // Stations: the PC, every interval after it, and the PT.
    let mut stations = vec![pc];
    let mut s = (pc / step).floor() * step + step;
    while s < pt - 1e-9 * step {
        if s > pc + 1e-9 * step {
            stations.push(s);
        }
        s += step;
    }
    stations.push(pt);
    let right = ctx.choice("turn")? != Some("left");
    let back = ctx
        .text("back_azimuth")?
        .map(|d| direction::parse(&d).map_err(|m| ToolError::invalid("/back_azimuth", m)))
        .transpose()?;
    let origin = match (pin, pie, back) {
        (Some(n), Some(e), Some(az)) => {
            let a = az.to_radians();
            Some((n.to(u) - t * cos(a), e.to(u) - t * sin(a), a))
        }
        (None, None, None) => None,
        _ => {
            return Err(ToolError::invalid(
                "/back_azimuth",
                "For coordinates, give the PI northing, easting, and the back tangent direction.",
            ));
        }
    };
    let metric = is_metric(u);
    let q = |v: f64| Q { value: v, unit: u };
    let mut rows = Vec::with_capacity(stations.len());
    let mut last = pc;
    for &st in &stations {
        let arc = st - pc;
        let defl = arc / (2.0 * r);
        let chord_pc = 2.0 * r * sin(defl);
        let chord = 2.0 * r * sin((st - last) / (2.0 * r));
        last = st;
        let mut row = vec![
            ("station", Json::Str(fmt_station(st, metric))),
            (
                "deflection",
                Json::Str(dms::format(
                    defl.to_degrees(),
                    Axis::Lon,
                    Style::Dms,
                    1,
                    false,
                )),
            ),
            ("chord_from_pc", ctx.emit("chord_from_pc", q(chord_pc), u)),
            ("chord", ctx.emit("chord", q(chord), u)),
        ];
        if let Some((n0, e0, a)) = origin {
            let dir = if right { a + defl } else { a - defl };
            row.push((
                "northing",
                ctx.emit("northing", q(n0 + chord_pc * cos(dir)), u),
            ));
            row.push((
                "easting",
                ctx.emit("easting", q(e0 + chord_pc * sin(dir)), u),
            ));
        }
        rows.push(Json::obj(row));
    }
    Ok(Json::obj(vec![
        ("pc_station", Json::Str(fmt_station(pc, metric))),
        ("pt_station", Json::Str(fmt_station(pt, metric))),
        ("tangent", ctx.emit("tangent", q(t), u)),
        ("length", ctx.emit("length", q(l), u)),
        ("rows", Json::Arr(rows)),
    ]))
}
