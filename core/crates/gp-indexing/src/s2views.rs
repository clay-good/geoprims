//! Focused S2 and map-tile operations (add-spatial-indexing-and-raster, the
//! `s2` and `tile` rows of the design inventory), on the same cell and tile
//! helpers as the general S2 inspector and tile tools.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};

use crate::codes;
use crate::s2;
use crate::s2tools::{CELL_FIELD, CELL_INFO, CELL_OUT, EARTH_RADIUS_M, read_cell};
use crate::{TILE_BOUNDS, parse_tile};

const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];

macro_rules! view {
    ($name:ident, $id:literal, $run:ident, $title:literal, $summary:literal,
     $aliases:expr, $keywords:expr, $inputs:expr, $outputs:expr, $refs:expr,
     $when:literal, $limits:literal, $ex_title:literal, $ex:literal, $related:expr,
     $sentence:literal) => {
        pub static $name: ToolDef = ToolDef {
            id: $id,
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: $inputs,
            outputs: $outputs,
            errors: &[],
            warnings: &["EXPERIMENTAL_TOOL"],
            model: "S2 geometry and Web Mercator tile arithmetic, as the general S2 and tile tools compute them",
            accuracy: "Exact for indexes and tokens; S2 areas to the precision of the cell's spherical polygon",
            when_to_use: $when,
            limitations: $limits,
            references: $refs,
            examples: &[Example {
                id: "primary",
                title: $ex_title,
                input: $ex,
                source: "S2 and slippy-map conventions, computed by the core",
            }],
            primary_example: "primary",
            visualization: TABLE,
            related: $related,
            sentence: $sentence,
            limits: &[("batchRows", 10_000)],
            run: $run,
            ..ToolDef::BLANK
        };
    };
}

macro_rules! rel {
    ($($id:literal => $why:literal),* $(,)?) => {
        &[$(Related { id: $id, reason: $why }),*]
    };
}

view!(
    S2_TOKEN,
    "indexing.s2.token",
    run_s2_token,
    "S2 token and cell id",
    "Converts an S2 cell between its compact hexadecimal token and its 64-bit decimal cell id, with its level and face.",
    &[
        "S2 token to cell id",
        "S2 cell id to token",
        "S2 token converter"
    ],
    &["S2", "token", "cell id", "64-bit", "hex", "decimal"],
    &[CELL_FIELD.required().core()],
    &[CELL_OUT[0], CELL_OUT[1], CELL_OUT[2], CELL_OUT[3]],
    CELL_INFO.references,
    "Use this when S2 cells arrive in one form and a system wants the other: BigQuery and many APIs use the 64-bit decimal id, while URLs, logs, and the S2 libraries' debug output use the short hexadecimal token. Paste either. The tool returns both, with the cell's level and cube face, so you can check they name the same cell.",
    "Tokens drop trailing zeros, so they vary in length with the level; leading zeros are not dropped. A decimal id longer than 16 digits is read as an id and anything shorter as a token, since a short decimal number is also a valid hexadecimal token. Signed 64-bit stores may show the same id as a negative number.",
    "A level 13 cell over San Francisco",
    r#"{"cell":"8085809c"}"#,
    rel!("indexing.s2.cell-info" => "parent", "indexing.s2.lat-lng-to-cell" => "alternative", "indexing.s2.hierarchy" => "next"),
    "The token {cell} is cell id {cell_id}, at level {level}."
);

/// The token, decimal id, level, and face: the start of the inspector's
/// `cell_json`, without the center a focused operation does not declare.
fn identity(cell: s2::CellId) -> Vec<(&'static str, Json)> {
    vec![
        ("cell", Json::str(cell.token())),
        ("cell_id", Json::str(cell.0.to_string())),
        ("level", Json::Num(f64::from(cell.level()))),
        ("face", Json::Num(f64::from(cell.face()))),
    ]
}

fn run_s2_token(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    Ok(Json::obj(identity(cell)))
}

view!(
    S2_VERTICES,
    "indexing.s2.vertices",
    run_s2_vertices,
    "S2 cell vertices",
    "The four corners of an S2 cell as latitude and longitude, counterclockwise, ready to draw the cell.",
    &["S2 cell corners", "S2 cell polygon", "S2 vertices"],
    &["S2", "vertices", "corners", "polygon", "boundary", "quad"],
    &[CELL_FIELD.required().core()],
    &[CELL_OUT[0], CELL_INFO.outputs[8]],
    CELL_INFO.references,
    "Use this to draw an S2 cell or clip data to it. Paste the token or cell id. The tool returns the cell's four corners counterclockwise as latitude and longitude. S2 cells are quadrilaterals on the sphere whose edges are great-circle arcs, so the four corners define the cell exactly.",
    "The edges between the corners are great-circle arcs, not lines of latitude or longitude, so a large cell drawn as a straight-edged polygon on a flat map is slightly off. The ring is not closed: repeat the first corner if your format needs it. Cells near the cube's corners are noticeably skewed.",
    "A level 10 cell over Paris",
    r#"{"cell":"47e66f"}"#,
    rel!("indexing.s2.cell-info" => "parent", "indexing.s2.area" => "next", "indexing.s2.neighbors" => "alternative"),
    "The four corners of {cell} are listed counterclockwise."
);

fn run_s2_vertices(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    let boundary: Vec<Json> = cell
        .vertices()
        .iter()
        .map(|&(la, lo)| {
            Json::obj([
                ("lat", crate::s2tools::deg(la).to_json()),
                ("lon", crate::s2tools::deg(lo).to_json()),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("cell", Json::str(cell.token())),
        ("boundary", Json::Arr(boundary)),
    ]))
}

view!(
    S2_HIERARCHY,
    "indexing.s2.hierarchy",
    run_s2_hierarchy,
    "S2 cell hierarchy",
    "An S2 cell's parent, its ancestor at any coarser level, and its four children.",
    &["S2 parent cell", "S2 children", "S2 ancestor at level"],
    &["S2", "parent", "children", "ancestor", "level", "Hilbert"],
    &[
        CELL_FIELD.required().core(),
        Field::new(
            "level",
            "Ancestor level",
            "Optional: a coarser level for the ancestor, like 8",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .core(),
    ],
    &[
        CELL_OUT[0],
        CELL_OUT[2],
        CELL_INFO.outputs[9],
        Field::new(
            "ancestor",
            "Ancestor",
            "At the level asked for",
            Kind::Text { max_len: 20 },
        )
        .optional(),
        CELL_INFO.outputs[10],
    ],
    CELL_INFO.references,
    "Use this to roll S2 data up or down a level: aggregate fine cells into their parents, look up the coarse cell a fine one belongs to, or split a cell into its four children. Paste the token or cell id, and optionally a coarser level for its ancestor there. The tool returns the parent one level up, the ancestor, and the four children in Hilbert order.",
    "An S2 cell has exactly one parent and four children, except at level 0, which has no parent, and level 30, which has no children. The children are listed in Hilbert-curve order, not by position. The ancestor level must not be finer than the cell's own.",
    "A level 12 cell's ancestor at level 6",
    r#"{"cell":"89c25a3","level":6}"#,
    rel!("indexing.s2.cell-info" => "parent", "indexing.s2.token" => "alternative", "indexing.s2.covering" => "next"),
    "{cell} is a level {level} cell."
);

fn run_s2_hierarchy(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    let mut out: Vec<_> = identity(cell)
        .into_iter()
        .filter(|(k, _)| matches!(*k, "cell" | "level"))
        .collect();
    if cell.level() > 0 {
        out.push(("parent", Json::str(cell.parent(cell.level() - 1).token())));
    }
    if let Some(level) = ctx.number("level")? {
        if level.fract() != 0.0 || level > f64::from(cell.level()) {
            return Err(ToolError::invalid(
                "/level",
                format!(
                    "The ancestor is coarser: pick a whole level from 0 to {}.",
                    cell.level()
                ),
            ));
        }
        out.push(("ancestor", Json::str(cell.parent(level as u8).token())));
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

view!(
    S2_AREA,
    "indexing.s2.area",
    run_s2_area,
    "S2 cell area",
    "The exact area of an S2 cell on a sphere of the Earth's mean radius, with the average for its level and the ratio between them.",
    &[
        "S2 cell area calculator",
        "area of an S2 cell",
        "S2 cell size"
    ],
    &["S2", "area", "m²", "level", "density", "normalize"],
    &[CELL_FIELD.required().core()],
    &[
        CELL_INFO.outputs[6],
        CELL_INFO.outputs[7],
        Field::new(
            "ratio",
            "Share of the average",
            "This cell's area ÷ the level's average",
            Kind::Number {
                min: 0.0,
                max: 10.0
            },
        )
        .precision(Precision::Decimals(4)),
    ],
    CELL_INFO.references,
    "Use this to turn counts per S2 cell into densities, or to see how much one cell departs from its level's average. Paste the token or cell id. The tool returns the cell's exact area, the average area of cells at that level, and the ratio; cells near the cube's corners are up to about half again smaller or larger than the average.",
    "The area is on a sphere of radius 6,371,008.8 m, not the WGS 84 ellipsoid, so it can differ from an ellipsoidal area of the same outline by a few tenths of a percent. The average is the sphere's area divided by the number of cells at the level. For many cells, sum their own areas rather than using one average.",
    "A level 15 cell over Sydney",
    r#"{"cell":"6b12ae3fc"}"#,
    rel!("indexing.s2.cell-info" => "parent", "indexing.s2.vertices" => "alternative", "indexing.h3.cell-area" => "alternative"),
    "The cell covers {area}, {ratio} of its level's average."
);

fn run_s2_area(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = read_cell(ctx, "cell")?;
    let r2 = EARTH_RADIUS_M * EARTH_RADIUS_M;
    let (area, average) = (
        cell.area_steradians() * r2,
        s2::average_area_steradians(cell.level()) * r2,
    );
    let m2 = |v: f64| gp_base::tool::Q {
        value: v,
        unit: gp_base::units::by_symbol(gp_base::units::Quantity::Area, "m2").expect("m2"),
    };
    Ok(Json::obj([
        ("area", ctx.out("area", m2(area))),
        ("average_area", ctx.out("average_area", m2(average))),
        ("ratio", Json::Num(area / average)),
    ]))
}

const TILE_CONVENTION: Field = Field::new(
    "convention",
    "y convention of the input",
    "xyz (default; y counts from the north) or tms (y counts from the south)",
    Kind::Choice(&["xyz", "tms"]),
)
.core();

const fn tile_text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 40 })
}

view!(
    XYZ_TMS,
    "indexing.tile.xyz-tms",
    run_xyz_tms,
    "XYZ and TMS tile numbering",
    "Converts a map tile's z/x/y between the XYZ scheme, which counts rows from the north, and TMS, which counts them from the south.",
    &["XYZ to TMS", "TMS to XYZ", "flip tile y"],
    &["XYZ", "TMS", "slippy map", "tile row", "flip y", "MBTiles"],
    &[TILE_BOUNDS.inputs[0], TILE_CONVENTION],
    &[
        tile_text("converted", "Converted tile", "In the other numbering"),
        tile_text("xyz", "XYZ", "z/x/y counting rows from the north"),
        tile_text("tms", "TMS", "z/x/y counting rows from the south"),
    ],
    TILE_BOUNDS.references,
    "Use this when tiles move between systems that number rows differently: web maps and most tile servers use XYZ, while MBTiles files and older TMS servers count from the south. Enter the tile as zoom/x/y and say which scheme it is in. The tool returns it in the other scheme, and both forms side by side. Only the row number changes.",
    "The conversion flips the row within the zoom level, y' = 2^zoom − 1 − y, so the zoom must be right for the answer to be. It cannot tell which scheme a tile was written in; a tile that looks wrong on the map is often in the other one, which the tile bounds tool can show. A quadkey has no such ambiguity.",
    "An XYZ tile over Pittsburgh at zoom 14",
    r#"{"tile":"14/4551/6176","convention":"xyz"}"#,
    rel!("indexing.tile.bounds" => "parent", "indexing.tile.tile-quadkey" => "alternative", "indexing.tile.from-point" => "alternative"),
    "That tile is {converted}."
);

fn run_xyz_tms(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (z, x, y_in, is_quadkey) = parse_tile(&ctx.text("tile")?.expect("required"))?;
    let tms_in = ctx.choice("convention")? == Some("tms") && !is_quadkey;
    let n = 1u64 << z;
    let y = if tms_in { n - 1 - y_in } else { y_in };
    let (xyz, tms) = (format!("{z}/{x}/{y}"), format!("{z}/{x}/{}", n - 1 - y));
    let converted = if tms_in {
        format!("XYZ {xyz}")
    } else {
        format!("TMS {tms}")
    };
    Ok(Json::obj([
        ("converted", Json::str(converted)),
        ("xyz", Json::str(xyz)),
        ("tms", Json::str(tms)),
    ]))
}

view!(
    TILE_QUADKEY,
    "indexing.tile.tile-quadkey",
    run_tile_quadkey,
    "Map tile to quadkey and back",
    "Converts a map tile between zoom/x/y and the Bing Maps quadkey, a string of one digit per zoom level.",
    &["tile to quadkey", "quadkey to tile", "quadkey converter"],
    &["quadkey", "Bing Maps", "tile", "z/x/y", "zoom", "QuadTree"],
    &[TILE_BOUNDS.inputs[0], TILE_CONVENTION],
    &[
        tile_text("quadkey", "Quadkey", "One digit 0 to 3 per zoom level"),
        tile_text("xyz", "XYZ", "z/x/y counting rows from the north"),
        Field::new(
            "zoom",
            "Zoom",
            "The quadkey's length",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    TILE_BOUNDS.references,
    "Use this with Bing Maps, Azure Maps, or any system that keys tiles by quadkey, or to get a sortable key in which a tile's quadkey begins with its parent's. Enter the tile as zoom/x/y, or paste a quadkey to get the tile back. The tool returns both forms and the zoom, which is the quadkey's length.",
    "Quadkeys always count rows from the north, so a TMS tile is flipped first; say which scheme the tile is in. Zoom 0 has an empty quadkey. Digits interleave the column and row bits, so neighboring tiles can have very different quadkeys at tile edges.",
    "A zoom 13 tile over Pittsburgh",
    r#"{"tile":"13/2275/3088"}"#,
    rel!("indexing.tile.bounds" => "parent", "indexing.tile.xyz-tms" => "alternative", "indexing.tile.family" => "next"),
    "The quadkey is {quadkey}, for tile {xyz}."
);

fn run_tile_quadkey(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (z, x, y_in, is_quadkey) = parse_tile(&ctx.text("tile")?.expect("required"))?;
    let n = 1u64 << z;
    let y = if ctx.choice("convention")? == Some("tms") && !is_quadkey {
        n - 1 - y_in
    } else {
        y_in
    };
    Ok(Json::obj([
        ("quadkey", Json::str(codes::quadkey(z, x, y))),
        ("xyz", Json::str(format!("{z}/{x}/{y}"))),
        ("zoom", Json::Num(f64::from(z))),
    ]))
}
