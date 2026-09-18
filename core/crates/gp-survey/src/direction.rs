//! Survey directions (survey/cogo spec, "Bearings, azimuths, and angle
//! notation"): quadrant bearings like `N 45°30'15" E` or `S44-30-00W`, north
//! azimuths in decimal degrees or DMS, and formatting back to bearings.

use gp_geo::dms::{self, Axis, Style};

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
