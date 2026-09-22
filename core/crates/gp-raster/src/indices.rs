//! Spectral indices (raster/imagery-indices spec): the normalized differences
//! and their relatives, computed on single surface-reflectance values.
//!
//! Every index here is a ratio of reflectances, so the inputs have to be
//! reflectance on a 0-1 scale rather than raw digital numbers: a scene's DNs
//! give a number that looks like an index and is not one. Values far outside
//! that range raise `SUSPECT_SCALING` rather than being silently scaled, and a
//! denominator of zero is refused rather than returned as infinity.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};

pub const ROUSE: Reference = Reference {
    title: "Monitoring vegetation systems in the Great Plains with ERTS",
    issuer: "Rouse, J. W., Haas, R. H., Schell, J. A., and Deering, D. W., NASA",
    year: 1974,
    edition: "Third ERTS Symposium, NASA SP-351, volume 1, pages 309-317",
    locator: "The normalized difference of the near-infrared and red bands",
    url: "https://ntrs.nasa.gov/citations/19740022614",
};
pub const MCFEETERS: Reference = Reference {
    title: "The use of the Normalized Difference Water Index (NDWI) in the delineation of open water features",
    issuer: "McFeeters, S. K., International Journal of Remote Sensing",
    year: 1996,
    edition: "Volume 17, issue 7, pages 1425-1432",
    locator: "NDWI = (Green - NIR) / (Green + NIR), for open water",
    url: "https://doi.org/10.1080/01431169608948714",
};
pub const GAO: Reference = Reference {
    title: "NDWI - A normalized difference water index for remote sensing of vegetation liquid water from space",
    issuer: "Gao, B.-C., Remote Sensing of Environment",
    year: 1996,
    edition: "Volume 58, issue 3, pages 257-266",
    locator: "NDWI = (NIR - SWIR) / (NIR + SWIR), for water in vegetation",
    url: "https://doi.org/10.1016/S0034-4257(96)00067-3",
};
pub const XU: Reference = Reference {
    title: "Modification of normalised difference water index (NDWI) to enhance open water features in remotely sensed imagery",
    issuer: "Xu, H., International Journal of Remote Sensing",
    year: 2006,
    edition: "Volume 27, issue 14, pages 3025-3033",
    locator: "MNDWI = (Green - SWIR1) / (Green + SWIR1)",
    url: "https://doi.org/10.1080/01431160600589179",
};
pub const HUETE_2002: Reference = Reference {
    title: "Overview of the radiometric and biophysical performance of the MODIS vegetation indices",
    issuer: "Huete, A., Didan, K., Miura, T., Rodriguez, E. P., Gao, X., and Ferreira, L. G., Remote Sensing of Environment",
    year: 2002,
    edition: "Volume 83, issues 1-2, pages 195-213",
    locator: "EVI = 2.5 (NIR - Red) / (NIR + 6 Red - 7.5 Blue + 1)",
    url: "https://doi.org/10.1016/S0034-4257(02)00096-2",
};
pub const JIANG: Reference = Reference {
    title: "Development of a two-band enhanced vegetation index without a blue band",
    issuer: "Jiang, Z., Huete, A. R., Didan, K., and Miura, T., Remote Sensing of Environment",
    year: 2008,
    edition: "Volume 112, issue 10, pages 3833-3845",
    locator: "EVI2 = 2.5 (NIR - Red) / (NIR + 2.4 Red + 1)",
    url: "https://doi.org/10.1016/j.rse.2008.06.006",
};
pub const HUETE_1988: Reference = Reference {
    title: "A soil-adjusted vegetation index (SAVI)",
    issuer: "Huete, A. R., Remote Sensing of Environment",
    year: 1988,
    edition: "Volume 25, issue 3, pages 295-309",
    locator: "SAVI = (1 + L)(NIR - Red) / (NIR + Red + L), with L = 0.5 for intermediate cover",
    url: "https://doi.org/10.1016/0034-4257(88)90106-X",
};
pub const ZHA: Reference = Reference {
    title: "Use of normalized difference built-up index in automatically mapping urban areas from TM imagery",
    issuer: "Zha, Y., Gao, J., and Ni, S., International Journal of Remote Sensing",
    year: 2003,
    edition: "Volume 24, issue 3, pages 583-594",
    locator: "NDBI = (SWIR1 - NIR) / (SWIR1 + NIR)",
    url: "https://doi.org/10.1080/01431160304987",
};
pub const KEY_BENSON: Reference = Reference {
    title: "Landscape Assessment (LA): sampling and analysis methods, in FIREMON: Fire Effects Monitoring and Inventory System",
    issuer: "Key, C. H., and Benson, N. C., USDA Forest Service, Rocky Mountain Research Station",
    year: 2006,
    edition: "General Technical Report RMRS-GTR-164-CD, pages LA-1 to LA-55",
    locator: "NBR and dNBR, and the burn-severity ranges offered as a starting point rather than a rule",
    url: "https://www.fs.usda.gov/research/treesearch/24066",
};

/// Reflectance outside this range is not reflectance: raw DNs, a missing
/// scaling factor, or the wrong product level.
const PLAUSIBLE: (f64, f64) = (-0.2, 1.5);

const fn band(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -10_000.0,
            max: 10_000.0,
        },
    )
}

const fn index_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(4))
}

const TABLE: Layer = Layer {
    kind: "table-only",
    map: &[],
};

/// Reads a required band, flagging values that cannot be reflectance.
fn refl(ctx: &mut Ctx, name: &str) -> Result<f64, ToolError> {
    let v = ctx.number(name)?.expect("required");
    if v < PLAUSIBLE.0 || v > PLAUSIBLE.1 {
        ctx.warnings.push(
            Warning::new(
                "SUSPECT_SCALING",
                format!(
                    "{v} is outside the reflectance range of {} to {}, so it looks like a raw digital number rather than reflectance. Scale the band to reflectance first: Sentinel-2 L2A is (DN - 1000) / 10000 from processing baseline 04.00, and Landsat Collection 2 Level-2 is DN x 0.0000275 - 0.2.",
                    PLAUSIBLE.0, PLAUSIBLE.1
                ),
            )
            .at(&format!("/{name}")),
        );
    }
    Ok(v)
}

/// A normalized difference (a - b) / (a + b), refusing a zero denominator.
fn normalized(a: f64, b: f64, names: (&str, &str)) -> Result<f64, ToolError> {
    let sum = a + b;
    if sum == 0.0 {
        return Err(ToolError::invalid(
            &format!("/{}", names.0),
            format!(
                "{} + {} is zero, so the index is undefined here; in an image this pixel is no-data.",
                names.0, names.1
            ),
        )
        .hint("Check the bands are reflectance and that this pixel is not masked."));
    }
    Ok((a - b) / sum)
}

macro_rules! normalized_tool {
    ($static:ident, $id:literal, $title:literal, $summary:literal, $a:literal, $a_title:literal,
     $a_help:literal, $b:literal, $b_title:literal, $b_help:literal, $out:literal, $out_title:literal, $formula:literal,
     $reference:expr, $aliases:expr, $keywords:expr, $example:literal, $source:literal,
     $sentence:literal, $model:literal, $accuracy:literal, $when:literal, $limits:literal,
     $related:expr, $run:ident) => {
        pub static $static: ToolDef = ToolDef {
            id: $id,
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: &[
                band($a, $a_title, $a_help).required().core(),
                band($b, $b_title, $b_help).required().core(),
            ],
            outputs: &[index_out($out, $out_title, $formula)],
            errors: &[ErrorCode::InvalidInput],
            warnings: &["SUSPECT_SCALING", "EXPERIMENTAL_TOOL"],
            model: $model,
            accuracy: $accuracy,
            when_to_use: $when,
            limitations: $limits,
            references: &[$reference],
            examples: &[Example {
                id: "primary",
                title: $title,
                input: $example,
                source: $source,
            }],
            primary_example: "primary",
            visualization: &[TABLE],
            related: $related,
            sentence: $sentence,
            limits: &[("batchRows", 100_000)],
            run: $run,
            ..ToolDef::BLANK
        };
    };
}

fn run_ndvi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, red) = (refl(ctx, "nir")?, refl(ctx, "red")?);
    let v = normalized(nir, red, ("nir", "red"))?;
    Ok(Json::obj([("ndvi", Json::Num(v))]))
}

normalized_tool!(
    NDVI,
    "raster.index.ndvi",
    "NDVI",
    "The normalized difference vegetation index from near-infrared and red surface reflectance, the standard measure of how much green vegetation a pixel holds.",
    "nir",
    "Near-infrared reflectance",
    "Surface reflectance, 0 to 1, like 0.45",
    "red",
    "Red reflectance",
    "Surface reflectance, 0 to 1, like 0.08",
    "ndvi",
    "NDVI",
    "(NIR - Red) / (NIR + Red)",
    ROUSE,
    &[
        "NDVI calculator",
        "vegetation index",
        "normalized difference vegetation index"
    ],
    &[
        "NDVI",
        "vegetation",
        "greenness",
        "reflectance",
        "NIR",
        "red",
        "remote sensing"
    ],
    r#"{"nir":0.45,"red":0.08}"#,
    "add-spatial-indexing-and-raster imagery-indices scenario: NIR 0.45 and red 0.08 give 0.6981",
    "NDVI is {ndvi}.",
    "NDVI = (NIR - Red) / (NIR + Red) on surface reflectance (Rouse and others 1974)",
    "Exact arithmetic on the reflectance given. What the number means depends on the sensor, the atmosphere correction, and the season.",
    "Use this to turn a pixel's near-infrared and red reflectance into the vegetation index nearly every other measure is compared against: checking a value read off an image, sanity-checking a processing chain, or working an example by hand. Healthy dense vegetation runs high because leaves reflect near-infrared strongly and absorb red; bare soil sits near zero and water goes negative.",
    "It reads one pixel's reflectance, not an image, and it cannot tell you why a value is what it is: NDVI saturates over dense canopy, responds to soil brightness where cover is sparse, and shifts with atmospheric correction and sun angle. Compare values only within the same sensor and processing level, and use EVI or SAVI where saturation or soil background matters.",
    &[
        Related {
            id: "raster.index.evi",
            reason: "alternative"
        },
        Related {
            id: "raster.index.savi",
            reason: "alternative"
        },
        Related {
            id: "raster.index.ndwi-mcfeeters",
            reason: "next"
        },
    ],
    run_ndvi
);

fn run_ndwi_mcfeeters(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (green, nir) = (refl(ctx, "green")?, refl(ctx, "nir")?);
    let v = normalized(green, nir, ("green", "nir"))?;
    Ok(Json::obj([("ndwi", Json::Num(v))]))
}

normalized_tool!(
    NDWI_MCFEETERS,
    "raster.index.ndwi-mcfeeters",
    "NDWI (McFeeters, open water)",
    "The normalized difference water index of McFeeters 1996, from green and near-infrared reflectance: the one that maps open water.",
    "green",
    "Green reflectance",
    "Surface reflectance, 0 to 1, like 0.12",
    "nir",
    "Near-infrared reflectance",
    "Surface reflectance, 0 to 1, like 0.04",
    "ndwi",
    "NDWI",
    "(Green - NIR) / (Green + NIR)",
    MCFEETERS,
    &["NDWI calculator", "open water index", "McFeeters NDWI"],
    &[
        "NDWI",
        "water",
        "open water",
        "lake",
        "flood",
        "green",
        "NIR"
    ],
    r#"{"green":0.12,"nir":0.04}"#,
    "McFeeters 1996: green 0.12 and NIR 0.04 give 0.5",
    "NDWI is {ndwi}.",
    "NDWI = (Green - NIR) / (Green + NIR) on surface reflectance (McFeeters 1996)",
    "Exact arithmetic on the reflectance given.",
    "Use this to pick open water out of an image: water reflects green and absorbs near-infrared, so lakes, rivers, and flooding stand out positive while land goes negative. It is the index to reach for when the question is where the water is.",
    "It confuses built-up surfaces with water, which is what MNDWI was proposed to fix, and it says nothing about water inside vegetation, which is Gao's index of the same name. A threshold of zero is a starting point rather than a rule: shadows, turbidity, and thin water over bright bottoms all move it.",
    &[
        Related {
            id: "raster.index.ndwi-gao",
            reason: "alternative"
        },
        Related {
            id: "raster.index.mndwi",
            reason: "alternative"
        },
        Related {
            id: "raster.index.ndvi",
            reason: "parent"
        },
    ],
    run_ndwi_mcfeeters
);

fn run_ndwi_gao(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, swir1) = (refl(ctx, "nir")?, refl(ctx, "swir1")?);
    let v = normalized(nir, swir1, ("nir", "swir1"))?;
    Ok(Json::obj([("ndwi", Json::Num(v))]))
}

normalized_tool!(
    NDWI_GAO,
    "raster.index.ndwi-gao",
    "NDWI (Gao, vegetation water)",
    "The normalized difference water index of Gao 1996, from near-infrared and shortwave-infrared reflectance: the one that measures liquid water in vegetation.",
    "nir",
    "Near-infrared reflectance",
    "Surface reflectance, 0 to 1, like 0.38",
    "swir1",
    "Shortwave-infrared reflectance",
    "Surface reflectance around 1.6 micrometers, 0 to 1, like 0.22",
    "ndwi",
    "NDWI",
    "(NIR - SWIR1) / (NIR + SWIR1)",
    GAO,
    &[
        "Gao NDWI",
        "vegetation water content index",
        "NDMI calculator"
    ],
    &[
        "NDWI",
        "NDMI",
        "vegetation water",
        "moisture",
        "drought",
        "SWIR",
        "NIR"
    ],
    r#"{"nir":0.38,"swir1":0.22}"#,
    "Gao 1996: NIR 0.38 and SWIR1 0.22 give 0.2667",
    "NDWI is {ndwi}.",
    "NDWI = (NIR - SWIR1) / (NIR + SWIR1) on surface reflectance (Gao 1996)",
    "Exact arithmetic on the reflectance given.",
    "Use this for how much water the vegetation itself holds: shortwave-infrared is absorbed by liquid water in leaves, so the index falls as a canopy dries. It is used for drought stress, fuel moisture, and irrigation scheduling, and is the same quantity often published as NDMI.",
    "It is not an open-water index, despite the shared name: for mapping lakes use the McFeeters or modified index. It responds to canopy structure as well as water, and the shortwave band it needs is missing from sensors that carry only visible and near-infrared bands.",
    &[
        Related {
            id: "raster.index.ndwi-mcfeeters",
            reason: "alternative"
        },
        Related {
            id: "raster.index.nbr",
            reason: "next"
        },
        Related {
            id: "raster.index.ndvi",
            reason: "parent"
        },
    ],
    run_ndwi_gao
);

fn run_mndwi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (green, swir1) = (refl(ctx, "green")?, refl(ctx, "swir1")?);
    let v = normalized(green, swir1, ("green", "swir1"))?;
    Ok(Json::obj([("mndwi", Json::Num(v))]))
}

normalized_tool!(
    MNDWI,
    "raster.index.mndwi",
    "MNDWI",
    "The modified normalized difference water index from green and shortwave-infrared reflectance, which separates water from built-up surfaces better than NDWI.",
    "green",
    "Green reflectance",
    "Surface reflectance, 0 to 1, like 0.12",
    "swir1",
    "Shortwave-infrared reflectance",
    "Surface reflectance around 1.6 micrometers, 0 to 1, like 0.03",
    "mndwi",
    "MNDWI",
    "(Green - SWIR1) / (Green + SWIR1)",
    XU,
    &["MNDWI calculator", "modified NDWI", "water index urban"],
    &["MNDWI", "water", "urban", "built-up", "green", "SWIR"],
    r#"{"green":0.12,"swir1":0.03}"#,
    "Xu 2006: green 0.12 and SWIR1 0.03 give 0.6",
    "MNDWI is {mndwi}.",
    "MNDWI = (Green - SWIR1) / (Green + SWIR1) on surface reflectance (Xu 2006)",
    "Exact arithmetic on the reflectance given.",
    "Use this where NDWI struggles: in towns and cities, built-up surfaces come out positive under the McFeeters index and get mapped as water. Swapping near-infrared for shortwave-infrared pushes them negative, so water is cleaner to threshold in an urban scene.",
    "It still needs a threshold chosen for the scene, and shadow from buildings and terrain remains a source of false water. The shortwave band is coarser than the visible bands on some sensors, so a resampled MNDWI is softer at the shoreline than its pixel size suggests.",
    &[
        Related {
            id: "raster.index.ndwi-mcfeeters",
            reason: "alternative"
        },
        Related {
            id: "raster.index.ndbi",
            reason: "next"
        },
        Related {
            id: "raster.index.ndwi-gao",
            reason: "alternative"
        },
    ],
    run_mndwi
);

fn run_ndbi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (swir1, nir) = (refl(ctx, "swir1")?, refl(ctx, "nir")?);
    let v = normalized(swir1, nir, ("swir1", "nir"))?;
    Ok(Json::obj([("ndbi", Json::Num(v))]))
}

normalized_tool!(
    NDBI,
    "raster.index.ndbi",
    "NDBI",
    "The normalized difference built-up index from shortwave-infrared and near-infrared reflectance, which highlights roofs, roads, and other built surfaces.",
    "swir1",
    "Shortwave-infrared reflectance",
    "Surface reflectance around 1.6 micrometers, 0 to 1, like 0.31",
    "nir",
    "Near-infrared reflectance",
    "Surface reflectance, 0 to 1, like 0.26",
    "ndbi",
    "NDBI",
    "(SWIR1 - NIR) / (SWIR1 + NIR)",
    ZHA,
    &["NDBI calculator", "built-up index", "urban index"],
    &["NDBI", "built-up", "urban", "impervious", "SWIR", "NIR"],
    r#"{"swir1":0.31,"nir":0.26}"#,
    "Zha and others 2003: SWIR1 0.31 and NIR 0.26 give 0.0877",
    "NDBI is {ndbi}.",
    "NDBI = (SWIR1 - NIR) / (SWIR1 + NIR) on surface reflectance (Zha, Gao, and Ni 2003)",
    "Exact arithmetic on the reflectance given.",
    "Use this to separate built surfaces from vegetation: roofs, pavement, and bare construction reflect shortwave-infrared more than near-infrared, so they come out positive while vegetation is negative. It is usually read beside NDVI rather than alone.",
    "Bare soil and dry ground behave much like built-up surfaces here, which is the index's main weakness, and the original work paired it with NDVI to tell them apart. It is not a measure of impervious fraction, and the result depends on the sensor's shortwave band.",
    &[
        Related {
            id: "raster.index.ndvi",
            reason: "alternative"
        },
        Related {
            id: "raster.index.mndwi",
            reason: "alternative"
        },
        Related {
            id: "raster.index.nbr",
            reason: "next"
        },
    ],
    run_ndbi
);

fn run_nbr(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, swir2) = (refl(ctx, "nir")?, refl(ctx, "swir2")?);
    let v = normalized(nir, swir2, ("nir", "swir2"))?;
    Ok(Json::obj([("nbr", Json::Num(v))]))
}

normalized_tool!(
    NBR,
    "raster.index.nbr",
    "NBR",
    "The normalized burn ratio from near-infrared and the longer shortwave-infrared band, the index burn severity is measured with.",
    "nir",
    "Near-infrared reflectance",
    "Surface reflectance, 0 to 1, like 0.33",
    "swir2",
    "Shortwave-infrared reflectance",
    "Surface reflectance around 2.2 micrometers, 0 to 1, like 0.08",
    "nbr",
    "NBR",
    "(NIR - SWIR2) / (NIR + SWIR2)",
    KEY_BENSON,
    &[
        "NBR calculator",
        "normalized burn ratio",
        "fire severity index"
    ],
    &["NBR", "burn", "fire", "severity", "NIR", "SWIR"],
    r#"{"nir":0.33,"swir2":0.08}"#,
    "Key and Benson 2006: NIR 0.33 and SWIR2 0.08 give 0.6098",
    "NBR is {nbr}.",
    "NBR = (NIR - SWIR2) / (NIR + SWIR2) on surface reflectance (Key and Benson 2006)",
    "Exact arithmetic on the reflectance given.",
    "Use this on a scene before or after a fire: healthy vegetation reflects near-infrared and absorbs the longer shortwave band, while recently burned ground does the opposite, so NBR falls sharply where a fire has passed. One NBR is a state; the difference between two is what measures severity.",
    "A single NBR is not a severity: it reads low over water, rock, and bare ground that never burned. Severity comes from dNBR, the pre-fire value minus the post-fire one, and even that needs a pre-fire image of the same season to compare against.",
    &[
        Related {
            id: "raster.index.dnbr",
            reason: "next"
        },
        Related {
            id: "raster.index.ndwi-gao",
            reason: "alternative"
        },
        Related {
            id: "raster.index.ndvi",
            reason: "parent"
        },
    ],
    run_nbr
);

// ---------------------------------------------------------------- EVI, EVI2, SAVI, dNBR

fn run_evi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, red, blue) = (refl(ctx, "nir")?, refl(ctx, "red")?, refl(ctx, "blue")?);
    let den = nir + 6.0 * red - 7.5 * blue + 1.0;
    if den == 0.0 {
        return Err(ToolError::invalid(
            "/nir",
            "NIR + 6 Red - 7.5 Blue + 1 is zero, so the index is undefined here; in an image this pixel is no-data.",
        ));
    }
    let v = 2.5 * (nir - red) / den;
    Ok(Json::obj([("evi", Json::Num(v))]))
}

pub static EVI: ToolDef = ToolDef {
    id: "raster.index.evi",
    title: "EVI",
    summary: "The enhanced vegetation index from blue, red, and near-infrared reflectance, which stays responsive over dense canopy where NDVI saturates.",
    aliases: &["EVI calculator", "enhanced vegetation index"],
    keywords: &[
        "EVI",
        "vegetation",
        "canopy",
        "saturation",
        "MODIS",
        "blue",
        "aerosol",
    ],
    inputs: &[
        band(
            "nir",
            "Near-infrared reflectance",
            "Surface reflectance, 0 to 1, like 0.45",
        )
        .required()
        .core(),
        band(
            "red",
            "Red reflectance",
            "Surface reflectance, 0 to 1, like 0.08",
        )
        .required()
        .core(),
        band(
            "blue",
            "Blue reflectance",
            "Surface reflectance, 0 to 1, like 0.04",
        )
        .required()
        .core(),
    ],
    outputs: &[index_out(
        "evi",
        "EVI",
        "2.5 (NIR - Red) / (NIR + 6 Red - 7.5 Blue + 1)",
    )],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["SUSPECT_SCALING", "EXPERIMENTAL_TOOL"],
    model: "EVI = 2.5 (NIR - Red) / (NIR + 6 Red - 7.5 Blue + 1), the MODIS coefficients (Huete and others 2002)",
    accuracy: "Exact arithmetic on the reflectance given.",
    when_to_use: "Use this over dense vegetation, where NDVI flattens out and stops distinguishing more canopy from a lot of canopy. The blue band lets it correct for aerosol scattering, and the coefficients are the MODIS ones, so values are comparable with published MODIS products.",
    limitations: "It needs a blue band, which is the noisiest in most sensors and is missing from some, and the aerosol correction it applies is the reason EVI2 exists. The coefficients are tied to the MODIS formulation, so an EVI computed from another sensor's bands is not strictly the same quantity.",
    references: &[HUETE_2002],
    examples: &[Example {
        id: "primary",
        title: "EVI over a vegetated pixel",
        input: r#"{"nir":0.45,"red":0.08,"blue":0.04}"#,
        source: "Huete and others 2002: NIR 0.45, red 0.08, blue 0.04 give 0.5675",
    }],
    primary_example: "primary",
    visualization: &[TABLE],
    related: &[
        Related {
            id: "raster.index.evi2",
            reason: "alternative",
        },
        Related {
            id: "raster.index.ndvi",
            reason: "parent",
        },
        Related {
            id: "raster.index.savi",
            reason: "alternative",
        },
    ],
    sentence: "EVI is {evi}.",
    limits: &[("batchRows", 100_000)],
    run: run_evi,
    ..ToolDef::BLANK
};

fn run_evi2(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, red) = (refl(ctx, "nir")?, refl(ctx, "red")?);
    let den = nir + 2.4 * red + 1.0;
    if den == 0.0 {
        return Err(ToolError::invalid(
            "/nir",
            "NIR + 2.4 Red + 1 is zero, so the index is undefined here; in an image this pixel is no-data.",
        ));
    }
    let v = 2.5 * (nir - red) / den;
    Ok(Json::obj([("evi2", Json::Num(v))]))
}

pub static EVI2: ToolDef = ToolDef {
    id: "raster.index.evi2",
    title: "EVI2",
    summary: "The two-band enhanced vegetation index from red and near-infrared reflectance, which tracks EVI without needing a blue band.",
    aliases: &["EVI2 calculator", "two-band enhanced vegetation index"],
    keywords: &[
        "EVI2",
        "vegetation",
        "canopy",
        "no blue band",
        "VIIRS",
        "Landsat",
    ],
    inputs: &[
        band(
            "nir",
            "Near-infrared reflectance",
            "Surface reflectance, 0 to 1, like 0.45",
        )
        .required()
        .core(),
        band(
            "red",
            "Red reflectance",
            "Surface reflectance, 0 to 1, like 0.08",
        )
        .required()
        .core(),
    ],
    outputs: &[index_out(
        "evi2",
        "EVI2",
        "2.5 (NIR - Red) / (NIR + 2.4 Red + 1)",
    )],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["SUSPECT_SCALING", "EXPERIMENTAL_TOOL"],
    model: "EVI2 = 2.5 (NIR - Red) / (NIR + 2.4 Red + 1) (Jiang and others 2008)",
    accuracy: "Exact arithmetic on the reflectance given. Designed to track EVI closely where atmospheric correction is good.",
    when_to_use: "Use this when you want EVI's behavior over dense canopy but the imagery has no usable blue band, or the blue band is noisy. It was fitted to follow EVI on atmospherically corrected data, so the two can be compared across a long record made of sensors with different bands.",
    limitations: "Without a blue band it cannot correct for aerosols, so it relies on the surface reflectance product already having done so; over hazy scenes it drifts from EVI. It shares NDVI's sensitivity to soil background where cover is sparse.",
    references: &[JIANG],
    examples: &[Example {
        id: "primary",
        title: "EVI2 over a vegetated pixel",
        input: r#"{"nir":0.45,"red":0.08}"#,
        source: "Jiang and others 2008 formula: NIR 0.45 and red 0.08 give 0.5633",
    }],
    primary_example: "primary",
    visualization: &[TABLE],
    related: &[
        Related {
            id: "raster.index.evi",
            reason: "alternative",
        },
        Related {
            id: "raster.index.ndvi",
            reason: "parent",
        },
        Related {
            id: "raster.index.savi",
            reason: "alternative",
        },
    ],
    sentence: "EVI2 is {evi2}.",
    limits: &[("batchRows", 100_000)],
    run: run_evi2,
    ..ToolDef::BLANK
};

fn run_savi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nir, red) = (refl(ctx, "nir")?, refl(ctx, "red")?);
    let l = ctx.number("soil_factor")?.unwrap_or(0.5);
    let den = nir + red + l;
    if den == 0.0 {
        return Err(ToolError::invalid(
            "/nir",
            "NIR + Red + L is zero, so the index is undefined here; in an image this pixel is no-data.",
        ));
    }
    let v = (1.0 + l) * (nir - red) / den;
    Ok(Json::obj([("savi", Json::Num(v))]))
}

pub static SAVI: ToolDef = ToolDef {
    id: "raster.index.savi",
    title: "SAVI",
    summary: "The soil-adjusted vegetation index from red and near-infrared reflectance, with the soil adjustment factor L, for scenes where bare ground shows through.",
    aliases: &["SAVI calculator", "soil adjusted vegetation index"],
    keywords: &[
        "SAVI",
        "soil",
        "sparse vegetation",
        "arid",
        "vegetation",
        "L factor",
    ],
    inputs: &[
        band(
            "nir",
            "Near-infrared reflectance",
            "Surface reflectance, 0 to 1, like 0.45",
        )
        .required()
        .core(),
        band(
            "red",
            "Red reflectance",
            "Surface reflectance, 0 to 1, like 0.08",
        )
        .required()
        .core(),
        Field::new(
            "soil_factor",
            "Soil adjustment factor L",
            "0 for dense cover, 0.5 for intermediate (the default), 1 for very sparse",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .core(),
    ],
    outputs: &[index_out(
        "savi",
        "SAVI",
        "(1 + L)(NIR - Red) / (NIR + Red + L)",
    )],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["SUSPECT_SCALING", "EXPERIMENTAL_TOOL"],
    model: "SAVI = (1 + L)(NIR - Red) / (NIR + Red + L), L = 0.5 unless given (Huete 1988)",
    accuracy: "Exact arithmetic on the reflectance given.",
    when_to_use: "Use this where the ground shows between plants — rangeland, early growth, arid and semi-arid scenes — because soil brightness pushes NDVI around in exactly those conditions. The L factor sets how much of that soil effect is taken out: 1 for very sparse cover, 0.5 in between, 0 for closed canopy, where SAVI becomes NDVI scaled.",
    limitations: "L has to be chosen, and the right value depends on the cover you are trying to measure, which is often what you are trying to find out; the 0.5 default is a compromise from the original paper. It does not remove the soil signal, it reduces it, and it shares NDVI's saturation over dense canopy.",
    references: &[HUETE_1988],
    examples: &[Example {
        id: "primary",
        title: "SAVI over sparse cover",
        input: r#"{"nir":0.45,"red":0.08}"#,
        source: "Huete 1988 formula with L = 0.5: NIR 0.45 and red 0.08 give 0.5388",
    }],
    primary_example: "primary",
    visualization: &[TABLE],
    related: &[
        Related {
            id: "raster.index.ndvi",
            reason: "parent",
        },
        Related {
            id: "raster.index.evi",
            reason: "alternative",
        },
        Related {
            id: "raster.index.evi2",
            reason: "alternative",
        },
    ],
    sentence: "SAVI is {savi}.",
    limits: &[("batchRows", 100_000)],
    run: run_savi,
    ..ToolDef::BLANK
};

/// The Key and Benson severity ranges, offered as a starting point. They are
/// the published classification, not a rule: severity depends on the ecosystem.
const SEVERITY: &[(f64, f64, &str)] = &[
    (f64::NEG_INFINITY, -0.25, "high post-fire regrowth"),
    (-0.25, -0.1, "low post-fire regrowth"),
    (-0.1, 0.1, "unburned"),
    (0.1, 0.27, "low severity"),
    (0.27, 0.44, "moderate-low severity"),
    (0.44, 0.66, "moderate-high severity"),
    (0.66, f64::INFINITY, "high severity"),
];

fn run_dnbr(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let pre = ctx.number("nbr_pre")?.expect("required");
    let post = ctx.number("nbr_post")?.expect("required");
    for (name, v) in [("nbr_pre", pre), ("nbr_post", post)] {
        if !(-1.0..=1.0).contains(&v) {
            return Err(ToolError::invalid(
                &format!("/{name}"),
                format!("{name} is {v}; a normalized burn ratio lies between -1 and 1."),
            ));
        }
    }
    let d = pre - post;
    let class = SEVERITY
        .iter()
        .find(|(lo, hi, _)| d >= *lo && d < *hi)
        .map_or("unburned", |(_, _, name)| *name);
    Ok(Json::obj([
        ("dnbr", Json::Num(d)),
        ("severity", Json::str(class)),
    ]))
}

pub static DNBR: ToolDef = ToolDef {
    id: "raster.index.dnbr",
    title: "dNBR (burn severity)",
    summary: "The difference between a pre-fire and a post-fire normalized burn ratio, with the published severity ranges it is usually read against.",
    aliases: &["dNBR calculator", "delta NBR", "burn severity calculator"],
    keywords: &[
        "dNBR",
        "burn severity",
        "fire",
        "NBR",
        "pre-fire",
        "post-fire",
    ],
    inputs: &[
        Field::new(
            "nbr_pre",
            "Pre-fire NBR",
            "The normalized burn ratio before the fire, like 0.61",
            Kind::Number {
                min: -1.0,
                max: 1.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "nbr_post",
            "Post-fire NBR",
            "The normalized burn ratio after the fire, like 0.13",
            Kind::Number {
                min: -1.0,
                max: 1.0,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        index_out("dnbr", "dNBR", "Pre-fire NBR minus post-fire NBR"),
        Field::new(
            "severity",
            "Severity class",
            "The Key and Benson range this falls in, as a starting point",
            Kind::Text { max_len: 40 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "dNBR = NBR(pre-fire) - NBR(post-fire); classes from the Key and Benson (2006) ranges",
    accuracy: "Exact arithmetic. The class ranges are a published starting point, not a determination of severity.",
    when_to_use: "Use this once you have an NBR from before a fire and one from after it: the drop between them is what burn-severity mapping is based on, and it separates ground that burned from ground that was already bare. The class it names is the range the value falls in.",
    limitations: "The ranges come from western US conifer forests and do not transfer unchanged to other ecosystems; severity mapping normally calibrates them against field plots, and often uses the relativized form (RdNBR) where pre-fire cover varies. Both images need the same sensor, season, and processing, or the difference measures the images rather than the fire.",
    references: &[KEY_BENSON],
    examples: &[Example {
        id: "primary",
        title: "A moderate-high severity burn",
        input: r#"{"nbr_pre":0.61,"nbr_post":0.13}"#,
        source: "Key and Benson 2006: a drop of 0.48 falls in the moderate-high range (0.44 to 0.66)",
    }],
    primary_example: "primary",
    visualization: &[TABLE],
    related: &[
        Related {
            id: "raster.index.nbr",
            reason: "parent",
        },
        Related {
            id: "raster.index.ndvi",
            reason: "alternative",
        },
        Related {
            id: "raster.index.ndwi-gao",
            reason: "alternative",
        },
    ],
    sentence: "dNBR is {dnbr}, in the {severity} range.",
    limits: &[("batchRows", 100_000)],
    run: run_dnbr,
    ..ToolDef::BLANK
};
