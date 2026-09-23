//! The S2 tools (indexing/hierarchical-cells, "S2 cells"): a point to a cell,
//! a cell to everything about itself, and the cells around one.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;

use crate::s2::{self, CellId};

pub const S2_DOCS: Reference = Reference {
    title: "S2 Geometry",
    issuer: "Google",
    year: 2025,
    edition: "S2 Geometry library",
    locator: "Cell ids, the quadratic projection, the Hilbert curve, tokens, and cell hierarchy",
    url: "https://s2geometry.io/devguide/s2cell_hierarchy",
};

const EARTH_RADIUS_M: f64 = 6_371_008.8;

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

fn read_level(ctx: &mut Ctx, name: &str) -> Result<u8, ToolError> {
    let level = ctx.number(name)?.expect("required");
    if !(0.0..=30.0).contains(&level) || level.fract() != 0.0 {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            "An S2 level is a whole number from 0 to 30.",
        ));
    }
    Ok(level as u8)
}

/// Reads a cell from a token or a decimal id, which is how S2 ids travel.
fn read_cell(ctx: &mut Ctx, name: &str) -> Result<CellId, ToolError> {
    let text = ctx.text(name)?.expect("required");
    let t = text.trim();
    let cell = if t.chars().all(|c| c.is_ascii_digit()) && t.len() > 16 {
        let id: u64 = t.parse().map_err(|_| {
            ToolError::invalid(&format!("/{name}"), "That is not a 64-bit cell id.")
        })?;
        CellId(id)
    } else {
        CellId::from_token(t)
            .map_err(|e| ToolError::invalid(&format!("/{name}"), format!("{e}.")))?
    };
    if !cell.is_valid() {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("{t} is not a valid S2 cell: its face or level bits are wrong."),
        ));
    }
    Ok(cell)
}

fn cell_json(ctx: &mut Ctx, cell: CellId) -> Vec<(&'static str, Json)> {
    let (lat, lon) = cell.center();
    vec![
        ("cell", Json::str(cell.token())),
        ("cell_id", Json::str(cell.0.to_string())),
        ("level", Json::Num(f64::from(cell.level()))),
        ("face", Json::Num(f64::from(cell.face()))),
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
    ]
}

fn run_point_to_cell(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let level = read_level(ctx, "level")?;
    let cell = CellId::from_lat_lon(lat, lon, level);
    let mut out = cell_json(ctx, cell);
    out.push((
        "area",
        ctx.out(
            "area",
            Q {
                value: cell.area_steradians() * EARTH_RADIUS_M * EARTH_RADIUS_M,
                unit: units::by_symbol(QT::Area, "m2").expect("m2"),
            },
        ),
    ));
    Ok(Json::obj(out))
}

fn run_cell_info(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    let mut out = cell_json(ctx, cell);
    let area_m2 = cell.area_steradians() * EARTH_RADIUS_M * EARTH_RADIUS_M;
    let average = s2::average_area_steradians(cell.level()) * EARTH_RADIUS_M * EARTH_RADIUS_M;
    out.push((
        "area",
        ctx.out(
            "area",
            Q {
                value: area_m2,
                unit: units::by_symbol(QT::Area, "m2").expect("m2"),
            },
        ),
    ));
    out.push((
        "average_area",
        ctx.out(
            "average_area",
            Q {
                value: average,
                unit: units::by_symbol(QT::Area, "m2").expect("m2"),
            },
        ),
    ));
    out.push((
        "boundary",
        Json::Arr(
            cell.vertices()
                .iter()
                .map(|&(la, lo)| {
                    Json::obj([("lat", deg(la).to_json()), ("lon", deg(lo).to_json())])
                })
                .collect(),
        ),
    ));
    if cell.level() > 0 {
        out.push(("parent", Json::str(cell.parent(cell.level() - 1).token())));
    }
    if let Some(children) = cell.children() {
        out.push((
            "children",
            Json::Arr(
                children
                    .iter()
                    .map(|c| Json::obj([("cell", Json::str(c.token()))]))
                    .collect(),
            ),
        ));
    }
    Ok(Json::obj(out))
}

fn run_neighbors(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    let neighbors = cell.edge_neighbors();
    let rows: Vec<Json> = neighbors
        .iter()
        .zip(["down", "right", "up", "left"])
        .map(|(n, side)| {
            let (lat, lon) = n.center();
            Json::obj([
                ("cell", Json::str(n.token())),
                ("side", Json::str(side)),
                ("lat", deg(lat).to_json()),
                ("lon", deg(lon).to_json()),
            ])
        })
        .collect();
    let crossing = neighbors.iter().filter(|n| n.face() != cell.face()).count();
    Ok(Json::obj([
        ("cell", Json::str(cell.token())),
        ("level", Json::Num(f64::from(cell.level()))),
        ("neighbors", Json::Arr(rows)),
        ("across_a_face_edge", Json::Num(crossing as f64)),
    ]))
}

const CELL_FIELD: Field = Field::new(
    "cell",
    "S2 cell",
    "A token like 8834f3dec, or a 64-bit cell id",
    Kind::Text { max_len: 24 },
);

const BOUNDARY_ROW: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

const NEIGHBOR_ROW: &[Field] = &[
    Field::new("cell", "Cell", "Its token", Kind::Text { max_len: 24 }),
    Field::new(
        "side",
        "Side",
        "Which edge it is across, in the cell's own frame",
        Kind::Text { max_len: 8 },
    ),
    Field::new(
        "lat",
        "Latitude",
        "Of its center, in degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Of its center, in degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

const CELL_OUT: [Field; 6] = [
    Field::new("cell", "S2 cell", "Its token", Kind::Text { max_len: 24 }),
    Field::new(
        "cell_id",
        "Cell id",
        "The 64-bit id, as a string",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "level",
        "Level",
        "0 (a face) to 30 (about a centimeter)",
        Kind::Number {
            min: 0.0,
            max: 30.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "face",
        "Face",
        "Which of the six cube faces, 0 to 5",
        Kind::Number { min: 0.0, max: 5.0 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "lat",
        "Center latitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Center longitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

pub static POINT_TO_CELL: ToolDef = ToolDef {
    id: "indexing.s2.lat-lng-to-cell",
    stability: gp_base::tool::Stability::Stable,
    title: "S2 cell for a point",
    summary: "The S2 cell containing a latitude and longitude at a level from 0 to 30, with its token, its 64-bit id, the face it sits on, and its area.",
    aliases: &[
        "S2 cell id",
        "point to S2",
        "S2 token",
        "lat lng to S2 cell",
    ],
    keywords: &["S2", "cell", "token", "level", "Hilbert", "face", "index"],
    inputs: &[
        point::lat_field("lat", "Latitude").core(),
        point::lon_field("lon", "Longitude").core(),
        Field::new(
            "level",
            "Level",
            "0 for a whole cube face to 30 for about a centimeter, like 15",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        CELL_OUT[0],
        CELL_OUT[1],
        CELL_OUT[2],
        CELL_OUT[3],
        CELL_OUT[4],
        CELL_OUT[5],
        Field::new(
            "area",
            "Cell area",
            "The exact area of this cell",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "The S2 quadratic projection onto a cube face, the Hilbert curve across that face, and the cell id's face, position, and level bits",
    accuracy: "Matches s2sphere, an independent implementation of the same algorithms, on 400 cases across the sphere at every kind of level: identical tokens and neighbours, and geometry within 10 nanometers.",
    when_to_use: "Use this to key data by S2, which is what BigQuery GIS, MongoDB, and a number of mapping pipelines index with: a point becomes a cell at the level you choose, and cells at a level can be joined, counted, or compared. The token is the short form that travels in APIs; the decimal id is what most databases store.",
    limitations: "A cell is an area and the point is somewhere inside it, so a join at too coarse a level hides structure and one at too fine a level scatters it. S2 cells are not equal-area: the quadratic projection holds the variation to about 2.1 to 1 across a face, which is enough to matter when counting per cell. Levels run 0 to 30, and a level 30 cell is about a centimeter, which is finer than most positions are known to.",
    references: &[S2_DOCS],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at level 15",
        input: r#"{"lat":40.446111,"lon":-79.982222,"level":15}"#,
        source: "add-spatial-indexing-and-raster hierarchical-cells scenario: the token is 8834f3dec",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.s2.cell-info",
            reason: "next",
        },
        Related {
            id: "indexing.s2.neighbors",
            reason: "next",
        },
        Related {
            id: "indexing.convert.cross-index",
            reason: "alternative",
        },
    ],
    sentence: "That is S2 cell {cell} at level {level}.",
    limits: &[("batchRows", 10_000)],
    run: run_point_to_cell,
    ..ToolDef::BLANK
};

pub static CELL_INFO: ToolDef = ToolDef {
    id: "indexing.s2.cell-info",
    stability: gp_base::tool::Stability::Stable,
    title: "S2 cell details",
    summary: "Everything about an S2 cell: its center, its four corners, the face and level, its exact area against the average for that level, and its parent and children.",
    aliases: &["S2 cell info", "decode S2 token", "S2 cell boundary"],
    keywords: &[
        "S2", "cell", "boundary", "vertices", "parent", "children", "area", "token",
    ],
    inputs: &[CELL_FIELD.required().core()],
    outputs: &[
        CELL_OUT[0],
        CELL_OUT[1],
        CELL_OUT[2],
        CELL_OUT[3],
        CELL_OUT[4],
        CELL_OUT[5],
        Field::new(
            "area",
            "Cell area",
            "The exact area of this cell",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "average_area",
            "Average for the level",
            "The sphere divided by the cells at this level",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "boundary",
            "Corners",
            "The cell's four corners, counter-clockwise",
            Kind::List {
                items: BOUNDARY_ROW,
                min: 4,
                max: 4,
            },
        ),
        Field::new(
            "parent",
            "Parent",
            "The cell one level up",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "children",
            "Children",
            "The four cells one level down, in Hilbert order",
            Kind::List {
                items: &[Field::new(
                    "cell",
                    "Cell",
                    "Its token",
                    Kind::Text { max_len: 24 },
                )],
                min: 0,
                max: 4,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[],
    model: "The cell id's face, Hilbert position, and level bits, with the corners taken back through the quadratic projection",
    accuracy: "Matches s2sphere on 400 cases: identical hierarchy, and centers and corners within 10 nanometers. The exact area agrees to rounding down to level 20 and to about 2e-7 relative at level 30, where a cell is a centimeter across.",
    when_to_use: "Use this when an S2 token or id arrives and you need to know what it covers: the corners to draw it, the center to plot it, the area to weight it, and the parent and children to move between levels. The average area beside the exact one shows how far this particular cell sits from the level's nominal size.",
    limitations: "The corners are the cell's own, which are not a rectangle in latitude and longitude: S2 cells are quadrilaterals on the sphere, and drawing them as boxes overstates their extent near the face edges. Levels deeper than about 23 are finer than most positions are known to.",
    references: &[S2_DOCS],
    examples: &[Example {
        id: "primary",
        title: "The level 15 cell over Pittsburgh",
        input: r#"{"cell":"8834f3dec"}"#,
        source: "The cell from the token scenario, with its geometry checked against s2sphere",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.s2.lat-lng-to-cell",
            reason: "parent",
        },
        Related {
            id: "indexing.s2.neighbors",
            reason: "next",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "alternative",
        },
    ],
    sentence: "Cell {cell} is at level {level} on face {face}, covering {area}.",
    limits: &[("batchRows", 10_000)],
    run: run_cell_info,
    ..ToolDef::BLANK
};

pub static NEIGHBORS: ToolDef = ToolDef {
    id: "indexing.s2.neighbors",
    stability: gp_base::tool::Stability::Stable,
    title: "S2 edge neighbours",
    summary: "The four cells across an S2 cell's edges, at the same level, wrapping onto the next cube face where the cell lies on a face edge.",
    aliases: &["S2 neighbors", "S2 adjacent cells"],
    keywords: &["S2", "neighbors", "adjacent", "edge", "face", "wrap"],
    inputs: &[CELL_FIELD.required().core()],
    outputs: &[
        CELL_OUT[0],
        CELL_OUT[2],
        Field::new(
            "neighbors",
            "Neighbours",
            "The four cells across this cell's edges",
            Kind::List {
                items: NEIGHBOR_ROW,
                min: 4,
                max: 4,
            },
        ),
        Field::new(
            "across_a_face_edge",
            "On another face",
            "How many of the four lie on a different cube face",
            Kind::Number { min: 0.0, max: 4.0 },
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[],
    model: "The cell's (i, j) stepped by one cell in each direction, taken back through the sphere where it leaves the face, which is how S2 wraps across the cube",
    accuracy: "Matches s2sphere's edge neighbours exactly, in the same order, on 400 cases including cells on face edges.",
    when_to_use: "Use this for anything that has to look past one cell: a proximity search that must not miss points just over a boundary, a flood fill, or a smoothing window over cells. The count of neighbours on another cube face is the flag for the awkward cases, where the grid's axes turn.",
    limitations: "These are the four edge neighbours, not the eight cells touching the corners. Across a face edge the neighbour's own axes are rotated, so stepping further in the same direction is not the same as stepping twice; take neighbours of neighbours rather than adding offsets.",
    references: &[S2_DOCS],
    examples: &[Example {
        id: "primary",
        title: "The four cells around a level 15 cell",
        input: r#"{"cell":"8834f3dec"}"#,
        source: "Checked against s2sphere's get_edge_neighbors, in the same order",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.s2.cell-info",
            reason: "parent",
        },
        Related {
            id: "indexing.s2.lat-lng-to-cell",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.grid-disk",
            reason: "alternative",
        },
    ],
    sentence: "Cell {cell} has four neighbours at level {level}.",
    limits: &[("batchRows", 10_000)],
    run: run_neighbors,
    ..ToolDef::BLANK
};
