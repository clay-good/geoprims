//! A5 equal-area pentagonal grid tools, pinned to A5 0.10.1.
//! A5 remains experimental while its reference implementation is pre-1.0.

use a5::{
    CellToBoundaryOptions, LonLat, cell_area, cell_to_boundary, cell_to_children, cell_to_lonlat,
    cell_to_parent, get_resolution, lonlat_to_cell,
};
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;
use serde_json::Value;

const A5_DOCS: Reference = Reference {
    title: "A5 API reference",
    issuer: "A5 project",
    year: 2026,
    edition: "a5-js 0.10.1",
    locator: "Indexing, hierarchy, and cell-info APIs",
    url: "https://a5geo.org/docs/api-reference",
};
const A5_RS: Reference = Reference {
    title: "a5-rs: Rust implementation of A5",
    issuer: "A5 contributors",
    year: 2026,
    edition: "0.10.1",
    locator: "lonlat_to_cell, cell_to_boundary, hierarchy, and cell_area",
    url: "https://docs.rs/a5/0.10.1/a5/",
};

const RESOLUTION: Field = Field::new(
    "resolution",
    "Resolution",
    "0 (coarsest) to 30 (finest)",
    Kind::Number {
        min: 0.0,
        max: 30.0,
    },
)
.required()
.core();

const fn text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 32 })
}

const fn angle(name: &'static str, title: &'static str, range: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Decimal degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(9))
    .angle_range(range)
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

const fn cell_input() -> Field {
    Field::new(
        "cell",
        "A5 cell",
        "Hexadecimal A5 index, with or without 0x",
        Kind::Text { max_len: 24 },
    )
    .required()
    .core()
}

const VERTEX_FIELDS: &[Field] = &[
    angle("lat", "Latitude", "[-90,90]"),
    angle("lon", "Longitude", "[-180,180]"),
];
const CELL_FIELDS: &[Field] = &[text("cell", "Cell", "A5 index (hex)")];

fn deg(value: f64) -> Q {
    Q {
        value,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

fn area(value_m2: f64) -> Q {
    Q {
        value: value_m2 / 1e6,
        unit: units::by_symbol(QT::Area, "km2").expect("km2"),
    }
}

fn resolution(ctx: &Ctx) -> Result<i32, ToolError> {
    let value = ctx.number("resolution")?.expect("required");
    if value.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/resolution",
            "The resolution is a whole number from 0 to 30.",
        ));
    }
    Ok(value as i32)
}

fn hex(cell: u64) -> String {
    format!("{cell:016x}")
}

fn parse_cell(value: Option<&Value>) -> Result<u64, ToolError> {
    let raw = match value {
        Some(Value::String(value)) => value.trim(),
        Some(Value::Number(_)) => {
            return Err(ToolError::invalid(
                "/cell",
                "A5 indexes are 64-bit, which a JSON number cannot hold exactly. Pass the hexadecimal index as a string.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/cell",
                "Give the A5 index as a hexadecimal string.",
            ));
        }
    };
    let digits = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    let cell = u64::from_str_radix(digits, 16)
        .map_err(|_| ToolError::invalid("/cell", format!("\"{raw}\" is not an A5 cell index.")))?;
    if cell == 0 || cell_to_lonlat(cell).is_err() {
        return Err(ToolError::invalid(
            "/cell",
            format!("\"{raw}\" is not a bounded A5 cell index."),
        ));
    }
    Ok(cell)
}

fn cell(ctx: &Ctx) -> Result<u64, ToolError> {
    parse_cell(ctx.raw("cell"))
}

fn library_error(field: &str, error: String) -> ToolError {
    ToolError::invalid(field, format!("A5 rejected this value: {error}."))
}

pub static LON_LAT_TO_CELL: ToolDef = ToolDef {
    id: "indexing.a5.lon-lat-to-cell",
    title: "A5 cell from latitude and longitude",
    summary: "The equal-area A5 pentagonal cell containing a point at resolution 0 to 30, with its center and area.",
    aliases: &["A5 index", "lonLatToCell", "point to A5 cell"],
    keywords: &["A5", "pentagon", "DGGS", "equal area", "spatial index"],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        RESOLUTION,
    ],
    outputs: &[
        text("cell", "A5 cell", "16-digit hexadecimal index"),
        angle("center_lat", "Center latitude", "[-90,90]"),
        angle("center_lon", "Center longitude", "[-180,180]"),
        Field::new(
            "area",
            "Cell area",
            "Equal for every cell at this resolution",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(7)),
    ],
    errors: &[],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "A5 0.10.1, pinned to a5-js and a5-rs 0.10.1",
    accuracy: "A 120-case fixture spans every resolution, the poles, the antimeridian, and fixed landmarks. Cell indexes match a5-js 0.10.1 exactly; centers agree within 1e-9°. Cell areas agree to the fixture's 1e-14 relative tolerance in testing.",
    when_to_use: "Use this to group points into equal-area cells without latitude bias. Choose a coarse resolution for global summaries and a fine one for local bins; the index can then be stored, sorted, or passed to the boundary and hierarchy tools.",
    limitations: "A5 0.10.1 is pre-1.0, so its index encoding and API can still change. Resolution 30 cannot encode every quintant and can fall back to resolution 29; always read the returned index rather than assuming the requested resolution. A5 uses an authalic Earth model, not an ellipsoidal parcel boundary.",
    references: &[A5_DOCS, A5_RS],
    examples: &[Example {
        id: "primary",
        title: "London at resolution 10",
        input: r#"{"lat":51.5074,"lon":-0.1276,"resolution":10}"#,
        source: "a5-js 0.10.1 index-encoding example",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cell", "cell")],
    }],
    related: &[
        Related {
            id: "indexing.a5.cell-boundary",
            reason: "inverse",
        },
        Related {
            id: "indexing.a5.hierarchy",
            reason: "next",
        },
        Related {
            id: "indexing.h3.lat-lng-to-cell",
            reason: "alternative",
        },
    ],
    sentence: "The point is in A5 cell {cell}, with an area of {area}.",
    limits: &[("batchRows", 10_000)],
    run: run_lon_lat_to_cell,
    ..ToolDef::BLANK
};

fn run_lon_lat_to_cell(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let requested = resolution(ctx)?;
    let cell = lonlat_to_cell(LonLat::new(lon, lat), requested)
        .map_err(|error| library_error("/resolution", error))?;
    let center = cell_to_lonlat(cell).map_err(|error| library_error("/resolution", error))?;
    let actual = get_resolution(cell);
    Ok(Json::obj([
        ("cell", Json::str(hex(cell))),
        ("center_lat", ctx.out("center_lat", deg(center.latitude()))),
        ("center_lon", ctx.out("center_lon", deg(center.longitude()))),
        ("area", ctx.out("area", area(cell_area(actual)))),
    ]))
}

pub static CELL_BOUNDARY: ToolDef = ToolDef {
    id: "indexing.a5.cell-boundary",
    title: "A5 cell boundary",
    summary: "The center, equal area, resolution, and closed curved-edge boundary of an A5 cell for inspection or mapping.",
    aliases: &["A5 cellToBoundary", "decode A5", "A5 cell info"],
    keywords: &["A5", "boundary", "pentagon", "cell area", "center"],
    inputs: &[cell_input()],
    outputs: &[
        text("cell", "A5 cell", "Normalized 16-digit hexadecimal index"),
        count("resolution", "Resolution", "0 to 30"),
        angle("lat", "Center latitude", "[-90,90]"),
        angle("lon", "Center longitude", "[-180,180]"),
        Field::new(
            "area",
            "Cell area",
            "Equal for every cell at this resolution",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(7)),
        Field::new(
            "boundary",
            "Boundary",
            "Counterclockwise points; the final point repeats the first",
            Kind::List {
                items: VERTEX_FIELDS,
                min: 4,
                max: 321,
            },
        ),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "A5 0.10.1 boundary on the authalic sphere",
    accuracy: "Every vertex in the 120-case fixture agrees with a5-js 0.10.1 within 1e-9°. The fixture covers all resolutions, both poles, and antimeridian cells across the tested globe.",
    when_to_use: "Use this to inspect or draw an A5 index. It returns the cell center, its exact equal-area value, and a boundary with enough edge segments for the resolution. Use the returned closed ring directly for a map, selection preview, or spatial-data export.",
    limitations: "The returned points approximate A5's curved spherical edges with line segments; low-resolution cells therefore have many points and larger payloads. The area comes from A5's equal-area model and is more authoritative than recomputing area from the segmented boundary. The polygon is for display and exchange, not a surveyed boundary. This is the pinned pre-1.0 encoding.",
    references: &[A5_DOCS, A5_RS],
    examples: &[Example {
        id: "primary",
        title: "London resolution 10 cell",
        input: r#"{"cell":"63611a8000000000"}"#,
        source: "A5 0.10.1 index-encoding example",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cell", "cell")],
    }],
    related: &[
        Related {
            id: "indexing.a5.lon-lat-to-cell",
            reason: "inverse",
        },
        Related {
            id: "indexing.a5.hierarchy",
            reason: "next",
        },
    ],
    sentence: "A5 cell {cell} is at resolution {resolution}, centered at {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_cell_boundary,
    ..ToolDef::BLANK
};

fn run_cell_boundary(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = cell(ctx)?;
    let resolution = get_resolution(cell);
    let center = cell_to_lonlat(cell).map_err(|error| library_error("/cell", error))?;
    let boundary = cell_to_boundary(
        cell,
        Some(CellToBoundaryOptions {
            closed_ring: true,
            segments: None,
        }),
    )
    .map_err(|error| library_error("/cell", error))?
    .into_iter()
    .map(|point| {
        Json::obj([
            ("lat", Json::Num(point.latitude())),
            ("lon", Json::Num(point.longitude())),
        ])
    })
    .collect();
    Ok(Json::obj([
        ("cell", Json::str(hex(cell))),
        ("resolution", Json::Num(resolution as f64)),
        ("lat", ctx.out("lat", deg(center.latitude()))),
        ("lon", ctx.out("lon", deg(center.longitude()))),
        ("area", ctx.out("area", area(cell_area(resolution)))),
        ("boundary", Json::Arr(boundary)),
    ]))
}

pub static HIERARCHY: ToolDef = ToolDef {
    id: "indexing.a5.hierarchy",
    title: "A5 parent and children",
    summary: "The immediate parent and children of an A5 cell in its logical resolution hierarchy.",
    aliases: &["A5 cellToParent", "A5 cellToChildren", "A5 hierarchy"],
    keywords: &["A5", "parent", "children", "hierarchy", "resolution"],
    inputs: &[cell_input()],
    outputs: &[
        count("resolution", "Resolution", "The input cell's resolution"),
        text("parent", "Parent", "One resolution coarser"),
        Field::new(
            "children",
            "Children",
            "One resolution finer",
            Kind::List {
                items: CELL_FIELDS,
                min: 0,
                max: 5,
            },
        ),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "A5 0.10.1 logical hierarchy",
    accuracy: "Parent and child indexes match a5-js 0.10.1 exactly, with exact ordering, in a 120-case fixture that spans every resolution, the poles, the antimeridian, and fixed landmarks.",
    when_to_use: "Use this to move one level through the A5 hierarchy: the parent is the coarser bucket and the children are the finer buckets used to refine a selection. It is useful for rollups, drill-down views, storage partitions, and map level-of-detail changes.",
    limitations: "A5's hierarchy is logical: a parent and its children are near one another but the child polygons do not exactly partition the parent polygon, so do not use the children as a geometric subdivision. Resolution 0 has the world cell as its parent and five resolution 1 children; ordinary higher-resolution cells have four children. Resolution 30 has no finer children.",
    references: &[A5_DOCS, A5_RS],
    examples: &[Example {
        id: "primary",
        title: "London resolution 10 hierarchy",
        input: r#"{"cell":"63611a8000000000"}"#,
        source: "A5 0.10.1 hierarchy API",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "cell-set",
        map: &[("cells", "children")],
    }],
    related: &[
        Related {
            id: "indexing.a5.cell-boundary",
            reason: "parent",
        },
        Related {
            id: "indexing.a5.lon-lat-to-cell",
            reason: "next",
        },
    ],
    sentence: "The parent is {parent}; this cell has {children:count} immediate children.",
    limits: &[("batchRows", 10_000)],
    run: run_hierarchy,
    ..ToolDef::BLANK
};

fn run_hierarchy(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = cell(ctx)?;
    let resolution = get_resolution(cell);
    if resolution >= 30 {
        return Err(ToolError::invalid(
            "/cell",
            "A resolution 30 cell has no finer children.",
        ));
    }
    let parent = cell_to_parent(cell, None).map_err(|error| library_error("/cell", error))?;
    let children = cell_to_children(cell, None)
        .map_err(|error| library_error("/cell", error))?
        .into_iter()
        .map(|child| Json::obj([("cell", Json::str(hex(child)))]))
        .collect();
    Ok(Json::obj([
        ("resolution", Json::Num(resolution as f64)),
        ("parent", Json::str(hex(parent))),
        ("children", Json::Arr(children)),
    ]))
}
