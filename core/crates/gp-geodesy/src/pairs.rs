//! Generated conversion endpoints for the national grids
//! (add-international-grids task 7): the four pairs the design names, each a
//! parent tool with its datum fixed, under the id people search for. They run
//! the parent's own function; nothing here is math. Until one is listed in
//! `data/seo/high-intent-pages.json` it renders as a noindex preset of its
//! parent, canonical to the parent's page.

use gp_base::tool::{Example, Related, ToolDef};

use crate::gridref::{BNG_FORWARD, BNG_INVERSE};
use crate::methods::{MGA_INVERSE, NZTM_INVERSE};

macro_rules! grid_pair {
    ($name:ident, $parent:ident, $parent_id:literal, $id:literal, $title:literal,
     $summary:literal, $inputs:expr, $preset:expr, $example_title:literal, $example:literal,
     $source:literal, $sentence:literal, $why:literal) => {
        pub static $name: ToolDef = ToolDef {
            id: $id,
            title: $title,
            summary: $summary,
            keywords: $parent.keywords,
            inputs: $inputs,
            outputs: $parent.outputs,
            stability: $parent.stability,
            errors: $parent.errors,
            warnings: $parent.warnings,
            model: $parent.model,
            accuracy: $parent.accuracy,
            limitations: $parent.limitations,
            references: $parent.references,
            examples: &[Example {
                id: "primary",
                title: $example_title,
                input: $example,
                source: $source,
            }],
            primary_example: "primary",
            visualization: $parent.visualization,
            related: &[Related {
                id: $parent_id,
                reason: "parent",
            }],
            composed_of: &[$parent_id],
            parent: Some(&$parent),
            preset: $preset,
            justification: $why,
            assumptions: $parent.assumptions,
            sentence: $sentence,
            limits: $parent.limits,
            run: $parent.run,
            ..ToolDef::BLANK
        };
    };
}

/// The datum a phone or a web map reads: the pairs that leave the grid for
/// latitude and longitude, or start from them, fix it to WGS 84.
const ON_WGS84: &[(&str, &str)] = &[("datum", "\"WGS84\"")];
const ON_MGA2020: &[(&str, &str)] = &[("datum", "\"MGA2020\"")];

// The parents' inputs without the field each pair fixes. The indexes are
// checked against the parents' own field names in tests/pairs.rs, so a
// reordered parent fails a test rather than quietly dropping the wrong field.
const LATLON_TO_BNG_INPUTS: &[gp_base::tool::Field] = &[
    BNG_FORWARD.inputs[0],
    BNG_FORWARD.inputs[1],
    BNG_FORWARD.inputs[3],
];
const BNG_TO_LATLON_INPUTS: &[gp_base::tool::Field] = &[BNG_INVERSE.inputs[0]];
const MGA_TO_LATLON_INPUTS: &[gp_base::tool::Field] = &[
    MGA_INVERSE.inputs[0],
    MGA_INVERSE.inputs[1],
    MGA_INVERSE.inputs[2],
];

grid_pair!(
    LATLON_TO_BNG,
    BNG_FORWARD,
    "geodesy.grid-ref.bng-forward",
    "geodesy.grid-ref.latlon-to-bng",
    "GPS latitude and longitude to a National Grid reference",
    "Turns a WGS 84 position, the kind a phone or a GPS gives, into its British National Grid reference, like TQ 30624 78388.",
    LATLON_TO_BNG_INPUTS,
    ON_WGS84,
    "The Tower of London",
    r#"{"lat":51.508056,"lon":-0.076111,"figures":10}"#,
    "The parent tool's projection and EPSG 1314 datum step, with this pair's WGS 84 preset; checked against PROJ 9.9.0",
    "That point is {reference}.",
    "People asking for a grid reference from latitude and longitude almost always hold WGS 84 coordinates from a phone or a GPS, and the parent's OSGB36 choice is the one most often picked wrongly."
);

grid_pair!(
    BNG_TO_LATLON,
    BNG_INVERSE,
    "geodesy.grid-ref.bng-inverse",
    "geodesy.grid-ref.bng-to-latlon",
    "National Grid reference to GPS latitude and longitude",
    "Reads a British National Grid reference like SU 14 42 and gives the WGS 84 latitude and longitude of the square's center, the coordinates a phone or a web map wants.",
    BNG_TO_LATLON_INPUTS,
    ON_WGS84,
    "Stonehenge's square",
    r#"{"reference":"SU 1224 4219"}"#,
    "The parent tool's projection and EPSG 1314 datum step, with this pair's WGS 84 preset; checked against PROJ 9.9.0",
    "That reference names a {square_size} square centered on {lat}, {lon} on WGS 84.",
    "A grid reference is read off a map to be found on a phone, which works in WGS 84; fixing the datum spares the reader the OSGB36 choice and its 100 m error."
);

grid_pair!(
    MGA_TO_LATLON,
    MGA_INVERSE,
    "geodesy.projection.mga-inverse",
    "geodesy.projection.mga-to-latlon",
    "MGA2020 coordinates to GDA2020 latitude and longitude",
    "Reads an easting and northing on a Map Grid of Australia 2020 zone and gives the GDA2020 latitude and longitude, with the grid convergence and the point scale factor there.",
    MGA_TO_LATLON_INPUTS,
    ON_MGA2020,
    "Uluru, in zone 52",
    r#"{"zone":52,"easting":"704993.882 m","northing":"7195356.236 m"}"#,
    "The parent tool's inverse transverse Mercator on the zone's EPSG parameters, with this pair's MGA2020 preset; checked against PROJ 9.9.0",
    "That is {lat}, {lon} on GDA2020.",
    "Australian coordinates published since 2020 are MGA2020, and the pair spares the reader the MGA94 choice, which reads the same numbers to a datum 1.8 m away."
);

grid_pair!(
    NZTM_TO_LATLON,
    NZTM_INVERSE,
    "geodesy.projection.nztm-inverse",
    "geodesy.projection.nztm-to-latlon",
    "NZTM2000 coordinates to NZGD2000 latitude and longitude",
    "Reads a New Zealand Transverse Mercator 2000 easting and northing and gives the NZGD2000 latitude and longitude, with the grid convergence and the point scale factor there.",
    NZTM_INVERSE.inputs,
    &[],
    "Queenstown",
    r#"{"easting":"1258332.205 m","northing":"5004425.269 m"}"#,
    "The parent tool's inverse transverse Mercator on EPSG 2193's parameters; checked against PROJ 9.9.0",
    "That is {lat}, {lon} on NZGD2000.",
    "NZTM2000 to latitude and longitude is the conversion New Zealand map and council data needs most, and the pair names it the way it is asked for."
);
