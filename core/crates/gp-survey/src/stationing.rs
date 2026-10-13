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
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 11 (coordinate geometry: perpendicular distance and station-offset)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const START_NORTHING: Field = len("start_northing", "Line start northing", "Like 5000.00 ft")
    .required()
    .core();
const START_EASTING: Field = len("start_easting", "Line start easting", "Like 5000.00 ft")
    .required()
    .core();
const START_STATION: Field = Field::new(
    "start_station",
    "Start station",
    "Like 10+00 (default 0+00)",
    Kind::Text { max_len: 16 },
);
const LINE_DIRECTION: Field = Field::new(
    "direction",
    "Line direction",
    "Bearing or azimuth of stationing, like N 60°00'00\" E",
    Kind::Text { max_len: 32 },
)
.core();
const END_NORTHING: Field = len(
    "end_northing",
    "Line end northing",
    "Instead of a direction, like 5500.00 ft",
);
const END_EASTING: Field = len(
    "end_easting",
    "Line end easting",
    "Instead of a direction, like 5866.03 ft",
);
const POINT_NORTHING: Field = len(
    "northing",
    "Point northing",
    "To find its station and offset, like 5300.00 ft",
)
.core();
const POINT_EASTING: Field = len("easting", "Point easting", "Like 5400.00 ft").core();
const TARGET_STATION: Field = Field::new(
    "station",
    "Station",
    "The station to set out, like 13+50",
    Kind::Text { max_len: 16 },
);
const OFFSET: Field = len(
    "offset",
    "Offset",
    "Right positive, left negative, like -12.5 ft",
);

const STATION_OFFSET_INPUTS: &[Field] = &[
    START_NORTHING,
    START_EASTING,
    START_STATION,
    LINE_DIRECTION,
    END_NORTHING,
    END_EASTING,
    POINT_NORTHING,
    POINT_EASTING,
    TARGET_STATION,
    OFFSET,
];
const PERPENDICULAR_FOOT_INPUTS: &[Field] = &[
    START_NORTHING,
    START_EASTING,
    START_STATION,
    LINE_DIRECTION,
    END_NORTHING,
    END_EASTING,
    POINT_NORTHING.required(),
    POINT_EASTING.required(),
];
const POINT_FROM_STATION_INPUTS: &[Field] = &[
    START_NORTHING,
    START_EASTING,
    START_STATION,
    LINE_DIRECTION,
    END_NORTHING,
    END_EASTING,
    TARGET_STATION.required().core(),
    OFFSET.required(),
];

pub static STATION_OFFSET: ToolDef = ToolDef {
    id: "survey.cogo.station-offset",
    title: "Station and offset from a line",
    summary: "A point's station and offset from a straight alignment, with the foot of its perpendicular, or the point at a given station and offset. Offsets are positive to the right.",
    aliases: &["station offset", "station offset calculator"],
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
    inputs: STATION_OFFSET_INPUTS,
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
    model: "With the line's unit direction t = (cos α, sin α) in (N, E): station = start + (P − S)·t, offset = (P − S) × t positive to the right; and back, P = S + along·t + offset·n with n the right-hand normal (Ghilani & Wolf 2021, ch. 11)",
    accuracy: "Exact",
    when_to_use: "Use this when you need to tie a point to a straight baseline or centerline by station and offset, or set out a point from a station and offset. Give the line's start point and station, plus a direction or an end point. Then enter a point to get its station, its offset right or left, and the foot of the perpendicular, or enter a station and offset to get coordinates. It suits as-built checks, utility locates, and staking off a tangent.",
    limitations: "The line is one straight tangent. It does not follow curves, spirals, or a full alignment with many PIs, so a point beside a curve needs the curve tools. Use plane grid values, not latitude and longitude. A point before the start or past the end still gets a station by extending the line, so check that it falls where you expect.",
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

macro_rules! station_method {
    ($name:ident, $run:ident, $id:literal, $title:literal, $summary:literal,
     $aliases:expr, $keywords:expr, $inputs:expr, $model:literal, $when:literal,
     $limitations:literal, $example_title:literal, $example:literal, $source:literal,
     $sentence:literal, $related:expr) => {
        pub static $name: ToolDef = ToolDef {
            id: concat!("survey.cogo.", $id),
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: $inputs,
            outputs: STATION_OFFSET.outputs,
            errors: STATION_OFFSET.errors,
            warnings: STATION_OFFSET.warnings,
            model: $model,
            accuracy: STATION_OFFSET.accuracy,
            when_to_use: $when,
            limitations: $limitations,
            references: STATION_OFFSET.references,
            examples: &[Example {
                id: "primary",
                title: $example_title,
                input: $example,
                source: $source,
            }],
            primary_example: "primary",
            visualization: STATION_OFFSET.visualization,
            related: $related,
            sentence: $sentence,
            limits: STATION_OFFSET.limits,
            run: $run,
            ..ToolDef::BLANK
        };
    };
}

station_method!(
    PERPENDICULAR_FOOT,
    run_perpendicular_foot,
    "perpendicular-foot",
    "Perpendicular foot and station",
    "Projects a point onto a straight alignment and reports the foot, station, signed offset, and side.",
    &[
        "foot of perpendicular",
        "perpendicular offset",
        "point to line station offset"
    ],
    &[
        "COGO",
        "perpendicular",
        "foot",
        "station",
        "offset",
        "alignment"
    ],
    PERPENDICULAR_FOOT_INPUTS,
    "Orthogonal projection onto a plane line: along = (P − S)·t and signed offset = (P − S)·n, with n to the right (Ghilani & Wolf 2021, ch. 11)",
    "Use this to tie a surveyed point to a straight centerline or baseline. Enter the line start and its station, define the direction by bearing or a second point, and enter the point to project. The result gives the station, signed right-or-left offset, and coordinates of the perpendicular foot on the line for as-built checks, utility ties, and tangent staking.",
    "The alignment is one infinite straight line, so points before its start or beyond a supplied end still project onto its extension. It does not follow circular curves, spirals, or a multi-segment route. Coordinates must be on one plane grid or local system. The sign is positive to the right while facing in the direction of increasing station.",
    "Point beside a north line",
    r#"{"start_northing":"0 ft","start_easting":"0 ft","start_station":"0+00","direction":"0","northing":"50 ft","easting":"-20 ft"}"#,
    "add-survey-suite station-offset geometry",
    "The perpendicular foot is at station {station}; the point is {offset} from the line ({side}).",
    &[
        Related {
            id: "survey.cogo.station-offset",
            reason: "parent"
        },
        Related {
            id: "survey.cogo.point-from-station-offset",
            reason: "inverse"
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "alternative"
        },
    ]
);

station_method!(
    POINT_FROM_STATION_OFFSET,
    run_point_from_station_offset,
    "point-from-station-offset",
    "Point from station and offset",
    "Computes plane coordinates at a station and signed offset from a straight alignment.",
    &[
        "point from station and offset",
        "station offset to coordinates",
        "stake point from baseline"
    ],
    &[
        "COGO",
        "station",
        "offset",
        "coordinates",
        "stakeout",
        "alignment"
    ],
    POINT_FROM_STATION_INPUTS,
    "Plane tangent setout: P = S + along·t + offset·n, with along = target station − start station and n to the right (Ghilani & Wolf 2021, ch. 11)",
    "Use this to compute the coordinate of a stakeout or design point described by station and offset from a straight centerline or baseline. Define the line from its start and a bearing or second point, state the starting station, then enter the target station and signed offset. Positive offsets are right and negative offsets are left while facing ahead on stationing.",
    "The calculation extends one straight line without clipping to a supplied end point. It does not place points along circular curves, spirals, or a full route alignment. All coordinates and offsets must use compatible plane units, and the station notation must match the project's metric or US stationing convention. Check the sign convention before staking the result.",
    "Point 12.5 ft left at station 13+50",
    r#"{"start_northing":"5000 ft","start_easting":"5000 ft","start_station":"10+00","direction":"60","station":"13+50","offset":"-12.5 ft"}"#,
    "add-survey-suite station-offset geometry",
    "The station-offset point is northing {northing}, easting {easting} ({side}).",
    &[
        Related {
            id: "survey.cogo.station-offset",
            reason: "parent"
        },
        Related {
            id: "survey.cogo.perpendicular-foot",
            reason: "inverse"
        },
        Related {
            id: "survey.cogo.forward",
            reason: "alternative"
        },
    ]
);

#[derive(Clone, Copy)]
enum StationMode {
    PerpendicularFoot,
    PointFromStationOffset,
}

fn run_station_offset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_station_offset_with(ctx, None)
}

fn run_perpendicular_foot(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_station_offset_with(ctx, Some(StationMode::PerpendicularFoot))
}

fn run_point_from_station_offset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    run_station_offset_with(ctx, Some(StationMode::PointFromStationOffset))
}

fn run_station_offset_with(ctx: &mut Ctx, mode: Option<StationMode>) -> Result<Json, ToolError> {
    let start_northing = ctx.req_quantity("start_northing")?;
    let start_easting = ctx.req_quantity("start_easting")?;
    let end_northing = ctx.quantity("end_northing")?;
    let end_easting = ctx.quantity("end_easting")?;
    let (northing, easting) = match mode {
        Some(StationMode::PointFromStationOffset) => (None, None),
        _ => (ctx.quantity("northing")?, ctx.quantity("easting")?),
    };
    let offset = match mode {
        Some(StationMode::PerpendicularFoot) => None,
        _ => ctx.quantity("offset")?,
    };
    let named = [
        ("start_northing", Some(start_northing)),
        ("start_easting", Some(start_easting)),
        ("end_northing", end_northing),
        ("end_easting", end_easting),
        ("northing", northing),
        ("easting", easting),
        ("offset", offset),
    ];
    let qs: Vec<(&str, Q)> = named
        .iter()
        .filter_map(|(name, value)| value.map(|value| (*name, value)))
        .collect();
    let u = common_unit(&qs)?;
    let (sn, se) = (start_northing.to(u), start_easting.to(u));
    let (en, ee) = (end_northing.map(|q| q.to(u)), end_easting.map(|q| q.to(u)));
    let (point_northing, point_easting) = (northing.map(|q| q.to(u)), easting.map(|q| q.to(u)));
    let offset = offset.map(|q| q.to(u));
    let s0 = station(ctx, "start_station")?.unwrap_or(0.0);
    let dir = ctx.text("direction")?;
    let az = match (dir, en, ee) {
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
    let target_station = match mode {
        Some(StationMode::PerpendicularFoot) => None,
        _ => station(ctx, "station")?,
    };
    let (along, off, pn, pe) = match (point_northing, point_easting, target_station, offset) {
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
