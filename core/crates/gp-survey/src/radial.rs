//! Multiple radial sideshots from one occupied station. Each point uses the
//! same plane forward computation as `survey.cogo.forward`; only the row
//! handling and output table are different.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::{GHILANI, common_unit, deg, direction, forward_point, len, len_out};

const SHOT: &[Field] = &[
    Field::new(
        "point",
        "Point",
        "Point name or number, like CP 12",
        Kind::Text { max_len: 40 },
    )
    .required(),
    Field::new(
        "direction",
        "Bearing or azimuth",
        "Direction from the occupied station, like N 35°15'20\" E",
        Kind::Text { max_len: 32 },
    )
    .required(),
    len(
        "distance",
        "Horizontal distance",
        "From the occupied station, like 185.42 ft",
    )
    .required(),
];

const POINT: &[Field] = &[
    Field::new(
        "point",
        "Point",
        "The entered point name",
        Kind::Text { max_len: 40 },
    ),
    Field::new(
        "bearing",
        "Bearing",
        "Quadrant bearing from the occupied station",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "azimuth",
        "Azimuth",
        "North azimuth from the occupied station",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(8))
    .angle_range("[0,360)"),
    len_out(
        "distance",
        "Distance",
        "Horizontal distance from the occupied station",
    ),
    len_out("northing", "Northing", "Calculated point northing"),
    len_out("easting", "Easting", "Calculated point easting"),
];

pub static SIDESHOTS: ToolDef = ToolDef {
    id: "survey.cogo.radial-sideshots",
    title: "Radial sideshots from one station",
    summary: "Calculates coordinates for a list of points observed by bearing or azimuth and horizontal distance from one occupied station.",
    aliases: &[
        "radial survey calculator",
        "multiple sideshots",
        "radial stakeout",
        "total station sideshots",
    ],
    keywords: &[
        "COGO",
        "radial",
        "sideshot",
        "occupied station",
        "bearing",
        "azimuth",
        "coordinates",
    ],
    inputs: &[
        len(
            "station_northing",
            "Station northing",
            "Occupied-station northing, like 5000.000 ft",
        )
        .required()
        .core(),
        len(
            "station_easting",
            "Station easting",
            "Occupied-station easting, like 10000.000 ft",
        )
        .required()
        .core(),
        Field::new(
            "shots",
            "Radial sideshots",
            "One point, direction, and horizontal distance per row, like CP 12, N 35°15'20\" E, 185.42 ft",
            Kind::List {
                items: SHOT,
                min: 1,
                max: 1_000,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "count",
            "Points calculated",
            "Number of radial sideshots",
            Kind::Number {
                min: 1.0,
                max: 1_000.0,
            },
        )
        .precision(Precision::Decimals(0)),
        len_out(
            "station_northing",
            "Station northing",
            "Occupied-station northing",
        ),
        len_out(
            "station_easting",
            "Station easting",
            "Occupied-station easting",
        ),
        Field::new(
            "points",
            "Calculated points",
            "One coordinate pair for each radial sideshot",
            Kind::List {
                items: POINT,
                min: 1,
                max: 1_000,
            },
        ),
    ],
    errors: &[ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    when_to_use: "Use this after occupying a known plane-grid point and measuring several independent points from that setup. Enter the station coordinates once, then give each point name, grid bearing or north azimuth, and horizontal grid distance. The tool calculates every coordinate in one table, which is useful for topographic pickup, control ties, construction layout checks, and hand reduction of a total-station field book.",
    limitations: "Every row is independent and starts at the same occupied station; this is not a traverse that carries coordinates from one point to the next. Distances must already be horizontal and on the grid. Reduce slope distances and apply the combined factor first when needed. All coordinates and distances must use one linear unit, and every direction must use the same grid or assumed north reference.",
    model: "For every row, N = Nstation + distance·cos(azimuth) and E = Estation + distance·sin(azimuth)",
    accuracy: "Exact for the plane coordinates, directions, and horizontal distances entered",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Three points from one occupied station",
        input: r#"{"station_northing":"5000 ft","station_easting":"10000 ft","shots":[{"point":"CP 12","direction":"N 35°15'20\" E","distance":"185.42 ft"},{"point":"EP 4","direction":"S 70°00'00\" E","distance":"240.00 ft"},{"point":"TBM","direction":"270","distance":"95.50 ft"}]}"#,
        source: "add-survey-suite radial-sideshots requirement",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.forward",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "next",
        },
    ],
    sentence: "Calculated {count} radial points from northing {station_northing}, easting {station_easting}.",
    limits: &[("batchRows", 1_000)],
    run,
    ..ToolDef::BLANK
};

fn run(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let station_northing = ctx.req_quantity("station_northing")?;
    let station_easting = ctx.req_quantity("station_easting")?;
    let rows = ctx.rows("shots")?;
    let mut shots = Vec::with_capacity(rows.len());
    let mut tagged = vec![
        ("/station_northing".to_owned(), station_northing),
        ("/station_easting".to_owned(), station_easting),
    ];
    for (i, row) in rows.iter().enumerate() {
        let point = ctx.row_text("shots", i, row, "point")?.expect("required");
        let text = ctx
            .row_text("shots", i, row, "direction")?
            .expect("required");
        let azimuth = direction::parse(&text)
            .map_err(|message| ToolError::invalid(&format!("/shots/{i}/direction"), message))?;
        let distance = ctx
            .row_quantity("shots", i, row, "distance")?
            .expect("required");
        tagged.push((format!("/shots/{i}/distance"), distance));
        shots.push((point, azimuth, distance));
    }
    let refs: Vec<(&str, Q)> = tagged
        .iter()
        .map(|(field, value)| (field.as_str(), *value))
        .collect();
    let unit = common_unit(&refs)?;
    let northing = station_northing.to(unit);
    let easting = station_easting.to(unit);
    let q = |value: f64| Q { value, unit };
    let mut points = Vec::with_capacity(shots.len());
    for (i, (point, azimuth, distance)) in shots.into_iter().enumerate() {
        let distance = distance.to(unit);
        if distance <= 0.0 {
            return Err(ToolError::invalid(
                &format!("/shots/{i}/distance"),
                "A radial sideshot needs a positive horizontal distance.",
            ));
        }
        let (point_northing, point_easting) = forward_point(northing, easting, azimuth, distance);
        points.push(Json::obj([
            ("point", Json::str(&point)),
            ("bearing", Json::str(direction::bearing(azimuth, 2))),
            (
                "azimuth",
                ctx.emit("azimuth", deg(azimuth), crate::unit(QT::Angle, "deg")),
            ),
            ("distance", ctx.emit("distance", q(distance), unit)),
            ("northing", ctx.emit("northing", q(point_northing), unit)),
            ("easting", ctx.emit("easting", q(point_easting), unit)),
        ]));
    }
    Ok(Json::obj([
        ("count", Json::Num(points.len() as f64)),
        (
            "station_northing",
            ctx.emit("station_northing", q(northing), unit),
        ),
        (
            "station_easting",
            ctx.emit("station_easting", q(easting), unit),
        ),
        ("points", Json::Arr(points)),
    ]))
}
