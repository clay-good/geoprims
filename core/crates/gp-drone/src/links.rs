//! Radio link budget (add-practitioner-essentials, drone/sensors-and-links
//! "Radio link budget"): free-space path loss, received power, and fade margin
//! against the receiver's sensitivity, with the transmitter's EIRP checked
//! against the dated 2.4 GHz limit for the jurisdiction chosen. It never
//! suggests raising power past that limit.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::log10;

use crate::unit;

const ITU_525: Reference = Reference {
    title: "Recommendation ITU-R P.525-4: Calculation of free-space attenuation",
    issuer: "International Telecommunication Union",
    year: 2019,
    edition: "P.525-4 (08/2019)",
    locator: "Equation 6: L = 32.4 + 20 log f + 20 log d, with f in MHz and d in km",
    url: "https://www.itu.int/rec/R-REC-P.525-4-201908-I/en",
};

const fn db(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    min: f64,
    max: f64,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Number { min, max }).measure("power_level", u)
}

pub static LINK_BUDGET: ToolDef = ToolDef {
    id: "drone.links.link-budget",
    version: "1.0.1",
    title: "Radio link budget",
    summary: "Free-space path loss, received signal, and fade margin for a control or video link, with the transmitter's EIRP checked against the dated 2.4 GHz limit where you fly.",
    aliases: &[
        "link budget calculator",
        "free space path loss",
        "FSPL calculator",
        "drone range radio",
    ],
    keywords: &[
        "link budget",
        "FSPL",
        "path loss",
        "dBm",
        "EIRP",
        "fade margin",
        "2.4 GHz",
        "radio",
        "antenna",
    ],
    inputs: &[
        Field::new(
            "frequency",
            "Frequency",
            "Like 2400 MHz",
            Kind::Quantity {
                q: QT::Frequency,
                unit: "MHz",
            },
        )
        .required()
        .core(),
        Field::new(
            "distance",
            "Distance",
            "Like 5 km",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .required()
        .core(),
        db(
            "tx_power",
            "Transmit power",
            "Conducted, in dBm, like 20",
            -50.0,
            60.0,
            "dBm",
        )
        .core(),
        db(
            "rx_sensitivity",
            "Receiver sensitivity",
            "In dBm, like -90",
            -150.0,
            0.0,
            "dBm",
        )
        .core(),
        Field::new(
            "jurisdiction",
            "Rules",
            "us (47 CFR 15.247) or eu (ETSI EN 300 328), for the 2.4 GHz EIRP limit",
            Kind::Choice(&["us", "eu"]),
        )
        .core(),
        db(
            "tx_gain",
            "Transmit antenna gain",
            "In dBi, like 2 (the default 0)",
            -20.0,
            40.0,
            "dBi",
        ),
        db(
            "rx_gain",
            "Receive antenna gain",
            "In dBi, like 2 (the default 0)",
            -20.0,
            40.0,
            "dBi",
        ),
        db(
            "cable_loss",
            "Cable and connector losses",
            "Both ends together, in dB, like 1 (the default 0)",
            0.0,
            60.0,
            "dB",
        ),
    ],
    outputs: &[
        db(
            "fspl",
            "Free-space path loss",
            "20 log d(km) + 20 log f(MHz) + 32.44",
            0.0,
            400.0,
            "dB",
        )
        .precision(Precision::Decimals(1)),
        db(
            "rx_power",
            "Received power",
            "Transmit + gains − losses − path loss",
            -400.0,
            100.0,
            "dBm",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        db(
            "fade_margin",
            "Fade margin",
            "Received power − sensitivity",
            -400.0,
            400.0,
            "dB",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        db(
            "eirp",
            "EIRP",
            "Transmit power + antenna gain − transmit-side loss",
            -100.0,
            100.0,
            "dBm",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "eirp_status",
            "EIRP limit",
            "Within, near, or beyond the limit, with its rule",
            Kind::Text { max_len: 200 },
        )
        .status("threshold", "cfr-47-15")
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "FSPL = 20 log₁₀ d_km + 20 log₁₀ f_MHz + 32.44 dB (ITU-R P.525); received power = transmit power + both antenna gains − cable losses − FSPL; fade margin = received power − sensitivity; EIRP = transmit power + transmit gain − half the cable loss, taken as the transmit side. 2.4 GHz limits from dated reference data",
    accuracy: "Free space only: terrain, the ground reflection, bodies, and vegetation add loss, so check the Fresnel zone and line of sight too. Planning aid; your equipment's certification governs",
    references: &[ITU_525],
    examples: &[Example {
        id: "primary",
        title: "2,400 MHz over 5 km, 20 dBm and 2 dBi each end, EU rules",
        input: r#"{"frequency":"2400 MHz","distance":"5 km","tx_power":20,"rx_sensitivity":-90,"tx_gain":2,"rx_gain":2,"jurisdiction":"eu"}"#,
        source: "add-practitioner-essentials link scenario: FSPL about 114.0 dB at 2.4 GHz and 5 km",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.los.fresnel",
            reason: "next",
        },
        Related {
            id: "drone.sensors.vlos",
            reason: "alternative",
        },
    ],
    sentence: "The path loses {fspl} dB.{if fade_margin > -1000} The link keeps {fade_margin} dB of fade margin.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_link,
    ..ToolDef::BLANK
};

fn run_link(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let f = ctx
        .req_quantity("frequency")?
        .to(unit(QT::Frequency, "MHz"));
    let d = ctx.req_quantity("distance")?.to(unit(QT::Distance, "km"));
    if !(f > 0.0 && d > 0.0) || f > 1e6 || d > 1e5 {
        return Err(ToolError::invalid(
            "/frequency",
            "Give a frequency above 0 (up to 1 THz) and a distance above 0 (up to 100,000 km).",
        ));
    }
    let fspl = 20.0 * log10(d) + 20.0 * log10(f) + 32.44;
    let (gt, gr) = (
        ctx.number("tx_gain")?.unwrap_or(0.0),
        ctx.number("rx_gain")?.unwrap_or(0.0),
    );
    let loss = ctx.number("cable_loss")?.unwrap_or(0.0);
    let mut out = vec![("fspl", Json::Num(fspl))];
    let fmt = ctx.options.format;
    let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
    if ctx.explaining() {
        ctx.step(
            "Distance term",
            "20 log₁₀ d_km",
            format!("20 log₁₀ {}", n(d, 3)),
            format!("{} dB", n(20.0 * log10(d), 2)),
        );
        ctx.step(
            "Free-space path loss",
            "20 log₁₀ d_km + 20 log₁₀ f_MHz + 32.44",
            format!(
                "{} + {} + 32.44",
                n(20.0 * log10(d), 2),
                n(20.0 * log10(f), 2)
            ),
            format!("{} dB", n(fspl, 1)),
        );
    }
    if let Some(pt) = ctx.number("tx_power")? {
        let rx = pt + gt + gr - loss - fspl;
        out.push(("rx_power", Json::Num(rx)));
        if let Some(s) = ctx.number("rx_sensitivity")? {
            out.push(("fade_margin", Json::Num(rx - s)));
        }
        let eirp = pt + gt - loss / 2.0;
        out.push(("eirp", Json::Num(eirp)));
        if let Some(j) = ctx.choice("jurisdiction")? {
            let status = if (2400.0..=2483.5).contains(&f) {
                let r = rule(if j == "us" {
                    "fcc-15-247-2g4"
                } else {
                    "etsi-en-300-328-2g4"
                });
                let label = format!(
                    "{} dBm EIRP limit ({}, rules as of {})",
                    n(r.value, 0),
                    r.citation,
                    r.reviewed
                );
                // A limit in dBm: compare in milliwatts so "near" means within the usual margin of power.
                let mw = |dbm: f64| libm::pow(10.0, dbm / 10.0);
                gp_base::status::threshold(
                    mw(eirp),
                    mw(r.value),
                    gp_base::status::NEAR_MARGIN,
                    &label,
                )
            } else {
                "No EIRP limit on file for this band; check the rules for your frequency".to_owned()
            };
            out.push(("eirp_status", Json::str(status)));
        }
    } else if ctx.number("rx_sensitivity")?.is_some() {
        return Err(ToolError::invalid(
            "/tx_power",
            "Give the transmit power to work out the received power and fade margin.",
        ));
    }
    Ok(Json::obj(out))
}
