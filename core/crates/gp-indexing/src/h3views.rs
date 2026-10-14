//! Focused H3 operations (add-spatial-indexing-and-raster, the `h3` row of the
//! design inventory): each answers one question the H3 API names, through the
//! same h3o calls and helpers as the general inspector, path, children, and
//! edge tools in `h3.rs`.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use h3o::{CellIndex, DirectedEdgeIndex, LatLng};
use serde_json::Value;

use crate::h3::{
    CELL_INFO, H3_DOCS, H3O, VERTEX_ITEM, angle, cell, cell_in, count, deg, hex, local_ij_error,
    pentagon_note, resolution, text,
};

const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];
const WARNINGS: &[&str] = &["PENTAGON_DISTORTION"];
const RESOLUTION_IN: Field = Field::new(
    "resolution",
    "Resolution",
    "A whole number from 0 to 15, like 11",
    Kind::Number {
        min: 0.0,
        max: 15.0,
    },
)
.required()
.core();

fn yes_no(b: bool) -> Json {
    Json::str(if b { "yes" } else { "no" })
}

fn km2(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Area, "km2").expect("km2"),
    }
}

macro_rules! view {
    ($name:ident, $id:literal, $run:ident, $title:literal, $summary:literal,
     $aliases:expr, $keywords:expr, $inputs:expr, $outputs:expr, $errors:expr,
     $when:literal, $limits:literal, $ex_title:literal, $ex:literal, $related:expr,
     $sentence:literal) => {
        pub static $name: ToolDef = ToolDef {
            id: concat!("indexing.h3.", $id),
            stability: gp_base::tool::Stability::Stable,
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: $inputs,
            outputs: $outputs,
            errors: $errors,
            warnings: WARNINGS,
            model: CELL_INFO.model,
            accuracy: CELL_INFO.accuracy,
            when_to_use: $when,
            limitations: $limits,
            references: &[H3_DOCS, H3O],
            examples: &[Example {
                id: "primary",
                title: $ex_title,
                input: $ex,
                source: "H3 v4 API, computed by h3o and checked against H3 C",
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
    CELL_TO_LATLNG,
    "cell-to-latlng",
    run_cell_to_latlng,
    "H3 cell to latitude and longitude",
    "The latitude and longitude of an H3 cell's center, with the cell's resolution.",
    &["cellToLatLng", "H3 cell center", "H3 index to coordinates"],
    &["H3", "cell", "center", "latitude", "longitude", "decode"],
    &[cell_in("cell", "H3 cell")],
    &[
        angle("lat", "Center latitude", "[-90,90]"),
        angle("lon", "Center longitude", "[-180,180]"),
        count("resolution", "Resolution", "0 to 15"),
    ],
    &[],
    "Use this when an H3 index needs to go on a map or into a table as a point: a cell from a dataset, a log, or an aggregate. Paste the index in hexadecimal. The tool returns the latitude and longitude of the cell's center and its resolution, which says how large the area behind that point is.",
    "A cell is an area, not a point, so the center stands in for every location inside it; at coarse resolutions that is kilometers of ground. The center is the cell's own, on the sphere H3 uses, so it is not the average of the boundary vertices. Use the cell inspector for the boundary and area too.",
    "8a2a1072b59ffff",
    r#"{"cell":"8a2a1072b59ffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.lat-lng-to-cell" => "alternative", "indexing.h3.cell-boundary" => "next"),
    "The cell's center is at {lat}, {lon} (resolution {resolution})."
);

fn run_cell_to_latlng(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let center = LatLng::from(c);
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(center.lat()))),
        ("lon", ctx.out("lon", deg(center.lng()))),
        ("resolution", Json::Num(f64::from(u8::from(c.resolution())))),
    ]))
}

view!(
    CELL_BOUNDARY,
    "cell-boundary",
    run_cell_boundary,
    "H3 cell boundary",
    "The vertices of an H3 cell's boundary in order, ready to draw the hexagon or pentagon on a map.",
    &["cellToBoundary", "H3 hexagon outline", "H3 cell polygon"],
    &[
        "H3", "boundary", "polygon", "outline", "vertices", "hexagon"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        count(
            "vertex_count",
            "Vertices",
            "6 for a hexagon, 5 for a pentagon, more where an edge crosses an icosahedron face"
        ),
        Field::new(
            "boundary",
            "Boundary",
            "Vertices counterclockwise",
            Kind::List {
                items: VERTEX_ITEM,
                min: 0,
                max: 10,
            },
        ),
    ],
    &[],
    "Use this to draw an H3 cell, clip data to it, or export its outline to a GIS. Paste the index. The tool returns the boundary's vertices counterclockwise as latitude and longitude, and how many there are: six for a hexagon, five for one of the twelve pentagons, and up to ten where the cell's edges cross a face of the icosahedron underneath.",
    "The vertices are joined by great-circle arcs on H3's sphere, so a long edge drawn as a straight line on a flat map is slightly off, most at coarse resolutions. The ring is not closed: repeat the first vertex if your format needs it. A cell that crosses the antimeridian has longitudes on both sides of ±180°.",
    "A resolution 7 cell near Lisbon",
    r#"{"cell":"873933676ffffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.cells-to-polygon" => "next", "indexing.h3.vertexes" => "alternative"),
    "The boundary has {vertex_count} vertices."
);

fn run_cell_boundary(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let boundary: Vec<Json> = c
        .boundary()
        .iter()
        .map(|ll| Json::obj([("lat", Json::Num(ll.lat())), ("lon", Json::Num(ll.lng()))]))
        .collect();
    Ok(Json::obj([
        ("vertex_count", Json::Num(boundary.len() as f64)),
        ("boundary", Json::Arr(boundary)),
    ]))
}

view!(
    GET_RESOLUTION,
    "resolution",
    run_resolution,
    "H3 cell resolution",
    "The resolution of an H3 cell, 0 (coarsest) to 15 (finest), with H3's average hexagon area at that resolution.",
    &[
        "getResolution",
        "what resolution is this H3 cell",
        "H3 cell level"
    ],
    &["H3", "resolution", "level", "cell size", "average area"],
    &[cell_in("cell", "H3 cell")],
    &[
        count("resolution", "Resolution", "0 to 15"),
        Field::new(
            "average_area",
            "Average cell area",
            "H3's average hexagon area at this resolution",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(6)),
    ],
    &[],
    "Use this to read the resolution packed into an H3 index before mixing cells, since most H3 operations need cells at one resolution. Paste the index. The tool returns its resolution and H3's average hexagon area at that resolution, a quick sense of the scale the data was aggregated at.",
    "The average is H3's published average over the resolution's hexagons, leaving out the 12 pentagons; a given cell can be smaller or larger, so use the cell area tool for this cell's own. The resolution is read from the index's bits, which says nothing about whether the data behind it was collected that finely.",
    "A resolution 12 cell",
    r#"{"cell":"8c2a1072b59adff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.resolution-chooser" => "alternative", "indexing.h3.cell-area" => "next"),
    "This is a resolution {resolution} cell; cells there average {average_area}."
);

fn run_resolution(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let r = c.resolution();
    Ok(Json::obj([
        ("resolution", Json::Num(f64::from(u8::from(r)))),
        ("average_area", ctx.out("average_area", km2(r.area_km2()))),
    ]))
}

view!(
    BASE_CELL,
    "base-cell",
    run_base_cell,
    "H3 base cell",
    "Which of H3's 122 resolution 0 base cells an H3 cell descends from, and whether that base cell is a pentagon.",
    &["getBaseCellNumber", "H3 base cell number", "H3 root cell"],
    &[
        "H3",
        "base cell",
        "resolution 0",
        "root",
        "pentagon",
        "icosahedron"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        count("base_cell", "Base cell", "0 to 121"),
        text(
            "base_pentagon",
            "Base cell is a pentagon",
            "yes for 12 of the 122"
        ),
        text(
            "base_index",
            "Base cell index",
            "The resolution 0 cell as an H3 index"
        ),
    ],
    &[],
    "Use this to group or shard H3 data by its top-level ancestor, or to check whether a cell sits under one of the twelve pentagon base cells, where H3's neighbor and path operations behave differently. Paste the index. The tool returns the base cell number, its resolution 0 index, and whether it is a pentagon.",
    "Base cells are large, about 4.4 million km² on average, so neighbors on the ground can have different base cells. A cell under a pentagon base cell is not itself a pentagon unless it is that base cell's center child at every step; the pentagon tool answers that for the cell.",
    "A cell over Bogotá",
    r#"{"cell":"8966e42888fffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.parent" => "alternative", "indexing.h3.is-pentagon" => "next"),
    "The cell descends from base cell {base_cell}."
);

fn run_base_cell(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let base = c
        .parent(h3o::Resolution::Zero)
        .expect("every cell has a resolution 0 parent");
    Ok(Json::obj([
        ("base_cell", Json::Num(f64::from(u8::from(c.base_cell())))),
        ("base_pentagon", yes_no(base.is_pentagon())),
        ("base_index", Json::str(hex(base))),
    ]))
}

view!(
    IS_PENTAGON,
    "is-pentagon",
    run_is_pentagon,
    "Is this H3 cell a pentagon?",
    "Whether an H3 cell is one of the twelve pentagons at its resolution, which have five neighbors instead of six.",
    &["isPentagon", "H3 pentagon check", "is H3 cell a pentagon"],
    &[
        "H3",
        "pentagon",
        "distortion",
        "neighbors",
        "icosahedron vertex"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        text("pentagon", "Pentagon", "yes or no"),
        count("resolution", "Resolution", "0 to 15"),
        count(
            "neighbors",
            "Neighbors",
            "5 for a pentagon, 6 for a hexagon"
        ),
    ],
    &[],
    "Use this when an H3 operation fails or gives an odd count near a cell, or before relying on six neighbors. Paste the index. The tool says whether the cell is a pentagon and how many neighbors it has. Each resolution has exactly twelve pentagons, centered on the vertices of the icosahedron and placed over oceans where possible.",
    "Only the cell itself is checked; its neighbors can be distorted near a pentagon without being one. Local grid coordinates and paths fail across pentagon distortion, which this does not predict. Twelve cells in billions is rare, but large global datasets do meet them.",
    "A resolution 3 pentagon",
    r#"{"cell":"830800fffffffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.grid-disk" => "next", "indexing.h3.is-class-iii" => "alternative"),
    "Pentagon: {pentagon}. The cell has {neighbors} neighbors."
);

fn run_is_pentagon(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    Ok(Json::obj([
        ("pentagon", yes_no(c.is_pentagon())),
        ("resolution", Json::Num(f64::from(u8::from(c.resolution())))),
        (
            "neighbors",
            Json::Num(if c.is_pentagon() { 5.0 } else { 6.0 }),
        ),
    ]))
}

view!(
    IS_CLASS_III,
    "is-class-iii",
    run_is_class_iii,
    "H3 resolution class check",
    "Whether an H3 cell's resolution is Class III (odd resolutions, rotated about 19.1° from the previous) or Class II (even).",
    &["isResClassIII", "H3 Class III check", "H3 resolution class"],
    &[
        "H3",
        "Class III",
        "Class II",
        "rotation",
        "aperture 7",
        "odd resolution"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        text("class3", "Class III", "yes for odd resolutions"),
        count("resolution", "Resolution", "0 to 15"),
    ],
    &[],
    "Use this when writing or checking code that handles H3's alternating grid orientation, such as local coordinates, drawing, or hierarchy traversal. Paste the index. The tool says whether its resolution is Class III, the odd resolutions whose grid is turned about 19.1° from the resolution above, or Class II, the even ones.",
    "The class depends only on the resolution, not on where the cell is, so every cell at one resolution gives the same answer. It describes the grid's orientation, not the cell's shape or size. Children at the next resolution are always of the other class.",
    "A resolution 9 cell",
    r#"{"cell":"89283082803ffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.resolution" => "alternative", "indexing.h3.children" => "next"),
    "Class III: {class3} (resolution {resolution})."
);

fn run_is_class_iii(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    Ok(Json::obj([
        ("class3", yes_no(c.resolution().is_class3())),
        ("resolution", Json::Num(f64::from(u8::from(c.resolution())))),
    ]))
}

view!(
    VALIDATE,
    "validate",
    run_validate,
    "Validate an H3 index",
    "Whether a string is a valid H3 cell index, and if not, why not.",
    &["isValidCell", "H3 index validator", "check H3 cell"],
    &["H3", "validate", "valid", "index", "check", "data quality"],
    &[Field::new(
        "cell",
        "Index to check",
        "Hexadecimal or decimal, like 892a8471487ffff",
        Kind::Text { max_len: 64 },
    )
    .required()
    .core()],
    &[
        text("valid", "Valid", "yes or no"),
        text("reason", "Why", "What makes it invalid, or what it is"),
        count("resolution", "Resolution", "0 to 15, when valid").optional(),
    ],
    &[],
    "Use this to check H3 values in a dataset or an API response before trusting them. Paste the index in hexadecimal or decimal. The tool says whether it is a valid cell and, when it is not, what is wrong: the wrong length, characters that are not hexadecimal, a mode that marks an edge or vertex rather than a cell, or digits that cannot occur.",
    "This checks that the value is a well-formed cell index, not that it is the cell you meant. Directed edge and vertex indexes are valid H3 values but not cells, and are reported as not valid here. Leading zeros and letter case do not matter in hexadecimal.",
    "A cell index with a digit that cannot occur",
    r#"{"cell":"8f2a1072b59a5f7"}"#,
    rel!("indexing.h3.cell-info" => "next", "indexing.h3.lat-lng-to-cell" => "alternative", "indexing.h3.resolution" => "next"),
    "Valid: {valid}. {reason}"
);

fn run_validate(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let raw = ctx.raw("cell").cloned().unwrap_or(Value::Null);
    Ok(match crate::h3::parse_cell(Some(&raw), "/cell") {
        Ok(c) => Json::obj([
            ("valid", yes_no(true)),
            (
                "reason",
                Json::str(format!(
                    "A resolution {} {}.",
                    u8::from(c.resolution()),
                    if c.is_pentagon() {
                        "pentagon"
                    } else {
                        "hexagon"
                    }
                )),
            ),
            ("resolution", Json::Num(f64::from(u8::from(c.resolution())))),
        ]),
        Err(e) => Json::obj([("valid", yes_no(false)), ("reason", Json::str(e.message))]),
    })
}

view!(
    CELL_AREA,
    "cell-area",
    run_cell_area,
    "H3 cell area",
    "The exact area of one H3 cell on H3's sphere, with its resolution's average hexagon area for comparison.",
    &[
        "cellAreaKm2",
        "H3 cell area calculator",
        "area of an H3 hexagon"
    ],
    &["H3", "area", "km²", "hexagon", "density", "normalize"],
    &[cell_in("cell", "H3 cell")],
    &[
        CELL_INFO.outputs[6],
        Field::new(
            "average_area",
            "Average for its resolution",
            "H3's average hexagon area at this resolution",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "ratio",
            "Share of the average",
            "This cell's area ÷ the average",
            Kind::Number {
                min: 0.0,
                max: 10.0
            },
        )
        .precision(Precision::Decimals(4)),
    ],
    &[],
    "Use this to turn counts per H3 cell into densities, or to see how far a cell departs from its resolution's average. Paste the index. The tool returns the cell's own area, H3's average hexagon area at that resolution, and the ratio between them; cells near pentagons and toward the icosahedron's vertices run smaller.",
    "The area is on H3's authalic sphere, not the WGS 84 ellipsoid, so it differs from an ellipsoidal area of the same outline by up to a few tenths of a percent. For a density over many cells, the cells' own areas are better than one average. The ratio compares with the resolution's average hexagon, so a pentagon's ratio is well under 1.",
    "A resolution 8 cell over Cape Town",
    r#"{"cell":"88ad361519fffff"}"#,
    rel!("indexing.h3.cell-info" => "parent", "indexing.h3.resolution" => "alternative", "indexing.h3.resolution-chooser" => "next"),
    "The cell covers {area}, {ratio} of its resolution's average."
);

fn run_cell_area(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let (area, average) = (c.area_km2(), c.resolution().area_km2());
    Ok(Json::obj([
        ("area", ctx.out("area", km2(area))),
        ("average_area", ctx.out("average_area", km2(average))),
        ("ratio", Json::Num(area / average)),
    ]))
}

view!(
    GRID_DISTANCE,
    "grid-distance",
    run_grid_distance,
    "H3 grid distance",
    "The number of H3 grid steps between two cells of the same resolution.",
    &[
        "gridDistance",
        "H3 distance in cells",
        "H3 hops between cells"
    ],
    &["H3", "grid distance", "steps", "hops", "rings", "k"],
    &[cell_in("from", "From cell"), cell_in("to", "To cell")],
    &[count(
        "distance",
        "Grid distance",
        "Steps between the cells"
    )],
    &[ErrorCode::InvalidInput, ErrorCode::DegenerateGeometry],
    "Use this to measure how far apart two H3 cells are in the grid itself, for neighborhood queries, smoothing radii, or checking a k-ring size. Enter two cells of the same resolution. The tool returns the number of steps between them, the k of the smallest ring around one that reaches the other.",
    "Grid steps are not ground distance: a step is about one cell's width, which changes across the globe, so use a geodesic tool for meters. The cells must share a resolution, and H3 cannot measure across pentagon distortion or between cells very far apart, where it says so instead of guessing.",
    "Two resolution 9 cells in Pittsburgh",
    r#"{"from":"892a8471487ffff","to":"892a84710cbffff"}"#,
    rel!("indexing.h3.grid-path" => "parent", "indexing.h3.grid-disk" => "next", "indexing.h3.cell-to-local-ij" => "alternative"),
    "The cells are {distance} grid steps apart."
);

fn run_grid_distance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (a, b) = (cell(ctx, "from")?, cell(ctx, "to")?);
    let d = a.grid_distance(b).map_err(local_ij_error)?;
    Ok(Json::obj([("distance", Json::Num(f64::from(d)))]))
}

view!(
    CENTER_CHILD,
    "center-child",
    run_center_child,
    "H3 center child",
    "The child cell at the center of an H3 cell, at a finer resolution.",
    &[
        "cellToCenterChild",
        "H3 center child cell",
        "H3 child at the center"
    ],
    &[
        "H3",
        "center child",
        "children",
        "hierarchy",
        "finer resolution"
    ],
    &[cell_in("cell", "H3 cell"), RESOLUTION_IN],
    &[
        text(
            "center_child",
            "Center child",
            "At the resolution asked for"
        ),
        angle("lat", "Center latitude", "[-90,90]"),
        angle("lon", "Center longitude", "[-180,180]"),
    ],
    &[ErrorCode::InvalidInput],
    "Use this to step down H3's hierarchy along the middle of a cell, for a representative fine cell, a consistent sample point, or a key that sorts with its parent. Enter the cell and a finer resolution. The tool returns the child at the center at that resolution and its center point, which lies close to the parent's own center.",
    "Only the center child is returned; the children tool lists all of them. The child's center is near, not exactly at, the parent's center, because aperture-7 cells do not nest exactly. The resolution must be finer than the cell's own.",
    "Resolution 11 under a resolution 7 cell",
    r#"{"cell":"872a10729ffffff","resolution":11}"#,
    rel!("indexing.h3.children" => "parent", "indexing.h3.parent" => "alternative", "indexing.h3.child-position" => "next"),
    "The center child at that resolution is {center_child}."
);

fn run_center_child(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let r = resolution(ctx)?;
    let child = c.center_child(r).ok_or_else(|| {
        ToolError::invalid(
            "/resolution",
            format!(
                "Children are finer: pick a resolution from {} to 15.",
                u8::from(c.resolution())
            ),
        )
    })?;
    let center = LatLng::from(child);
    Ok(Json::obj([
        ("center_child", Json::str(hex(child))),
        ("lat", ctx.out("lat", deg(center.lat()))),
        ("lon", ctx.out("lon", deg(center.lng()))),
    ]))
}

view!(
    CHILD_POSITION,
    "child-position",
    run_child_position,
    "H3 child position",
    "Where an H3 cell falls in the ordered list of its ancestor's children, and the ancestor at the resolution asked for.",
    &[
        "cellToChildPos",
        "H3 child position",
        "index of a child in its parent"
    ],
    &[
        "H3",
        "child position",
        "ordinal",
        "hierarchy",
        "children order"
    ],
    &[cell_in("cell", "H3 cell"), RESOLUTION_IN],
    &[
        count(
            "position",
            "Position",
            "Zero-based, among the ancestor's children at the cell's resolution"
        ),
        text("parent", "Ancestor", "At the resolution asked for"),
        count(
            "siblings",
            "Children of the ancestor",
            "At the cell's resolution"
        ),
    ],
    &[ErrorCode::InvalidInput],
    "Use this to store an H3 cell compactly as its ancestor and a number, or to address children in a fixed order. Enter the cell and a coarser resolution for the ancestor. The tool returns the cell's zero-based position among all of that ancestor's children at the cell's own resolution, the ancestor itself, and how many children there are.",
    "Positions follow H3's own child order, which is not spatial: neighbors on the ground are not neighbors in the list. Under a pentagon ancestor there are fewer children and the positions skip the deleted subsequence. The resolution must not be finer than the cell's own.",
    "A resolution 10 cell under its resolution 7 ancestor",
    r#"{"cell":"8a2a1072b59ffff","resolution":7}"#,
    rel!("indexing.h3.children" => "parent", "indexing.h3.center-child" => "alternative", "indexing.h3.parent" => "next"),
    "The cell is child {position} of {siblings} under {parent}."
);

fn run_child_position(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let r = resolution(ctx)?;
    let (pos, parent) = c.child_position(r).zip(c.parent(r)).ok_or_else(|| {
        ToolError::invalid(
            "/resolution",
            format!(
                "The ancestor is coarser: pick a resolution from 0 to {}.",
                u8::from(c.resolution())
            ),
        )
    })?;
    Ok(Json::obj([
        ("position", Json::Num(pos as f64)),
        ("parent", Json::str(hex(parent))),
        (
            "siblings",
            Json::Num(parent.children_count(c.resolution()) as f64),
        ),
    ]))
}

const EDGE_ITEM: &[Field] = &[
    text("edge", "Edge", "Directed edge index"),
    text("to", "Neighbor", "The cell across the edge"),
];

view!(
    DIRECTED_EDGES,
    "directed-edges",
    run_directed_edges,
    "H3 directed edges",
    "The directed edges leading out of an H3 cell, one to each neighbor, as H3 edge indexes.",
    &[
        "originToDirectedEdges",
        "H3 outgoing edges",
        "H3 neighbor edges"
    ],
    &[
        "H3",
        "directed edge",
        "neighbors",
        "flow",
        "graph",
        "adjacency"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        count("edge_count", "Edges", "6, or 5 for a pentagon"),
        Field::new(
            "edges",
            "Edges",
            "Each edge and the neighbor it leads to",
            Kind::List {
                items: EDGE_ITEM,
                min: 0,
                max: 6,
            },
        ),
    ],
    &[],
    "Use this to build a graph over H3 cells, to record movement or flow from one cell to the next, or to key data by the boundary between two cells. Paste the cell. The tool returns each directed edge leaving it as an H3 edge index, with the neighboring cell it leads to, in ascending index order.",
    "An edge index is directional: the edge from A to B is a different index from B to A. A pentagon has five edges, not six. The edge's ground length varies across the globe; the edge length tool gives it. These are indexes for the edges, not their geometry.",
    "A resolution 8 cell in Tokyo",
    r#"{"cell":"882f5a363bfffff"}"#,
    rel!("indexing.h3.edges" => "parent", "indexing.h3.edge-length" => "next", "indexing.h3.grid-disk" => "alternative"),
    "The cell has {edge_count} outgoing edges."
);

fn run_directed_edges(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    // In ascending index order, which for a hexagon is H3's direction order.
    let mut sorted: Vec<DirectedEdgeIndex> = c.edges().collect();
    sorted.sort_unstable();
    let edges: Vec<Json> = sorted
        .into_iter()
        .map(|e| {
            Json::obj([
                ("edge", Json::str(e.to_string())),
                ("to", Json::str(hex(e.destination()))),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("edge_count", Json::Num(edges.len() as f64)),
        ("edges", Json::Arr(edges)),
    ]))
}

const VERTEX_ROW: &[Field] = &[
    text("vertex", "Vertex", "H3 vertex index"),
    angle("lat", "Latitude", "[-90,90]"),
    angle("lon", "Longitude", "[-180,180]"),
];

view!(
    VERTEXES,
    "vertexes",
    run_vertexes,
    "H3 vertexes",
    "The topological vertexes of an H3 cell as H3 vertex indexes, with each vertex's latitude and longitude.",
    &["cellToVertexes", "H3 vertex indexes", "H3 cell corners"],
    &[
        "H3",
        "vertex",
        "corners",
        "vertex index",
        "topology",
        "mesh"
    ],
    &[cell_in("cell", "H3 cell")],
    &[
        count("vertex_count", "Vertexes", "6, or 5 for a pentagon"),
        Field::new(
            "vertexes",
            "Vertexes",
            "Each vertex index and its position",
            Kind::List {
                items: VERTEX_ROW,
                min: 0,
                max: 6,
            },
        ),
    ],
    &[],
    "Use this when cells must share corners exactly, for a mesh, a network of cell corners, or deduplicating boundary points between neighbors. Paste the cell. The tool returns each topological vertex as an H3 vertex index, which neighboring cells share, with its latitude and longitude.",
    "Topological vertexes are the cell's corners in H3's grid, five or six, and do not include the extra points the boundary gains where an edge crosses an icosahedron face; the boundary tool has those. Each vertex index is owned by one of the cells that meet there, so the same corner always has the same index.",
    "A resolution 6 cell in Nairobi",
    r#"{"cell":"867a6e42fffffff"}"#,
    rel!("indexing.h3.edges" => "parent", "indexing.h3.cell-boundary" => "alternative", "indexing.h3.directed-edges" => "alternative"),
    "The cell has {vertex_count} vertexes."
);

fn run_vertexes(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let vertexes: Vec<Json> = c
        .vertexes()
        .map(|v| {
            let ll = LatLng::from(v);
            Json::obj([
                ("vertex", Json::str(v.to_string())),
                ("lat", Json::Num(ll.lat())),
                ("lon", Json::Num(ll.lng())),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("vertex_count", Json::Num(vertexes.len() as f64)),
        ("vertexes", Json::Arr(vertexes)),
    ]))
}

view!(
    EDGE_LENGTH,
    "edge-length",
    run_edge_length,
    "H3 edge length",
    "The length of one H3 directed edge on the ground, with the cells on either side of it.",
    &[
        "edgeLengthM",
        "H3 edge length calculator",
        "length of an H3 edge"
    ],
    &[
        "H3",
        "edge length",
        "directed edge",
        "meters",
        "boundary length"
    ],
    &[Field::new(
        "edge",
        "Directed edge",
        "An H3 edge index, like 1192a8471487ffff",
        Kind::Text { max_len: 32 },
    )
    .required()
    .core()],
    &[
        Field::new(
            "length",
            "Edge length",
            "Along the edge on H3's sphere",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(6)),
        text("origin", "From cell", "The cell the edge leaves"),
        text("destination", "To cell", "The cell across the edge"),
    ],
    &[ErrorCode::InvalidInput],
    "Use this to weight a flow or a boundary by its real length, for example people or vehicles crossing between cells per meter of boundary. Paste a directed edge index, as the directed edges tool lists them. The tool returns the edge's length on the ground and the two cells it separates.",
    "The length is measured on H3's authalic sphere, so it differs from an ellipsoidal length by a few tenths of a percent at most. An edge that crosses an icosahedron face is measured along its true path, not as one straight segment. Only directed edge indexes are accepted; a cell or vertex index is refused.",
    "An edge out of a resolution 9 cell in Pittsburgh",
    r#"{"edge":"1192a8471487ffff"}"#,
    rel!("indexing.h3.directed-edges" => "parent", "indexing.h3.edges" => "parent", "indexing.h3.cell-area" => "alternative"),
    "The edge is {length} long, from {origin} to {destination}."
);

fn run_edge_length(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text = ctx.text("edge")?.expect("required");
    let e: DirectedEdgeIndex = text.trim().parse().map_err(|_| {
        ToolError::invalid(
            "/edge",
            format!("\"{}\" is not an H3 directed edge index.", text.trim()),
        )
        .hint("The directed edges tool lists a cell's edges, like 1192a8471487ffff.")
    })?;
    let (origin, destination): (CellIndex, CellIndex) = (e.origin(), e.destination());
    Ok(Json::obj([
        (
            "length",
            ctx.out(
                "length",
                Q {
                    value: e.length_m(),
                    unit: units::by_symbol(QT::Length, "m").expect("m"),
                },
            ),
        ),
        ("origin", Json::str(hex(origin))),
        ("destination", Json::str(hex(destination))),
    ]))
}
