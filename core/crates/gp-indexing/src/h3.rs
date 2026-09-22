//! H3 hexagonal grid tools (hexagonal-grids spec) on h3o, a Rust port of the
//! H3 C library. Indexes travel as strings: hex (optional 0x, any case) or
//! decimal; JSON numbers are refused because they cannot hold 64 bits.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Slot, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;
use h3o::{CellIndex, LatLng, Resolution};
use serde_json::{Map, Value};

const H3_DOCS: Reference = Reference {
    title: "H3: A Hexagonal Hierarchical Geospatial Indexing System (API reference v4)",
    issuer: "Uber Technologies and the H3 contributors",
    year: 2024,
    edition: "H3 4.x documentation",
    locator: "Indexing, inspection, traversal, hierarchy, directed edges, vertexes, and resolution tables",
    url: "https://h3geo.org/docs/api/indexing",
};
const H3O: Reference = Reference {
    title: "h3o: a Rust implementation of H3",
    issuer: "Hydronium Labs",
    year: 2025,
    edition: "0.11.0 (tested against H3 C)",
    locator: "CellIndex, LatLng, Resolution",
    url: "https://docs.rs/h3o/0.11.0/h3o/",
};

/// Most cells one call lists; larger answers give only a count.
const LIST_LIMIT: usize = 10_000;

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

const fn cell_in(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Hex like 892a8471487ffff (0x optional) or a decimal string",
        Kind::Text { max_len: 24 },
    )
    .required()
    .core()
}
const fn text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 40 })
}
const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: 0.0,
            max: 1e18,
        },
    )
    .precision(Precision::Decimals(0))
}
const fn angle(name: &'static str, title: &'static str, range: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Decimal degrees, like 40.4406 for a latitude and -80.002 for a longitude",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(9))
    .angle_range(range)
}
const RES_IN: Field = Field::new(
    "resolution",
    "Resolution",
    "0 (coarsest) to 15",
    Kind::Number {
        min: 0.0,
        max: 15.0,
    },
)
.required()
.core();

const CELL_ITEM: &[Field] = &[text("cell", "Cell", "H3 index (hex)")];
const fn cell_list(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::List {
            items: CELL_ITEM,
            min: 0,
            max: LIST_LIMIT,
        },
    )
}

/// Parses an H3 cell from a hex or decimal string.
fn parse_cell(v: Option<&Value>, at: &str) -> Result<CellIndex, ToolError> {
    let s = match v {
        Some(Value::String(s)) => s.trim(),
        Some(Value::Number(_)) => {
            return Err(ToolError::invalid(
                at,
                "H3 indexes are 64-bit, which a JSON number cannot hold exactly. Pass the index as a string, like \"892a8471487ffff\".",
            ));
        }
        _ => return Err(ToolError::invalid(at, "Give the H3 index as a string.")),
    };
    let bad = || ToolError::invalid(at, format!("\"{s}\" is not an H3 cell index."));
    let raw = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).map_err(|_| bad())?
    } else if s.len() >= 17 && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse::<u64>().map_err(|_| bad())?
    } else {
        u64::from_str_radix(s, 16).map_err(|_| bad())?
    };
    CellIndex::try_from(raw).map_err(|_| bad())
}

fn cell(ctx: &Ctx, name: &str) -> Result<CellIndex, ToolError> {
    parse_cell(ctx.raw(name), &format!("/{name}"))
}

fn resolution(ctx: &Ctx) -> Result<Resolution, ToolError> {
    let r = ctx.number("resolution")?.expect("required");
    if r.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/resolution",
            "The resolution is a whole number from 0 to 15.",
        ));
    }
    Resolution::try_from(r as u8)
        .map_err(|_| ToolError::invalid("/resolution", "The resolution is 0 to 15."))
}

fn hex(c: CellIndex) -> String {
    c.to_string()
}

fn list(cells: impl IntoIterator<Item = CellIndex>) -> Json {
    Json::Arr(
        cells
            .into_iter()
            .map(|c| Json::obj([("cell", Json::str(hex(c)))]))
            .collect(),
    )
}

fn pentagon_note(ctx: &mut Ctx, what: &str) {
    ctx.warnings.push(Warning::new("PENTAGON_DISTORTION", format!("{what} involves one of the 12 pentagons at this resolution, where cells have five neighbors and distortion is higher.")));
}

fn local_ij_error(e: h3o::error::LocalIjError) -> ToolError {
    use h3o::error::LocalIjError as E;
    let why = match e {
        E::ResolutionMismatch => "the two cells have different resolutions",
        E::Pentagon => "the path crosses pentagon distortion, where H3's local grid is undefined",
        _ => "the cells are too far apart for H3's local grid",
    };
    let code = if matches!(e, E::ResolutionMismatch) {
        ErrorCode::InvalidInput
    } else {
        ErrorCode::DegenerateGeometry
    };
    ToolError::new(code, format!("H3 cannot compute this: {why}.")).at("/to")
}

// ---------------------------------------------------------------- indexing

pub static LAT_LNG_TO_CELL: ToolDef = ToolDef {
    id: "indexing.h3.lat-lng-to-cell",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 cell for a point",
    summary: "The H3 cell containing a latitude and longitude at resolution 0 to 15 (latLngToCell), with the cell's center and area.",
    aliases: &["latLngToCell", "H3 index calculator", "lat long to H3"],
    keywords: &[
        "H3",
        "hexagon",
        "cell",
        "index",
        "latLngToCell",
        "resolution",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        RES_IN,
    ],
    outputs: &[
        text("cell", "Cell", "H3 index (hex)"),
        angle("center_lat", "Center latitude", "[-90,90]"),
        angle("center_lon", "Center longitude", "[-180,180]"),
        Field::new(
            "area",
            "Cell area",
            "Exact area of this cell",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(4)),
        text("pentagon", "Pentagon", "yes or no"),
    ],
    errors: &[],
    warnings: &[
        "PENTAGON_DISTORTION",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "H3 v4 (h3o 0.11): gnomonic projection onto icosahedron faces, aperture-7 hexagon hierarchy",
    accuracy: "Identical indexes to H3 C; coordinates within 1e-12° below 88° latitude, 5e-11° nearer the poles",
    when_to_use: "Use this to put a point on the H3 grid: the cell containing a latitude and longitude at the resolution you choose, with the cell's center and area. It is the entry point for aggregating points into hexagons, joining datasets on a common grid, or keying rows by cell. It is also the join key between datasets that have nothing else in common: two sets of points indexed to the same resolution can be aggregated and compared cell by cell.",
    limitations: "The cell is an area and the point is somewhere inside it, so aggregation at too coarse a resolution hides real structure and too fine a one scatters it; the resolution chooser helps pick. H3 cells are not equal-area, so counts per cell are not strictly comparable without dividing by each cell's own area. Points on a cell boundary fall into exactly one cell by the library's rule, so a dataset binned at one resolution cannot be re-binned by string manipulation; it has to be indexed again.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at resolution 9",
        input: r#"{"lat":40.446111,"lon":-79.982222,"resolution":9}"#,
        source: "add-spatial-indexing-and-raster scenario: 892a8471487ffff",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cell", "cell")],
    }],
    related: &[
        Related {
            id: "indexing.h3.cell-info",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.resolution-chooser",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.grid-disk",
            reason: "next",
        },
    ],
    sentence: "The H3 cell is {cell}, about {area}.",
    limits: &[("batchRows", 10_000)],
    slots: &[Slot::new(
        "resolution",
        &["res", "resolution", "r", "level"],
    )],
    run: run_to_cell,
    ..ToolDef::BLANK
};

fn run_to_cell(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let res = resolution(ctx)?;
    let ll = LatLng::new(lat, lon)
        .map_err(|_| ToolError::invalid("/lat", "Latitude and longitude must be finite."))?;
    let c = ll.to_cell(res);
    let center = LatLng::from(c);
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Resolution",
            "the cell size the resolution picks, 0 coarsest to 15 finest",
            format!("resolution {}", res as u8),
            format!("{} km² per cell", n(c.area_km2(), 6)),
        );
        ctx.step(
            "Cell centre",
            "the centre of the cell the point falls in",
            format!("{}°, {}°", n(lat, 6), n(lon, 6)),
            format!("{}°, {}°", n(center.lat(), 6), n(center.lng(), 6)),
        );
        ctx.step(
            "Cell",
            "the index of that cell, in hexadecimal",
            format!("resolution {} at {}°, {}°", res as u8, n(lat, 6), n(lon, 6)),
            hex(c),
        );
    }
    Ok(Json::obj([
        ("cell", Json::str(hex(c))),
        ("center_lat", ctx.out("center_lat", deg(center.lat()))),
        ("center_lon", ctx.out("center_lon", deg(center.lng()))),
        (
            "area",
            ctx.out(
                "area",
                Q {
                    value: c.area_km2(),
                    unit: units::by_symbol(QT::Area, "km2").expect("km2"),
                },
            ),
        ),
        (
            "pentagon",
            Json::str(if c.is_pentagon() { "yes" } else { "no" }),
        ),
    ]))
}

const VERTEX_ITEM: &[Field] = &[
    angle("lat", "Latitude", "[-90,90]"),
    angle("lon", "Longitude", "[-180,180]"),
];

pub static CELL_INFO: ToolDef = ToolDef {
    id: "indexing.h3.cell-info",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 cell inspector",
    summary: "Everything about an H3 cell: center (cellToLatLng), boundary (cellToBoundary), resolution, base cell, pentagon and Class III checks, area, and decimal form.",
    aliases: &[
        "cellToLatLng",
        "cellToBoundary",
        "H3 to lat long",
        "decode H3",
    ],
    keywords: &[
        "H3",
        "cell",
        "boundary",
        "center",
        "base cell",
        "pentagon",
        "Class III",
    ],
    inputs: &[cell_in("cell", "H3 cell")],
    outputs: &[
        angle("lat", "Center latitude", "[-90,90]"),
        angle("lon", "Center longitude", "[-180,180]"),
        count("resolution", "Resolution", "0 to 15"),
        count("base_cell", "Base cell", "0 to 121"),
        text("pentagon", "Pentagon", "yes or no"),
        text("class3", "Class III", "yes for odd resolutions"),
        Field::new(
            "area",
            "Cell area",
            "Exact area",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(6)),
        text(
            "decimal",
            "Decimal index",
            "The same 64-bit index in decimal",
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
    errors: &[],
    warnings: &["PENTAGON_DISTORTION", "EXPERIMENTAL_TOOL"],
    model: "H3 v4 (h3o 0.11)",
    accuracy: "Identical to H3 C; coordinates within 1e-12° below 88° latitude, 5e-11° nearer the poles; areas within 6e-15 relative at resolution 0 and 2e-8 at resolution 15, where the 1 m² cell's spherical excess runs out of digits",
    when_to_use: "Use this when an H3 index arrives and you need to know what it is: the center, the boundary to draw, the resolution, the base cell, whether it is one of the twelve pentagons, whether it is Class III, and the cell's area. It is the first stop when debugging an H3 dataset.",
    limitations: "The area is that cell's own; H3 cells are not equal-area, and neighbors differ by a few percent. A boundary at a Class III resolution carries extra vertices where cells meet, and a pentagon has no sixth neighbor, which is the case most code built on hexagons forgets.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "892a8471487ffff",
        input: r#"{"cell":"892a8471487ffff"}"#,
        source: "add-spatial-indexing-and-raster scenario: center about (40.444866°, -79.981847°)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.h3.lat-lng-to-cell",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.grid-disk",
            reason: "next",
        },
        Related {
            id: "indexing.h3.edges",
            reason: "next",
        },
    ],
    sentence: "The cell's center is {lat}, {lon}, at resolution {resolution}.",
    limits: &[("batchRows", 10_000)],
    run: run_cell_info,
    ..ToolDef::BLANK
};

fn run_cell_info(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let center = LatLng::from(c);
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let yn = |b: bool| Json::str(if b { "yes" } else { "no" });
    let boundary: Vec<Json> = c
        .boundary()
        .iter()
        .map(|ll| Json::obj([("lat", Json::Num(ll.lat())), ("lon", Json::Num(ll.lng()))]))
        .collect();
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(center.lat()))),
        ("lon", ctx.out("lon", deg(center.lng()))),
        ("resolution", Json::Num(f64::from(u8::from(c.resolution())))),
        ("base_cell", Json::Num(f64::from(u8::from(c.base_cell())))),
        ("pentagon", yn(c.is_pentagon())),
        ("class3", yn(c.resolution().is_class3())),
        (
            "area",
            ctx.out(
                "area",
                Q {
                    value: c.area_km2(),
                    unit: units::by_symbol(QT::Area, "km2").expect("km2"),
                },
            ),
        ),
        ("decimal", Json::str(u64::from(c).to_string())),
        ("boundary", Json::Arr(boundary)),
    ]))
}

// ---------------------------------------------------------------- traversal

const K_IN: Field = Field::new(
    "k",
    "k (rings)",
    "Grid distance, 0 to 100",
    Kind::Number {
        min: 0.0,
        max: 100.0,
    },
)
.required()
.core();

fn k_input(ctx: &Ctx) -> Result<u32, ToolError> {
    let k = ctx.number("k")?.expect("required");
    if k.fract() != 0.0 {
        return Err(ToolError::invalid("/k", "k is a whole number."));
    }
    Ok(k as u32)
}

pub static GRID_DISK: ToolDef = ToolDef {
    id: "indexing.h3.grid-disk",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 grid disk (k-ring)",
    summary: "All H3 cells within k steps of a cell (gridDisk); fewer than 3k(k+1)+1 when a pentagon is nearby.",
    aliases: &["gridDisk", "kRing", "H3 neighbors"],
    keywords: &["H3", "gridDisk", "k-ring", "neighbors", "traversal"],
    inputs: &[cell_in("cell", "Center cell"), K_IN],
    outputs: &[
        count("count", "Cells", "Number of cells"),
        cell_list("cells", "Cells", "Center first, then outward"),
    ],
    errors: &[],
    warnings: &["PENTAGON_DISTORTION", "EXPERIMENTAL_TOOL"],
    model: "H3 v4 gridDisk (h3o 0.11), falling back to the safe traversal near pentagons",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this for neighborhood queries on an H3 grid: every cell within k steps of a center, which is how a local aggregation, a buffer in cell counts, or a spread from a point is expressed. The disk is the usual first step in a spatial join on hexagons. It is also the cheap way to express a buffer: rather than a distance in meters, a disk of k rings at a chosen resolution gives a neighborhood whose size you control by the grid itself.",
    limitations: "The count is fewer than the hexagonal formula when a pentagon is within reach, and the result says so. Steps are grid distance, not ground distance: the disk is roughly circular but not exactly, and its radius in meters depends on the resolution and, slightly, on where it is. Because the disk is a count of steps, its ground radius changes with resolution and its edge is a ragged hexagon rather than a circle, so a true distance filter still belongs after it.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Ring 1 around pentagon 85080003fffffff",
        input: r#"{"cell":"85080003fffffff","k":1}"#,
        source: "add-spatial-indexing-and-raster scenario: 6 cells, not 7",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.grid-ring",
            reason: "alternative",
        },
        Related {
            id: "indexing.h3.grid-path",
            reason: "next",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "The disk of radius {k} holds {count} {plural count \"cell\" \"cells\"}.{warn PENTAGON_DISTORTION} A pentagon is nearby.{/warn}",
    limits: &[("batchRows", 1_000)],
    run: run_grid_disk,
    ..ToolDef::BLANK
};

fn run_grid_disk(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let k = k_input(ctx)?;
    let cells: Vec<CellIndex> = c.grid_disk(k);
    let full = 3 * k as usize * (k as usize + 1) + 1;
    if cells.len() < full || cells.iter().any(|x| x.is_pentagon()) {
        pentagon_note(ctx, "This disk");
    }
    Ok(Json::obj([
        ("count", Json::Num(cells.len() as f64)),
        ("cells", list(cells)),
    ]))
}

pub static GRID_RING: ToolDef = ToolDef {
    id: "indexing.h3.grid-ring",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 grid ring",
    summary: "The H3 cells exactly k steps from a cell (gridRing), the hollow ring.",
    aliases: &["gridRing", "hexRing"],
    keywords: &["H3", "gridRing", "ring", "traversal"],
    inputs: &[cell_in("cell", "Center cell"), K_IN],
    outputs: &[
        count("count", "Cells", "Number of cells"),
        cell_list("cells", "Cells", "The ring"),
    ],
    errors: &[],
    warnings: &["PENTAGON_DISTORTION", "EXPERIMENTAL_TOOL"],
    model: "H3 v4 gridRing (h3o 0.11)",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this when you want a hollow ring rather than a filled disk: the cells exactly k steps out. It is how a band at a given distance is selected, how a disk is built up one ring at a time, and how a search expands outward without repeating the cells it has already seen. Rings are how an expanding search is done without rework: take ring one, then ring two, stopping as soon as enough is found, instead of taking a large disk and discarding most of it.",
    limitations: "Like the disk, a ring is measured in grid steps rather than meters, and it is distorted near the twelve pentagons, where a ring can be incomplete. Rings at the same k have different ground radii at different resolutions. Near a pentagon a ring can come back incomplete or fail, so a search that relies on rings alone should handle that rather than assume six times k cells.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Ring 2 around 892a8471487ffff",
        input: r#"{"cell":"892a8471487ffff","k":2}"#,
        source: "H3 gridRing: 6k cells away from pentagons",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.grid-disk",
            reason: "alternative",
        },
        Related {
            id: "indexing.h3.grid-path",
            reason: "next",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "The ring at k = {k} has {count} {plural count \"cell\" \"cells\"}.",
    limits: &[("batchRows", 1_000)],
    run: run_grid_ring,
    ..ToolDef::BLANK
};

fn run_grid_ring(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let k = k_input(ctx)?;
    let cells: Vec<CellIndex> = c.grid_ring(k);
    let full = if k == 0 { 1 } else { 6 * k as usize };
    if cells.len() != full || cells.iter().any(|x| x.is_pentagon()) {
        pentagon_note(ctx, "This ring");
    }
    Ok(Json::obj([
        ("count", Json::Num(cells.len() as f64)),
        ("cells", list(cells)),
    ]))
}

pub static GRID_PATH: ToolDef = ToolDef {
    id: "indexing.h3.grid-path",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 grid path and distance",
    summary: "The grid distance between two H3 cells (gridDistance) and the line of cells joining them (gridPathCells).",
    aliases: &["gridPathCells", "gridDistance", "H3 line"],
    keywords: &[
        "H3",
        "gridPathCells",
        "gridDistance",
        "path",
        "line",
        "distance",
    ],
    inputs: &[cell_in("from", "From cell"), cell_in("to", "To cell")],
    outputs: &[
        count("distance", "Grid distance", "Steps between the cells"),
        cell_list("cells", "Path", "From first to last, inclusive"),
    ],
    errors: &[ErrorCode::DegenerateGeometry],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "H3 v4 local IJ coordinates (h3o 0.11)",
    accuracy: "Identical to H3 C; fails where H3 C fails (across pentagon distortion or very far apart)",
    when_to_use: "Use this to measure separation in cells rather than meters, and to draw the line of cells between two of them: the grid distance and the path, which is how movement, corridors, and step counts are expressed on an H3 grid. It is also how a corridor is built on the grid: the path's cells, widened by a ring or two, give a band between two places without any geometry work.",
    limitations: "Grid distance is a count of steps, not a distance on the ground, and the two only track each other within a resolution. Near the pentagons the path can fail to exist, which the result reports with the reason rather than returning a line that is not one. Both cells must be at the same resolution. The path is a grid line, not a route: it ignores terrain, roads, and obstacles, and between distant cells it can be long and is not guaranteed to exist.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Across Pittsburgh at resolution 9",
        input: r#"{"from":"892a8471487ffff","to":"892a847148bffff"}"#,
        source: "H3 gridPathCells",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.grid-disk",
            reason: "next",
        },
        Related {
            id: "indexing.h3.grid-ring",
            reason: "alternative",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "The cells are {distance} steps apart.",
    limits: &[("batchRows", 1_000)],
    run: run_grid_path,
    ..ToolDef::BLANK
};

fn run_grid_path(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (a, b) = (cell(ctx, "from")?, cell(ctx, "to")?);
    let d = a.grid_distance(b).map_err(local_ij_error)?;
    if d as usize >= LIST_LIMIT {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The path has {} cells, over the {LIST_LIMIT}-cell limit.",
                d + 1
            ),
        )
        .at("/to"));
    }
    let path: Vec<CellIndex> = a
        .grid_path_cells(b)
        .map_err(local_ij_error)?
        .collect::<Result<_, _>>()
        .map_err(local_ij_error)?;
    Ok(Json::obj([
        ("distance", Json::Num(f64::from(d))),
        ("cells", list(path)),
    ]))
}

// ---------------------------------------------------------------- hierarchy

pub static PARENT: ToolDef = ToolDef {
    id: "indexing.h3.parent",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 parent cell",
    summary: "The coarser H3 cell that contains a cell (cellToParent), and the cell's position among that parent's children.",
    aliases: &["cellToParent", "H3 parent"],
    keywords: &[
        "H3",
        "parent",
        "hierarchy",
        "cellToParent",
        "child position",
    ],
    inputs: &[cell_in("cell", "Cell"), RES_IN],
    outputs: &[
        text("parent", "Parent", "At the requested resolution"),
        count(
            "child_position",
            "Child position",
            "Index among the parent's children",
        ),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "H3 v4 hierarchy (h3o 0.11)",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this when rolling detail up: the coarser cell that contains a cell, and the child position it occupies within that parent. It is how a fine aggregation is summarized, and how cells at mixed resolutions are compared on common ground. The child position it reports is what lets you tell siblings apart within a parent, which matters when building keys or checking that a set is complete before compacting it.",
    limitations: "H3's hierarchy is approximate: a child is not wholly inside its parent, so rolling up near boundaries moves a little area between parents. The parent's resolution must be coarser than the cell's, and a chain of parents accumulates that approximation. Because the hierarchy is approximate, a point near a cell boundary can belong to a parent that does not contain its own cell's center, so rolled-up counts shift slightly between levels. Resolution 0 has no parent.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Parent of 892a8471487ffff at resolution 5",
        input: r#"{"cell":"892a8471487ffff","resolution":5}"#,
        source: "add-spatial-indexing-and-raster scenario: 852a8473fffffff",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cell", "parent")],
    }],
    related: &[
        Related {
            id: "indexing.h3.children",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
        Related {
            id: "indexing.h3.compact",
            reason: "alternative",
        },
    ],
    sentence: "The parent is {parent}.",
    limits: &[("batchRows", 10_000)],
    run: run_parent,
    ..ToolDef::BLANK
};

fn run_parent(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let r = resolution(ctx)?;
    let p = c.parent(r).ok_or_else(|| {
        ToolError::invalid(
            "/resolution",
            format!(
                "A parent is coarser: pick a resolution from 0 to {}.",
                u8::from(c.resolution())
            ),
        )
    })?;
    Ok(Json::obj([
        ("parent", Json::str(hex(p))),
        (
            "child_position",
            Json::Num(c.child_position(r).unwrap_or(0) as f64),
        ),
    ]))
}

pub static CHILDREN: ToolDef = ToolDef {
    id: "indexing.h3.children",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 children cells",
    summary: "The finer H3 cells inside a cell (cellToChildren), their count, and the center child.",
    aliases: &["cellToChildren", "cellToCenterChild", "H3 children"],
    keywords: &["H3", "children", "hierarchy", "center child"],
    inputs: &[cell_in("cell", "Cell"), RES_IN],
    outputs: &[
        count("count", "Children", "Number of children at that resolution"),
        text("center_child", "Center child", "cellToCenterChild"),
        cell_list("cells", "Children", "Listed when 10,000 or fewer").optional(),
    ],
    errors: &[],
    warnings: &["PENTAGON_DISTORTION", "EXPERIMENTAL_TOOL"],
    model: "H3 v4 hierarchy (h3o 0.11): 7 children per step, 6 for pentagons",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this when moving to a finer resolution: the cells inside a cell one or more levels down, their count, and the center child. It is how a coarse aggregation is broken into a finer one, and how a cell is refined where more detail is wanted. It is also how a coarse selection is refined for rendering: draw the parent at a distance, its children when zoomed in, which is the usual level-of-detail pattern on a hexagonal grid.",
    limitations: "H3's hierarchy is not an exact subdivision: children do not tile their parent exactly, so a child near the edge overlaps the neighboring parent. The count grows by about seven per level, so refining several levels at once produces very large sets; a pentagon has fewer children than a hexagon. Each level multiplies the count by roughly seven, so going three levels down from one cell is already several hundred cells and five levels is tens of thousands.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Children of 892a8471487ffff at resolution 10",
        input: r#"{"cell":"892a8471487ffff","resolution":10}"#,
        source: "add-spatial-indexing-and-raster scenario: 7 children",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.parent",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
        Related {
            id: "indexing.h3.uncompact",
            reason: "alternative",
        },
    ],
    sentence: "The cell has {count} {plural count \"child\" \"children\"} at that resolution.",
    limits: &[("batchRows", 1_000)],
    run: run_children,
    ..ToolDef::BLANK
};

fn run_children(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    let r = resolution(ctx)?;
    let center = c.center_child(r).ok_or_else(|| {
        ToolError::invalid(
            "/resolution",
            format!(
                "Children are finer: pick a resolution from {} to 15.",
                u8::from(c.resolution())
            ),
        )
    })?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let n = c.children_count(r);
    let mut out = vec![
        ("count", Json::Num(n as f64)),
        ("center_child", Json::str(hex(center))),
    ];
    if n as usize <= LIST_LIMIT {
        out.push(("cells", list(c.children(r))));
    }
    Ok(Json::obj(out))
}

const CELLS_IN: Field = Field::new(
    "cells",
    "Cells",
    "One H3 index per row",
    Kind::List {
        items: &[cell_in("cell", "Cell")],
        min: 1,
        max: LIST_LIMIT,
    },
)
.required()
.core();

fn cells_input(ctx: &Ctx) -> Result<Vec<CellIndex>, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows("cells")?;
    rows.iter()
        .enumerate()
        .map(|(i, r)| parse_cell(r.get("cell"), &format!("/cells/{i}/cell")))
        .collect()
}

pub static COMPACT: ToolDef = ToolDef {
    id: "indexing.h3.compact",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 compact",
    summary: "Replaces every complete group of 7 sibling cells with their parent, repeatedly, for the smallest set covering the same area (compactCells).",
    aliases: &["compactCells", "H3 compaction"],
    keywords: &["H3", "compact", "compaction", "hierarchy", "set"],
    inputs: &[CELLS_IN],
    outputs: &[
        count("count", "Cells after", "Compacted count"),
        cell_list("cells", "Compacted cells", "Mixed resolutions"),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "H3 v4 compactCells (h3o 0.11)",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this to shrink a large set of cells before storing or sending it: wherever seven siblings are all present they are replaced by their parent, repeatedly, giving the smallest mixed-resolution set that covers the same area. It is the usual way to keep an H3 coverage compact. It is what makes large coverages practical to store and ship: a continent-sized area at a fine resolution collapses to a few thousand mixed-resolution cells.",
    limitations: "The result is mixed resolution, so anything consuming it has to handle cells of different sizes, or expand it again with the uncompact tool. Compaction is exact — the area covered does not change — but it only collapses complete groups, so a set with gaps compacts little. Pentagons, having fewer children, constrain what can collapse. Compaction changes the resolution mix but not the area, so a consumer that assumes one resolution has to uncompact first or it will silently drop the coarse cells.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "The 7 children of 892a8471487ffff",
        input: r#"{"cells":[{"cell":"8a2a84714847fff"},{"cell":"8a2a8471484ffff"},{"cell":"8a2a84714857fff"},{"cell":"8a2a8471485ffff"},{"cell":"8a2a84714867fff"},{"cell":"8a2a8471486ffff"},{"cell":"8a2a84714877fff"}]}"#,
        source: "H3 compactCells: the 7 children compact to their parent, 892a8471487ffff",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.uncompact",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.polygon-to-cells",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "The set compacts to {count} {plural count \"cell\" \"cells\"}.",
    limits: &[("batchRows", 100)],
    run: run_compact,
    ..ToolDef::BLANK
};

/// Compacts one level at a time. (h3o's `compact` sizes whole subtrees as
/// `usize`, which overflows on 32-bit wasm below resolution 12.)
fn compact(mut cells: Vec<CellIndex>) -> Result<Vec<CellIndex>, &'static str> {
    let Some(res) = cells.first().map(|c| c.resolution()) else {
        return Ok(cells);
    };
    if cells.iter().any(|c| c.resolution() != res) {
        return Err("they must all share one resolution");
    }
    cells.sort_unstable();
    let n = cells.len();
    cells.dedup();
    if cells.len() < n {
        return Err("they must be unique");
    }
    let mut done = Vec::new();
    let mut level = cells;
    let mut r = u8::from(res);
    while r > 0 && !level.is_empty() {
        let (child_res, parent_res) = (
            Resolution::try_from(r).expect("1-15"),
            Resolution::try_from(r - 1).expect("0-14"),
        );
        let mut next = Vec::new();
        // Sorted H3 indexes keep siblings together.
        let mut i = 0;
        while i < level.len() {
            let parent = level[i].parent(parent_res).expect("coarser");
            let mut j = i;
            while j < level.len() && level[j].parent(parent_res) == Some(parent) {
                j += 1;
            }
            if (j - i) as u64 == parent.children_count(child_res) {
                next.push(parent);
            } else {
                done.extend_from_slice(&level[i..j]);
            }
            i = j;
        }
        next.sort_unstable();
        level = next;
        r -= 1;
    }
    done.extend(level);
    done.sort_unstable();
    Ok(done)
}

fn run_compact(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cells = compact(cells_input(ctx)?).map_err(|why| {
        ToolError::invalid("/cells", format!("These cells cannot be compacted: {why}."))
    })?;
    Ok(Json::obj([
        ("count", Json::Num(cells.len() as f64)),
        ("cells", list(cells)),
    ]))
}

pub static UNCOMPACT: ToolDef = ToolDef {
    id: "indexing.h3.uncompact",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 uncompact",
    summary: "Expands a mixed-resolution set of H3 cells to one resolution (uncompactCells).",
    aliases: &["uncompactCells"],
    keywords: &["H3", "uncompact", "expand", "hierarchy", "set"],
    inputs: &[CELLS_IN, RES_IN],
    outputs: &[
        count("count", "Cells after", "Expanded count"),
        cell_list("cells", "Cells", "Listed when 10,000 or fewer").optional(),
    ],
    errors: &[ErrorCode::LimitExceeded],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "H3 v4 uncompactCells (h3o 0.11)",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this to flatten a mixed-resolution set back to one resolution, which is what most joins, counts, and exports need: give the set and the resolution and it returns every cell at that level covering the same area. It is the other half of compaction: data is stored or transmitted compacted, then expanded to a single resolution for joining, counting, or rendering, because most consumers assume one cell size.",
    limitations: "Expanding is where a compacted set becomes large: each level multiplies the count by about seven, so uncompacting a coarse set to a fine resolution can produce millions of cells. The target resolution has to be at least as fine as the finest cell in the set. Expansion is exact in area but not in count: cells that were coarse become many, and the memory and join cost follow. Uncompacting to a resolution finer than needed is the usual cause of a query that will not finish.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "892a8471487ffff to resolution 10",
        input: r#"{"cells":[{"cell":"892a8471487ffff"}],"resolution":10}"#,
        source: "H3 uncompactCells",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[
        Related {
            id: "indexing.h3.compact",
            reason: "inverse",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
        Related {
            id: "indexing.h3.children",
            reason: "alternative",
        },
    ],
    sentence: "The set expands to {count} {plural count \"cell\" \"cells\"}.",
    limits: &[("batchRows", 100)],
    run: run_uncompact,
    ..ToolDef::BLANK
};

fn run_uncompact(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cells = cells_input(ctx)?;
    let r = resolution(ctx)?;
    if let Some((i, c)) = cells.iter().enumerate().find(|(_, c)| c.resolution() > r) {
        return Err(ToolError::invalid(
            &format!("/cells/{i}/cell"),
            format!("{c} is finer than resolution {}.", u8::from(r)),
        ));
    }
    let n: u64 = cells.iter().map(|c| c.children_count(r)).sum();
    let mut out = vec![("count", Json::Num(n as f64))];
    if n as usize <= LIST_LIMIT {
        out.push(("cells", list(CellIndex::uncompact(cells, r))));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- edges and vertexes

const EDGE_ITEM: &[Field] = &[
    text("edge", "Directed edge", "H3 edge index"),
    text("to", "Neighbor", "Destination cell"),
    Field::new(
        "length",
        "Length",
        "Edge length",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Significant(6)),
];
const VERTEX_OUT_ITEM: &[Field] = &[
    text("vertex", "Vertex", "H3 vertex index"),
    angle("lat", "Latitude", "[-90,90]"),
    angle("lon", "Longitude", "[-180,180]"),
];

pub static EDGES: ToolDef = ToolDef {
    id: "indexing.h3.edges",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 edges and vertexes",
    summary: "The directed edges (with lengths and neighbors) and vertexes of an H3 cell (originToDirectedEdges, cellToVertexes).",
    aliases: &["originToDirectedEdges", "cellToVertexes", "H3 edge length"],
    keywords: &["H3", "directed edge", "vertex", "edge length", "neighbors"],
    inputs: &[cell_in("cell", "Cell")],
    outputs: &[
        count("edge_count", "Edges", "6, or 5 for a pentagon"),
        Field::new(
            "edges",
            "Directed edges",
            "One per neighbor",
            Kind::List {
                items: EDGE_ITEM,
                min: 0,
                max: 6,
            },
        ),
        Field::new(
            "vertexes",
            "Vertexes",
            "Shared corners",
            Kind::List {
                items: VERTEX_OUT_ITEM,
                min: 0,
                max: 6,
            },
        ),
    ],
    errors: &[],
    warnings: &["PENTAGON_DISTORTION", "EXPERIMENTAL_TOOL"],
    model: "H3 v4 directed edges and vertexes (h3o 0.11)",
    accuracy: "Identical to H3 C",
    when_to_use: "Use this when the boundaries between cells matter rather than the cells: the directed edges leaving a cell, with their lengths and the neighbor each one leads to, and the cell's vertexes. It is what routing, flow, and adjacency work on an H3 grid is built from. Edges are also how a boundary between two regions is expressed on the grid: the edges whose two cells fall on different sides trace it, which is how outlines are extracted from a cell set.",
    limitations: "A hexagon has six edges and a pentagon five, so code that assumes six breaks at the twelve pentagons of each resolution. Edge lengths vary across the grid because H3 cells are not identical, and an edge is a grid relationship rather than a physical boundary on the ground. A directed edge belongs to its origin cell, so an edge and its reverse are different objects, and code that treats them as one counts every boundary twice.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "Edges of 892a8471487ffff",
        input: r#"{"cell":"892a8471487ffff"}"#,
        source: "H3 originToDirectedEdges",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.h3.cell-info",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.grid-disk",
            reason: "alternative",
        },
        Related {
            id: "indexing.h3.lat-lng-to-cell",
            reason: "parent",
        },
    ],
    sentence: "The cell has {edge_count} edges.",
    limits: &[("batchRows", 1_000)],
    run: run_edges,
    ..ToolDef::BLANK
};

fn run_edges(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = cell(ctx, "cell")?;
    if c.is_pentagon() {
        pentagon_note(ctx, "This cell");
    }
    let edges: Vec<Json> = c
        .edges()
        .map(|e| {
            Json::obj([
                ("edge", Json::str(e.to_string())),
                ("to", Json::str(hex(e.destination()))),
                (
                    "length",
                    Json::obj([("value", Json::Num(e.length_m())), ("unit", Json::str("m"))]),
                ),
            ])
        })
        .collect();
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
        ("edge_count", Json::Num(edges.len() as f64)),
        ("edges", Json::Arr(edges)),
        ("vertexes", Json::Arr(vertexes)),
    ]))
}

// ---------------------------------------------------------------- resolution chooser

const RES_ROW: &[Field] = &[
    count("resolution", "Resolution", "0 to 15"),
    Field::new(
        "area",
        "Average area",
        "Hexagon average",
        Kind::Quantity {
            q: QT::Area,
            unit: "km2",
        },
    )
    .precision(Precision::Significant(4)),
    Field::new(
        "edge",
        "Average edge",
        "Hexagon average",
        Kind::Quantity {
            q: QT::Length,
            unit: "km",
        },
    )
    .precision(Precision::Significant(4)),
    count("cells", "Cells on Earth", "Including 12 pentagons"),
];

pub static RESOLUTION_CHOOSER: ToolDef = ToolDef {
    id: "indexing.h3.resolution-chooser",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 resolution chooser",
    summary: "The H3 resolution whose average cell is closest to a target area or edge length, with the full resolution table (122 base cells, 12 pentagons at each resolution).",
    aliases: &["which H3 resolution", "H3 resolution table", "H3 cell size"],
    keywords: &[
        "H3",
        "resolution",
        "cell size",
        "area",
        "edge length",
        "table",
    ],
    inputs: &[
        Field::new(
            "target_area",
            "Target cell area",
            "Like 1 km2",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .core(),
        Field::new(
            "target_edge",
            "Target edge length",
            "Instead of area, like 150 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "km",
            },
        )
        .core(),
    ],
    outputs: &[
        count(
            "resolution",
            "Recommended resolution",
            "Closest average size (log scale)",
        ),
        Field::new(
            "average_area",
            "Average area there",
            "Hexagon average",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(4)),
        count("coarser", "Next coarser", "One resolution up").optional(),
        Field::new(
            "coarser_area",
            "Next coarser average area",
            "Hexagon average",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
        Field::new(
            "table",
            "Resolution table",
            "All 16 resolutions",
            Kind::List {
                items: RES_ROW,
                min: 16,
                max: 16,
            },
        ),
    ],
    errors: &[],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "H3 average hexagon area and edge length per resolution (h3o 0.11 tables, same as H3 C)",
    accuracy: "Averages; real cells vary by about ±50% from center to icosahedron edges",
    when_to_use: "Use this before anything else on an H3 grid: it names the resolution whose average cell is closest to the area or edge length you want, and shows the whole table, so a choice between two levels can be made with the numbers in front of you rather than by trial.",
    limitations: "The table is averages: real cells vary by a few percent, cells are not equal-area, and each resolution has twelve pentagons that are smaller than their neighbors. An average area is a guide to a resolution, not a guarantee about any one cell, and the right resolution also depends on how many cells the work can carry.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "1 km² cells",
        input: r#"{"target_area":"1 km2"}"#,
        source: "add-spatial-indexing-and-raster scenario: resolution 8 (≈ 0.74 km²), 7 is ≈ 5.16 km²",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.h3.lat-lng-to-cell",
            reason: "next",
        },
        Related {
            id: "indexing.h3.polygon-to-cells",
            reason: "next",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "Use resolution {resolution}, averaging {average_area} per cell.",
    limits: &[("batchRows", 1_000)],
    run: run_chooser,
    ..ToolDef::BLANK
};

fn run_chooser(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let km2 = units::by_symbol(QT::Area, "km2").expect("km2");
    let km = units::by_symbol(QT::Length, "km").expect("km");
    let all: Vec<Resolution> = (0..=15u8)
        .map(|r| Resolution::try_from(r).expect("0-15"))
        .collect();
    let pick = match (ctx.quantity("target_area")?, ctx.quantity("target_edge")?) {
        (Some(a), None) => {
            let t = a.to(km2);
            if t <= 0.0 {
                return Err(ToolError::invalid(
                    "/target_area",
                    "The target area must be positive.",
                ));
            }
            all.iter()
                .min_by(|x, y| {
                    libm::log(x.area_km2() / t)
                        .abs()
                        .total_cmp(&libm::log(y.area_km2() / t).abs())
                })
                .copied()
        }
        (None, Some(e)) => {
            let t = e.to(km);
            if t <= 0.0 {
                return Err(ToolError::invalid(
                    "/target_edge",
                    "The target edge must be positive.",
                ));
            }
            all.iter()
                .min_by(|x, y| {
                    libm::log(x.edge_length_km() / t)
                        .abs()
                        .total_cmp(&libm::log(y.edge_length_km() / t).abs())
                })
                .copied()
        }
        _ => {
            return Err(ToolError::invalid(
                "/target_area",
                "Give a target area or a target edge length, not both.",
            ));
        }
    }
    .expect("16 resolutions");
    let r = u8::from(pick);
    let area = |r: Resolution| {
        Json::obj([
            ("value", Json::Num(r.area_km2())),
            ("unit", Json::str("km2")),
        ])
    };
    let table: Vec<Json> = all
        .iter()
        .map(|x| {
            Json::obj([
                ("resolution", Json::Num(f64::from(u8::from(*x)))),
                ("area", area(*x)),
                (
                    "edge",
                    Json::obj([
                        ("value", Json::Num(x.edge_length_km())),
                        ("unit", Json::str("km")),
                    ]),
                ),
                ("cells", Json::Num(x.cell_count() as f64)),
            ])
        })
        .collect();
    let mut out = vec![
        ("resolution", Json::Num(f64::from(r))),
        (
            "average_area",
            ctx.out(
                "average_area",
                Q {
                    value: pick.area_km2(),
                    unit: km2,
                },
            ),
        ),
    ];
    if r > 0 {
        let up = all[usize::from(r) - 1];
        out.push(("coarser", Json::Num(f64::from(r - 1))));
        out.push((
            "coarser_area",
            ctx.out(
                "coarser_area",
                Q {
                    value: up.area_km2(),
                    unit: km2,
                },
            ),
        ));
    }
    out.push(("table", Json::Arr(table)));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- polygon fill

const POINT_ROW: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.4406",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required()
    .angle_range("[-90,90]"),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -80.002",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required()
    .angle_range("[-180,180]"),
];

/// Most cells a fill computes; the web app's estimate gate is 5,000,000.
const FILL_LIMIT: usize = 1_000_000;
const ESTIMATE_LIMIT: f64 = 5_000_000.0;

pub static POLYGON_TO_CELLS: ToolDef = ToolDef {
    id: "indexing.h3.polygon-to-cells",
    stability: gp_base::tool::Stability::Stable,
    title: "H3 polygon fill (polygonToCells)",
    summary: "The H3 cells that fill a polygon (with holes, across the antimeridian) at a resolution, by an explicit containment mode: cell centers inside, whole cells inside, or any overlap.",
    aliases: &["polygonToCells", "polyfill", "H3 polygon to cells", "h3shape_to_cells"],
    keywords: &["H3", "polyfill", "polygonToCells", "polygon", "containment", "coverage", "GeoJSON"],
    inputs: &[
        Field::new("points", "Polygon", "Corners in order, one per row (lat, lon), like 40.4406, -80.002", Kind::List { items: POINT_ROW, min: 3, max: 10_000 }).core(),
        Field::new("geojson", "GeoJSON", "Instead of points: a Polygon or MultiPolygon (holes allowed), like a Polygon with its corners as [lon, lat] pairs", Kind::Text { max_len: 1_000_000 }),
        RES_IN,
        Field::new("containment", "Containment", "center (default: cell centers inside), full (whole cells inside), or overlapping (any overlap)", Kind::Choice(&["center", "full", "overlapping"])).core(),
        Field::new("offset", "List from", "For paging: position of the first cell to list, in index order, like 0", Kind::Number { min: 0.0, max: FILL_LIMIT as f64 }),
        Field::new("limit", "List at most", "For paging: most cells to list (up to 10,000)", Kind::Number { min: 1.0, max: LIST_LIMIT as f64 }),
    ],
    outputs: &[
        count("count", "Cells", "Number of cells"),
        text("containment", "Containment used", "Echoes the mode"),
        count("estimate", "Estimate", "Bounding-box estimate made before filling"),
        Field::new("area", "Covered area", "Sum of the cells' exact areas", Kind::Quantity { q: QT::Area, unit: "km2" }).precision(Precision::Significant(6)),
        angle("south", "South", "[-90,90]").optional(),
        angle("west", "West", "[-180,180]").optional(),
        angle("north", "North", "[-90,90]").optional(),
        angle("east", "East", "[-180,180]").optional(),
        cell_list("cells", "Cells", "Listed when 10,000 or fewer, or the page asked for with offset and limit").optional(),
        count("compacted_count", "Compacted cells", "After compaction").optional(),
        cell_list("compacted", "Compacted set", "Mixed resolutions, when the full list is too long").optional(),
    ],
    errors: &[ErrorCode::LimitExceeded],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "H3 C polygonToCells containment tests (point in polygon on latitude and longitude, boundary crossings), breadth-first fill from cells along every edge",
    accuracy: "Identical cell sets to H3 C 4.4.1 in center, full, and overlapping modes on 1,000 random polygons with holes and antimeridian crossings",
    when_to_use: "Use this to turn an area into cells: the H3 cells that fill a polygon, with holes and across the antimeridian, at a resolution you choose. It is how a region, a service area, or an administrative boundary becomes a set of keys that rows can be joined on.",
    limitations: "Which cells count as inside is a choice, not a fact, so the containment mode has to be stated: centers inside, whole cells inside, or any overlap all give different sets, and the difference is largest at coarse resolutions relative to the polygon. A fine resolution over a large area produces very many cells.",
    references: &[H3_DOCS, H3O],
    examples: &[Example {
        id: "primary",
        title: "A block in downtown Pittsburgh at resolution 9",
        input: r#"{"points":[{"lat":40.4406,"lon":-80.0020},{"lat":40.4406,"lon":-79.9900},{"lat":40.4480,"lon":-79.9900},{"lat":40.4480,"lon":-80.0020}],"resolution":9,"containment":"center"}"#,
        source: "H3 polygonToCells",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "cells")],
    }],
    related: &[Related { id: "indexing.h3.compact", reason: "next" },
        Related {
            id: "indexing.h3.resolution-chooser",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.cell-info",
            reason: "next",
        },
    ],
    sentence: "The polygon fills with {count} {plural count \"cell\" \"cells\"} by {containment} containment.",
    limits: &[("batchRows", 10)],
    run: run_polygon_to_cells,
    ..ToolDef::BLANK
};

/// South, west, north, east over every cell's boundary. West is greater than
/// east when the cells cross the antimeridian.
fn cells_bbox(cells: &[CellIndex]) -> Option<[f64; 4]> {
    let (mut s, mut n) = (f64::INFINITY, f64::NEG_INFINITY);
    // Longitudes as given and shifted to [0, 360); the narrower span wins.
    let (mut w1, mut e1, mut w2, mut e2) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for c in cells {
        for v in c.boundary().iter() {
            let (la, lo) = (v.lat(), v.lng());
            s = s.min(la);
            n = n.max(la);
            w1 = w1.min(lo);
            e1 = e1.max(lo);
            let lo2 = if lo < 0.0 { lo + 360.0 } else { lo };
            w2 = w2.min(lo2);
            e2 = e2.max(lo2);
        }
    }
    if !s.is_finite() {
        return None;
    }
    let wrap = |x: f64| if x > 180.0 { x - 360.0 } else { x };
    Some(if e2 - w2 < e1 - w1 {
        [s, wrap(w2), n, wrap(e2)]
    } else {
        [s, w1, n, e1]
    })
}

/// Rings of (lat, lon) degrees: outer ring first, then holes.
type Rings = Vec<Vec<(f64, f64)>>;

fn geojson_rings(text: &str) -> Result<Vec<Rings>, String> {
    let v: Value =
        serde_json::from_str(text).map_err(|_| "The GeoJSON is not valid JSON.".to_owned())?;
    let geom = if v["type"] == "Feature" {
        &v["geometry"]
    } else {
        &v
    };
    let ring = |r: &Value| -> Result<Vec<(f64, f64)>, String> {
        r.as_array()
            .ok_or("A ring must be an array of [longitude, latitude] positions.")?
            .iter()
            .map(|p| {
                match (
                    p.get(0).and_then(Value::as_f64),
                    p.get(1).and_then(Value::as_f64),
                ) {
                    (Some(lng), Some(lat))
                        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng) =>
                    {
                        Ok((lat, lng))
                    }
                    _ => Err("Each position is [longitude, latitude] within range.".to_owned()),
                }
            })
            .collect()
    };
    let polygon = |p: &Value| -> Result<Rings, String> {
        p.as_array()
            .ok_or("A polygon is an array of rings.")?
            .iter()
            .map(ring)
            .collect()
    };
    match geom["type"].as_str() {
        Some("Polygon") => Ok(vec![polygon(&geom["coordinates"])?]),
        Some("MultiPolygon") => geom["coordinates"]
            .as_array()
            .ok_or("Bad MultiPolygon.")?
            .iter()
            .map(polygon)
            .collect(),
        _ => Err("Give a GeoJSON Polygon or MultiPolygon (or a Feature holding one).".to_owned()),
    }
}

fn run_polygon_to_cells(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use crate::h3fill::{Mode, Polygon, edge_samples, estimate, fill};
    let res = resolution(ctx)?;
    let mode = match ctx.choice("containment")?.unwrap_or("center") {
        "full" => Mode::Full,
        "overlapping" => Mode::Overlap,
        _ => Mode::Center,
    };
    let polys: Vec<Rings> = match (ctx.is_set("points"), ctx.is_set("geojson")) {
        (true, false) => {
            let rows = ctx.rows("points")?;
            let mut ring = Vec::with_capacity(rows.len());
            for (i, r) in rows.iter().enumerate() {
                let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
                let la = ctx
                    .row_quantity("points", i, r, "lat")?
                    .expect("required")
                    .to(deg);
                let lo = ctx
                    .row_quantity("points", i, r, "lon")?
                    .expect("required")
                    .to(deg);
                if !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
                    return Err(ToolError::invalid(
                        &format!("/points/{i}"),
                        "Latitude is −90 to 90 and longitude −180 to 180.",
                    ));
                }
                ring.push((la, lo));
            }
            vec![vec![ring]]
        }
        (false, true) => geojson_rings(&ctx.text("geojson")?.expect("set"))
            .map_err(|m| ToolError::invalid("/geojson", m))?,
        _ => {
            return Err(ToolError::invalid(
                "/points",
                "Give the polygon as points or as GeoJSON, not both.",
            ));
        }
    };
    // Paging, checked before the fill so a bad page costs nothing.
    let whole = |ctx: &Ctx, name: &str| -> Result<Option<usize>, ToolError> {
        match ctx.number(name)? {
            Some(x) if x.fract() != 0.0 => Err(ToolError::invalid(
                &format!("/{name}"),
                "Give a whole number.",
            )),
            Some(x) => Ok(Some(x as usize)),
            None => Ok(None),
        }
    };
    let (offset, limit) = (whole(ctx, "offset")?, whole(ctx, "limit")?);
    let page = (offset.is_some() || limit.is_some())
        .then(|| (offset.unwrap_or(0), limit.unwrap_or(LIST_LIMIT)));
    let mut total_estimate = 0.0;
    let mut total_samples = 0.0;
    let mut built = Vec::new();
    for rings in &polys {
        if rings.is_empty() || rings.iter().any(|r| r.len() < 3) {
            return Err(ToolError::invalid(
                "/geojson",
                "Every ring needs at least 3 corners.",
            ));
        }
        let p = Polygon::new(rings);
        total_estimate += estimate(&p, res);
        total_samples += edge_samples(&p, res);
        built.push(p);
    }
    if total_estimate > ESTIMATE_LIMIT {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "About {} cells would result, over the 5,000,000 limit.",
                total_estimate.round() as u64
            ),
        )
        .at("/resolution")
        .hint("Choose a coarser resolution, or fill in pieces and compact the result."));
    }
    // Long edges at a fine resolution cost time even when the area is small
    // (about 2 µs per sample); 2,000,000 samples keeps a call to a few seconds.
    if total_samples > 2_000_000.0 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            "The polygon's edges are too long for this resolution.",
        )
        .at("/resolution")
        .hint("Choose a coarser resolution, or split the polygon into smaller pieces."));
    }
    let mut cells = std::collections::BTreeSet::new();
    for p in &built {
        let got = fill(p, res, mode, FILL_LIMIT).map_err(|n| {
            ToolError::new(
                ErrorCode::LimitExceeded,
                format!("More than {n} cells, over the {FILL_LIMIT} limit for one call."),
            )
            .at("/resolution")
        })?;
        cells.extend(got);
    }
    let cells: Vec<CellIndex> = cells.into_iter().collect();
    let area: f64 = cells.iter().map(|c| c.area_km2()).sum();
    let bbox = cells_bbox(&cells);
    let name = match mode {
        Mode::Center => "center",
        Mode::Full => "full",
        Mode::Overlap => "overlapping",
    };
    let mut out = vec![
        ("count", Json::Num(cells.len() as f64)),
        ("containment", Json::str(name)),
        ("estimate", Json::Num(total_estimate.round())),
        (
            "area",
            ctx.out(
                "area",
                Q {
                    value: area,
                    unit: units::by_symbol(QT::Area, "km2").expect("km2"),
                },
            ),
        ),
    ];
    if let Some([s, w, n, e]) = bbox {
        out.push(("south", ctx.out("south", deg(s))));
        out.push(("west", ctx.out("west", deg(w))));
        out.push(("north", ctx.out("north", deg(n))));
        out.push(("east", ctx.out("east", deg(e))));
    }
    if let Some((from, take)) = page {
        let from = from.min(cells.len());
        out.push(("cells", list(cells[from..].iter().take(take).copied())));
    } else if cells.len() <= LIST_LIMIT {
        out.push(("cells", list(cells)));
    } else if let Ok(small) = compact(cells) {
        out.push(("compacted_count", Json::Num(small.len() as f64)));
        if small.len() <= LIST_LIMIT {
            out.push(("compacted", list(small)));
        }
    }
    Ok(Json::obj(out))
}
