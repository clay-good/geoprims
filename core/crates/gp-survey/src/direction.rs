//! Survey directions (survey/cogo spec, "Bearings, azimuths, and angle
//! notation"): quadrant bearings like `N 45°30'15" E` or `S44-30-00W`, north
//! azimuths in decimal degrees, DMS, or gons (`123.4567 gon`, 400 to the
//! circle), and formatting back to bearings.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::dms::{self, Axis, Style};

use crate::{GHILANI, deg, unit};

pub static PARSE: ToolDef = ToolDef {
    id: "survey.cogo.direction-parse",
    title: "Survey bearing and azimuth converter",
    summary: "Reads a quadrant bearing, north azimuth, or gon value and writes the same direction as an azimuth, bearing, and gons.",
    aliases: &[
        "bearing to azimuth",
        "azimuth to bearing",
        "quadrant bearing converter",
        "bearing converter",
    ],
    keywords: &["COGO", "direction", "DMS", "gons", "grads", "north azimuth"],
    inputs: &[
        Field::new(
            "direction",
            "Direction",
            "Like S 44°30'00\" W, 224°30'00\", or 249.444444 gon",
            Kind::Text { max_len: 48 },
        )
        .required()
        .core(),
        Field::new(
            "reference",
            "Direction reference",
            "grid, geodetic, or assumed (default)",
            Kind::Choice(&["grid", "geodetic", "assumed"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "azimuth",
            "North azimuth",
            "Degrees clockwise from north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(8))
        .angle_range("[0,360)"),
        Field::new(
            "azimuth_dms",
            "North azimuth (DMS)",
            "Degrees, minutes, and seconds clockwise from north",
            Kind::Text { max_len: 32 },
        ),
        Field::new(
            "bearing",
            "Quadrant bearing",
            "North or south, then the acute angle toward east or west",
            Kind::Text { max_len: 32 },
        ),
        Field::new(
            "gons",
            "Gons",
            "The north azimuth with 400 gons to a circle",
            Kind::Quantity {
                q: QT::Angle,
                unit: "gon",
            },
        )
        .precision(Precision::Decimals(8))
        .angle_range("[0,400)"),
        Field::new(
            "reference",
            "Direction reference",
            "The reference supplied with the direction",
            Kind::Text { max_len: 16 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    when_to_use: "Use this before entering a record bearing in software that expects a north azimuth, or when field notes and a data collector use different direction notation. It accepts quadrant bearings, decimal or DMS north azimuths, and gons, then shows all three forms without changing the stated direction reference. It is also useful for checking a hand conversion before a traverse, forward calculation, stakeout, or plan annotation. State the same reference that the source document or job grid uses.",
    limitations: "This changes notation only. It does not rotate between grid, geodetic, assumed, or magnetic north, and it does not determine which reference a record intended. Supply the reference from the survey record, coordinate system, or instrument setup. A direction labeled with the wrong reference can still be formatted correctly but point the wrong way. Use convergence, declination, or basis-rotation tools before this one when the north reference itself must change.",
    model: "Quadrant bearings map their acute angle into the matching north-azimuth quadrant; 400 gon = 360°",
    accuracy: "Exact apart from displaying DMS to 0.01 arcsecond and decimal values to eight places",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Southwest quadrant bearing",
        input: r#"{"direction":"S 44°30'00\" W","reference":"grid"}"#,
        source: "add-survey-suite quadrant-bearing scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.forward",
            reason: "next",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
    ],
    sentence: "The {reference} direction is azimuth {azimuth}, or {bearing}.",
    run: run_parse,
    ..ToolDef::BLANK
};

fn run_parse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text = ctx
        .text("direction")?
        .ok_or_else(|| ToolError::invalid("/direction", "Direction is required."))?;
    let az = parse(&text).map_err(|message| ToolError::invalid("/direction", message))?;
    let reference = ctx.choice("reference")?.unwrap_or("assumed");
    Ok(Json::obj([
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("azimuth_dms", Json::str(azimuth(az, 2))),
        ("bearing", Json::str(bearing(az, 2))),
        ("gons", ctx.emit("gons", deg(az), unit(QT::Angle, "gon"))),
        ("reference", Json::str(reference)),
    ]))
}

/// Parses a direction to a north azimuth in degrees [0, 360).
pub fn parse(s: &str) -> Result<f64, String> {
    let t = s.trim().to_ascii_uppercase();
    let first = t.chars().next().ok_or("the direction is empty")?;
    let last = t.chars().last().ok_or("the direction is empty")?;
    if matches!(first, 'N' | 'S') && matches!(last, 'E' | 'W') {
        let body = t[1..t.len() - 1].trim().replace('-', " ");
        let angle =
            dms::parse_plain(&body).map_err(|_| format!("\"{s}\" has an unreadable angle"))?;
        if !(0.0..=90.0).contains(&angle) {
            return Err(format!(
                "\"{s}\": a quadrant bearing angle must be between 0° and 90°"
            ));
        }
        let az = match (first, last) {
            ('N', 'E') => angle,
            ('S', 'E') => 180.0 - angle,
            ('S', 'W') => 180.0 + angle,
            _ => 360.0 - angle,
        };
        return Ok(gp_base::angle::wrap_azimuth(az));
    }
    // Gons (grads): 400 to the circle, so one gon is 0.9°.
    if let Some(num) = ["GON", "GRAD", "G"].iter().find_map(|u| t.strip_suffix(u)) {
        let g: f64 = num
            .trim()
            .parse()
            .map_err(|_| format!("\"{s}\" has an unreadable number of gons"))?;
        if !(0.0..=400.0).contains(&g) {
            return Err(format!(
                "\"{s}\": an azimuth in gons must be between 0 and 400"
            ));
        }
        return Ok(gp_base::angle::wrap_azimuth(g * 0.9));
    }
    let angle = dms::parse_plain(&t.replace('-', " "))
        .map_err(|_| format!("\"{s}\" is not a bearing or azimuth"))?;
    if !(0.0..=360.0).contains(&angle) {
        return Err(format!("\"{s}\": an azimuth must be between 0° and 360°"));
    }
    Ok(gp_base::angle::wrap_azimuth(angle))
}

/// Formats an azimuth as a quadrant bearing in DMS with `decimals` second decimals.
pub fn bearing(az: f64, decimals: u32) -> String {
    let az = gp_base::angle::wrap_azimuth(az);
    let (ns, ew, angle) = if az <= 90.0 {
        ('N', 'E', az)
    } else if az <= 180.0 {
        ('S', 'E', 180.0 - az)
    } else if az <= 270.0 {
        ('S', 'W', az - 180.0)
    } else {
        ('N', 'W', 360.0 - az)
    };
    format!(
        "{ns} {} {ew}",
        dms::format(angle, Axis::Lon, Style::Dms, decimals, false)
    )
}

/// Formats an azimuth in DMS.
pub fn azimuth(az: f64, decimals: u32) -> String {
    dms::format(
        gp_base::angle::wrap_azimuth(az),
        Axis::Lon,
        Style::Dms,
        decimals,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gons() {
        assert_eq!(parse("100 gon").unwrap(), 90.0);
        assert_eq!(parse("200g").unwrap(), 180.0);
        assert!((parse("123.4567 grad").unwrap() - 111.11103).abs() < 1e-9);
        assert!(parse("401 gon").unwrap_err().contains("between 0 and 400"));
    }

    #[test]
    fn quadrant_bearings() {
        assert_eq!(parse("S 44°30'00\" W").unwrap(), 224.5);
        assert_eq!(parse("N45-30-00E").unwrap(), 45.5);
        assert_eq!(parse("N 10 W").unwrap(), 350.0);
        assert_eq!(parse("S 0 E").unwrap(), 180.0);
        assert!(
            parse("N 95°00'00\" E")
                .unwrap_err()
                .contains("between 0° and 90°")
        );
        assert_eq!(parse("224°30'").unwrap(), 224.5);
        assert_eq!(parse("90").unwrap(), 90.0);
        assert!(parse("400").is_err());
    }

    #[test]
    fn format_bearings() {
        assert_eq!(bearing(45.0, 0), "N 45°00'00\" E");
        assert_eq!(bearing(224.5, 0), "S 44°30'00\" W");
        assert_eq!(bearing(350.0, 0), "N 10°00'00\" W");
        assert_eq!(bearing(180.0, 0), "S 0°00'00\" E");
    }
}
