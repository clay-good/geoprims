//! Sensor presets and reflectance scaling (raster/imagery-indices spec).
//!
//! An index is a ratio of reflectances, and a product's bands are stored as
//! integers. Getting from one to the other is the step that is quietly wrong
//! most often: Sentinel-2 shifted its dynamic range at processing baseline
//! 04.00, so the same digital number means different reflectance before and
//! after January 2022, and Landsat Collection 2 uses a gain and an offset of
//! its own. This turns a digital number into reflectance with the product's
//! own numbers, and says which ones it used.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef,
};

pub const S2_PRODUCTS: Reference = Reference {
    title: "Sentinel-2 products (SentiWiki)",
    issuer: "European Space Agency and European Commission, Copernicus",
    year: 2025,
    edition: "SentiWiki, read 2026-09-22",
    locator: "Level-2A surface reflectance: L2A_SR = (L2A_DN + BOA_ADD_OFFSET) / QUANTIFICATION_VALUE, band-dependent offset introduced at processing baseline 04.00 on 25 January 2022; DN 0 is no-data",
    url: "https://sentiwiki.copernicus.eu/web/s2-products",
};
pub const LANDSAT_SCALING: Reference = Reference {
    title: "How do I use a scale factor with Landsat Level-2 science products?",
    issuer: "United States Geological Survey",
    year: 2025,
    edition: "USGS FAQ, read 2026-09-22",
    locator: "Collection 2 surface reflectance: DN x 0.0000275 - 0.2, valid range 7,273 to 43,636, fill value 0",
    url: "https://www.usgs.gov/faqs/how-do-i-use-a-scale-factor-landsat-level-2-science-products",
};

/// Reflectance outside this range is not reflectance.
const PLAUSIBLE: (f64, f64) = (-0.2, 1.5);

/// Which bands a preset names, and what each one measures.
struct Preset {
    bands: &'static [(&'static str, &'static str)],
}

const SENTINEL2: Preset = Preset {
    bands: &[
        ("B2", "blue"),
        ("B3", "green"),
        ("B4", "red"),
        ("B8", "near-infrared"),
        ("B8A", "narrow near-infrared"),
        ("B11", "shortwave-infrared 1"),
        ("B12", "shortwave-infrared 2"),
    ],
};
const LANDSAT: Preset = Preset {
    bands: &[
        ("B2", "blue"),
        ("B3", "green"),
        ("B4", "red"),
        ("B5", "near-infrared"),
        ("B6", "shortwave-infrared 1"),
        ("B7", "shortwave-infrared 2"),
        ("SR_B2", "blue"),
        ("SR_B3", "green"),
        ("SR_B4", "red"),
        ("SR_B5", "near-infrared"),
        ("SR_B6", "shortwave-infrared 1"),
        ("SR_B7", "shortwave-infrared 2"),
    ],
};

fn run_scale(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dn = ctx.number("dn")?.expect("required");
    let sensor = ctx.choice("sensor")?.unwrap_or("sentinel-2-l2a");
    let preset = if sensor == "landsat-c2-l2" {
        &LANDSAT
    } else {
        &SENTINEL2
    };
    let band = ctx.text("band")?;
    // The band name is the preset's own; an unknown one is a mistake worth
    // naming, since it usually means the wrong preset.
    let role = match band.as_deref() {
        None => None,
        Some(b) => {
            let up = b.trim().to_uppercase();
            match preset.bands.iter().find(|(n, _)| *n == up) {
                Some((_, role)) => Some(*role),
                None => {
                    return Err(ToolError::invalid(
                        "/band",
                        format!(
                            "{sensor} has no band {b}. Its bands are {}.",
                            preset
                                .bands
                                .iter()
                                .map(|(n, _)| *n)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ));
                }
            }
        }
    };
    let (reflectance, how) = if sensor == "landsat-c2-l2" {
        if dn == 0.0 {
            return Err(ToolError::invalid(
                "/dn",
                "0 is the fill value in Landsat Collection 2, so this pixel is no-data rather than dark.",
            ));
        }
        if !(7273.0..=43636.0).contains(&dn) {
            ctx.warnings.push(
                Warning::new(
                    "SUSPECT_SCALING",
                    format!(
                        "{dn} is outside the valid range of 7,273 to 43,636 for Landsat Collection 2 surface reflectance, so it may be a different product or already scaled."
                    ),
                )
                .at("/dn"),
            );
        }
        (
            dn * 0.000_027_5 - 0.2,
            "Landsat Collection 2 Level-2: DN x 0.0000275 - 0.2".to_owned(),
        )
    } else {
        if dn == 0.0 {
            return Err(ToolError::invalid(
                "/dn",
                "0 is the no-data value in Sentinel-2 Level-2A, so this pixel has no reflectance rather than none reflected.",
            ));
        }
        // The offset and the quantification value live in the product's
        // metadata; these are the values it carries unless it says otherwise.
        let offset = ctx.number("offset")?.unwrap_or_else(|| {
            if ctx.choice("baseline").ok().flatten() == Some("before-04.00") {
                0.0
            } else {
                -1000.0
            }
        });
        let quantification = ctx.number("quantification")?.unwrap_or(10_000.0);
        if quantification == 0.0 {
            return Err(ToolError::invalid(
                "/quantification",
                "The quantification value divides the digital number, so it cannot be zero.",
            ));
        }
        (
            (dn + offset) / quantification,
            format!(
                "Sentinel-2 Level-2A: (DN {} {}) / {}",
                if offset < 0.0 { "-" } else { "+" },
                (offset as i64).abs(),
                quantification as i64
            ),
        )
    };
    if reflectance < PLAUSIBLE.0 || reflectance > PLAUSIBLE.1 {
        ctx.warnings.push(
            Warning::new(
                "SUSPECT_SCALING",
                format!(
                    "The scaled value {reflectance} is outside the reflectance range of {} to {}, so the preset, the baseline, or the product level may not match this band.",
                    PLAUSIBLE.0, PLAUSIBLE.1
                ),
            )
            .at("/dn"),
        );
    }
    let mut out = vec![
        ("reflectance", Json::Num(reflectance)),
        ("scaling", Json::str(how)),
    ];
    if let Some(role) = role {
        out.push(("band_role", Json::str(role)));
    }
    Ok(Json::obj(out))
}

pub static SCALE: ToolDef = ToolDef {
    id: "raster.scale.reflectance",
    stability: gp_base::tool::Stability::Stable,
    title: "Digital number to reflectance",
    summary: "Turns a band's stored digital number into surface reflectance with its sensor's own scaling: the Sentinel-2 Level-2A offset and quantification value, or the Landsat Collection 2 gain and offset.",
    aliases: &[
        "DN to reflectance",
        "Sentinel-2 scaling",
        "Landsat scale factor",
        "surface reflectance converter",
    ],
    keywords: &[
        "reflectance",
        "digital number",
        "DN",
        "scaling",
        "Sentinel-2",
        "Landsat",
        "BOA_ADD_OFFSET",
        "baseline 04.00",
    ],
    inputs: &[
        Field::new(
            "dn",
            "Digital number",
            "The stored integer for the pixel, like 1450",
            Kind::Number {
                min: -100_000.0,
                max: 100_000.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "sensor",
            "Sensor preset",
            "sentinel-2-l2a (the default) or landsat-c2-l2",
            Kind::Choice(&["sentinel-2-l2a", "landsat-c2-l2"]),
        )
        .core(),
        Field::new(
            "band",
            "Band",
            "The product's own band name, like B8 or SR_B5",
            Kind::Text { max_len: 8 },
        )
        .core(),
        Field::new(
            "baseline",
            "Sentinel-2 processing baseline",
            "04.00-or-later (the default, from 25 January 2022) or before-04.00",
            Kind::Choice(&["04.00-or-later", "before-04.00"]),
        )
        .core(),
        Field::new(
            "offset",
            "BOA_ADD_OFFSET",
            "From the product metadata, if it differs from -1000",
            Kind::Number {
                min: -100_000.0,
                max: 100_000.0,
            },
        ),
        Field::new(
            "quantification",
            "QUANTIFICATION_VALUE",
            "From the product metadata, if it differs from 10000",
            Kind::Number {
                min: 1.0,
                max: 1_000_000.0,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "reflectance",
            "Reflectance",
            "Surface reflectance, normally 0 to 1",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "scaling",
            "Scaling applied",
            "The arithmetic used, with the numbers in it",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "band_role",
            "Band",
            "What that band measures, when a band name was given",
            Kind::Text { max_len: 32 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["SUSPECT_SCALING"],
    model: "Sentinel-2 L2A: (DN + BOA_ADD_OFFSET) / QUANTIFICATION_VALUE, offset -1000 and quantification 10000 unless given; Landsat Collection 2 Level-2: DN x 0.0000275 - 0.2",
    accuracy: "Exact arithmetic on the product's own constants. The offset is band-dependent and both values are in the product metadata, which governs.",
    when_to_use: "Use this before any index, when the numbers you have came out of a GeoTIFF rather than out of a reflectance product: bands are stored as integers, and each sensor undoes that differently. It is also the tool to reach for when an NDVI looks wrong by a constant amount, which is usually the Sentinel-2 baseline offset applied or not applied.",
    limitations: "The Sentinel-2 offset is band-dependent and both it and the quantification value come from the product metadata, which governs over the usual values assumed here; pass them when the product says otherwise. The Landsat constants are for Collection 2 Level-2 surface reflectance, not for surface temperature or Collection 1. A digital number of zero is no-data in both products and is refused rather than scaled.",
    references: &[S2_PRODUCTS, LANDSAT_SCALING],
    assumptions: &[
        Assumption {
            name: "Sentinel-2 L2A offset from processing baseline 04.00, unless given",
            value: "-1000",
            unit: "1",
            source: "sentinel2-products",
        },
        Assumption {
            name: "Sentinel-2 L2A quantification value, unless given",
            value: "10000",
            unit: "1",
            source: "sentinel2-products",
        },
        Assumption {
            name: "Landsat Collection 2 surface reflectance scale factor",
            value: "0.0000275",
            unit: "1",
            source: "landsat-scale-factor",
        },
        Assumption {
            name: "Landsat Collection 2 surface reflectance offset",
            value: "-0.2",
            unit: "1",
            source: "landsat-scale-factor",
        },
    ],
    examples: &[
        Example {
            id: "primary",
            title: "Sentinel-2 L2A digital number 1,450 at baseline 04.00",
            input: r#"{"dn":1450,"sensor":"sentinel-2-l2a","band":"B8"}"#,
            source: "add-spatial-indexing-and-raster imagery-indices scenario: (1450 - 1000) / 10000 = 0.045",
        },
        Example {
            id: "landsat",
            title: "Landsat Collection 2 Level-2 digital number 18,639",
            input: r#"{"dn":18639,"sensor":"landsat-c2-l2","band":"SR_B5"}"#,
            source: "USGS worked example: 18,639 x 0.0000275 - 0.2 = 0.313",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.index.ndvi",
            reason: "next",
        },
        Related {
            id: "raster.index.evi",
            reason: "next",
        },
        Related {
            id: "raster.index.nbr",
            reason: "next",
        },
    ],
    sentence: "Reflectance is {reflectance}, by {scaling}.",
    limits: &[("batchRows", 100_000)],
    run: run_scale,
    ..ToolDef::BLANK
};
