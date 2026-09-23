//! Station and offset along a line (add-survey-suite, survey/cogo: station-
//! offset, point-from-station-offset, and perpendicular-foot): a point's
//! station and offset from an alignment, the foot of its perpendicular, or
//! the other way, the point at a station and offset. Offsets are positive to
//! the right of the direction of stationing, and the result says so.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use libm::{atan2, cos, hypot, sin};

use crate::{common_unit, direction, fmt_station, is_metric, len, len_out, station};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 11 (coordinate geometry: perpendicular distance and station-offset)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

pub static STATION_OFFSET: ToolDef = ToolDef {
    id: "survey.cogo.station-offset",
    title: "Station and offset from a line",
    summary: "A point's station and offset from a straight alignment, with the foot of its perpendicular, or the point at a given station and offset. Offsets are positive to the right.",
    aliases: &[
        "station offset",
        "perpendicular offset",
        "point from station and offset",
        "foot of perpendicular",
    ],
    keywords: &[
        "station",
        "offset",
        "alignment",
        "perpendicular",
        "centerline",
        "left",
        "right",
        "COGO",
    ],
    inputs: &[
        len("start_northing", "Line start northing", "Like 5000.00 ft")
            .required()
            .core(),
        len("start_easting", "Line start easting", "Like 5000.00 ft")
            .required()
            .core(),
        Field::new(
            "start_station",
            "Start station",
            "Like 10+00 (default 0+00)",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "direction",
            "Line direction",
            "Bearing or azimuth of stationing, like N 60°00'00\" E",
            Kind::Text { max_len: 32 },
        )
        .core(),
        len(
            "end_northing",
            "Line end northing",
            "Instead of a direction, like 5500.00 ft",
        ),
        len(
            "end_easting",
            "Line end easting",
            "Instead of a direction, like 5866.03 ft",
        ),
        len(
            "northing",
            "Point northing",
            "To find its station and offset, like 5300.00 ft",
        )
        .core(),
        len("easting", "Point easting", "Like 5400.00 ft").core(),
        Field::new(
            "station",
            "Station",
            "Instead of a point: the station to set out, like 13+50",
            Kind::Text { max_len: 16 },
        ),
        len(
            "offset",
            "Offset",
            "With the station: right positive, left negative, like -12.5 ft",
        ),
    ],
    outputs: &[
        Field::new(
            "station",
            "Station",
            "Along the line",
            Kind::Text { max_len: 16 },
        ),
        len_out(
            "offset",
            "Offset",
            "Right of the line positive, left negative",
        ),
        Field::new(
            "side",
            "Side",
            "left, right, or on the line",
            Kind::Text { max_len: 12 },
        ),
        len_out(
            "foot_northing",
            "Foot of the perpendicular, northing",
            "On the line",
        ),
        len_out(
            "foot_easting",
            "Foot of the perpendicular, easting",
            "On the line",
        ),
        len_out("northing", "Point northing", "The point itself"),
        len_out("easting", "Point easting", "The point itself"),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "With the line's unit direction t = (cos α, sin α) in (N, E): station = start + (P − S)·t, offset = (P − S) × t positive to the right; and back, P = S + along·t + offset·n with n the right-hand normal (Ghilani & Wolf 2018, ch. 11)",
    accuracy: "Exact",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A point beside a line bearing N 60° E from station 10+00",
        input: r#"{"start_northing":"5000 ft","start_easting":"5000 ft","start_station":"10+00","direction":"N 60°00'00\" E","northing":"5300 ft","easting":"5400 ft"}"#,
        source: "Plane geometry",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.inverse",
            reason: "alternative",
        },
        Related {
            id: "survey.curves.curve-layout",
            reason: "next",
        },
    ],
    sentence: "The point is at station {station}, {offset} from the line ({side}).",
    limits: &[("batchRows", 10_000)],
    run: run_station_offset,
    ..ToolDef::BLANK
};

fn run_station_offset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let names = [
        "start_northing",
        "start_easting",
        "end_northing",
        "end_easting",
        "northing",
        "easting",
        "offset",
    ];
    let mut vals = Vec::new();
    for n in names {
        vals.push(ctx.quantity(n)?);
    }
    let qs: Vec<(&str, Q)> = names
        .iter()
        .zip(&vals)
        .filter_map(|(n, q)| q.map(|q| (*n, q)))
        .collect();
    let u = common_unit(&qs)?;
    let v: Vec<Option<f64>> = vals.iter().map(|q| q.map(|q| q.to(u))).collect();
    let (sn, se) = (v[0].expect("required"), v[1].expect("required"));
    let s0 = station(ctx, "start_station")?.unwrap_or(0.0);
    let dir = ctx.text("direction")?;
    let az = match (dir, v[2], v[3]) {
        (Some(d), None, None) => direction::parse(&d)
            .map_err(|m| ToolError::invalid("/direction", m))?
            .to_radians(),
        (None, Some(en), Some(ee)) => {
            if hypot(en - sn, ee - se) == 0.0 {
                return Err(ToolError::invalid(
                    "/end_northing",
                    "The line's end is its start.",
                ));
            }
            atan2(ee - se, en - sn)
        }
        (Some(_), ..) => {
            return Err(ToolError::invalid(
                "/end_northing",
                "Give the line's direction or its end point, not both.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/direction",
                "Give the line's direction, or its end point's northing and easting.",
            ));
        }
    };
    let (tn, te) = (cos(az), sin(az));
    // The right-hand normal, in (N, E): a quarter turn clockwise.
    let (rn, re) = (-te, tn);
    let st_in = station(ctx, "station")?;
    let (along, off, pn, pe) = match (v[4], v[5], st_in, v[6]) {
        (Some(pn), Some(pe), None, None) => {
            let (dn, de) = (pn - sn, pe - se);
            (dn * tn + de * te, dn * rn + de * re, pn, pe)
        }
        (None, None, Some(st), off) => {
            let (a, o) = (st - s0, off.unwrap_or(0.0));
            (a, o, sn + a * tn + o * rn, se + a * te + o * re)
        }
        (None, None, None, _) => {
            return Err(ToolError::invalid(
                "/northing",
                "Give a point to station, or a station (and offset) to set out.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/station",
                "Give a point, or a station and offset, not both.",
            ));
        }
    };
    let q = |x: f64| Q { value: x, unit: u };
    let side = if off > 0.0 {
        "right"
    } else if off < 0.0 {
        "left"
    } else {
        "on the line"
    };
    Ok(Json::obj(vec![
        ("station", Json::Str(fmt_station(s0 + along, is_metric(u)))),
        ("offset", ctx.emit("offset", q(off), u)),
        ("side", Json::Str(side.into())),
        (
            "foot_northing",
            ctx.emit("foot_northing", q(sn + along * tn), u),
        ),
        (
            "foot_easting",
            ctx.emit("foot_easting", q(se + along * te), u),
        ),
        ("northing", ctx.emit("northing", q(pn), u)),
        ("easting", ctx.emit("easting", q(pe), u)),
    ]))
}
