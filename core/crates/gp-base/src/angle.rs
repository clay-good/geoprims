//! Angle normalization (numeric-determinism spec). Every reduction uses an exact
//! IEEE remainder, never repeated subtraction, so results are the same bits on
//! every host.

use crate::error::{ToolError, Warning};
use crate::num::format_f64;

/// Reduces a longitude to `[-180, 180)` exactly. `remainder` is exact in IEEE 754.
pub fn wrap_lon(lon: f64) -> f64 {
    let y = libm::remainder(lon, 360.0);
    // remainder returns [-180, 180]; 180 maps to -180. Adding 0.0 clears -0.
    if y == 180.0 { -180.0 } else { y + 0.0 }
}

/// Reduces an azimuth or heading to `[0, 360)`.
pub fn wrap_azimuth(az: f64) -> f64 {
    let mut y = az % 360.0; // fmod is exact
    if y < 0.0 {
        y += 360.0; // may round up to exactly 360 for tiny negative inputs
    }
    if y >= 360.0 { 0.0 } else { y + 0.0 }
}

/// Validates a latitude in degrees. Latitudes are never wrapped.
pub fn check_lat(lat: f64, field: &str) -> Result<f64, ToolError> {
    if (-90.0..=90.0).contains(&lat) {
        Ok(lat + 0.0)
    } else {
        Err(ToolError::invalid(
            field,
            format!(
                "Latitude must be between -90 and 90 degrees; got {}.",
                show(lat)
            ),
        ))
    }
}

/// Accepts a longitude, normalizing it to `[-180, 180)` with an `INPUT_NORMALIZED`
/// warning when it changes.
pub fn accept_lon(lon: f64, field: &str) -> Result<(f64, Option<Warning>), ToolError> {
    if !lon.is_finite() {
        return Err(ToolError::invalid(
            field,
            "Longitude must be a finite number.",
        ));
    }
    let y = wrap_lon(lon);
    let warning = (y != lon).then(|| {
        Warning::new(
            "INPUT_NORMALIZED",
            format!("Longitude {} was normalized to {}.", show(lon), show(y)),
        )
        .at(field)
    });
    Ok((y, warning))
}

fn show(x: f64) -> String {
    format_f64(x).unwrap_or_else(|| "a non-finite value".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longitude_180_normalized() {
        let (y, w) = accept_lon(180.0, "/lon").unwrap();
        assert_eq!(y, -180.0);
        let w = w.unwrap();
        assert_eq!(
            (w.code, w.field.as_deref()),
            ("INPUT_NORMALIZED", Some("/lon"))
        );
    }

    #[test]
    fn large_longitude_normalized_exactly() {
        assert_eq!(wrap_lon(540.25), -179.75);
        assert_eq!(wrap_lon(-180.0), -180.0);
        assert_eq!(wrap_lon(-540.0), -180.0);
        assert_eq!(wrap_lon(359.5), -0.5);
        assert!(wrap_lon(-360.0).is_sign_positive());
    }

    #[test]
    fn in_range_longitude_has_no_warning() {
        assert_eq!(accept_lon(-73.5, "/lon").unwrap(), (-73.5, None));
    }

    #[test]
    fn latitude_out_of_range_rejected() {
        let e = check_lat(90.0000001, "/lat").unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            ("INVALID_INPUT", Some("/lat"))
        );
        assert!(check_lat(f64::NAN, "/lat").is_err());
        assert_eq!(check_lat(-90.0, "/lat").unwrap(), -90.0);
    }

    #[test]
    fn azimuth_range() {
        assert_eq!(wrap_azimuth(360.0), 0.0);
        assert!(wrap_azimuth(-0.0).is_sign_positive());
        assert_eq!(wrap_azimuth(-1e-20), 0.0);
        assert_eq!(wrap_azimuth(-90.0), 270.0);
        assert_eq!(wrap_azimuth(725.5), 5.5);
    }

    /// 1,000,000 random and edge-biased cases: range, idempotence, and exactness
    /// against integer arithmetic for inputs that are multiples of 1/1024.
    #[test]
    fn property_million_cases() {
        let mut s: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for i in 0..1_000_000u32 {
            let r = next();
            let x = match i % 4 {
                0 => f64::from_bits(r) % 1e9, // wide random magnitudes (may be NaN-free after %)
                1 => (r % 4_000_000) as f64 / 1024.0 - 2000.0, // exact binary fractions
                2 => ((r % 21) as f64 - 10.0) * 180.0, // multiples of 180
                _ => {
                    ((r % 2001) as f64 - 1000.0) * 360.0
                        + 180.0 * ((r >> 40) % 2) as f64
                        + f64::EPSILON * ((r >> 50) % 3) as f64
                }
            };
            if !x.is_finite() {
                continue;
            }
            let lon = wrap_lon(x);
            assert!((-180.0..180.0).contains(&lon), "{x} → {lon}");
            assert_eq!(wrap_lon(lon), lon);
            let az = wrap_azimuth(x);
            assert!((0.0..360.0).contains(&az), "{x} → {az}");
            assert_eq!(wrap_azimuth(az), az);
            if i % 4 == 1 {
                let q = (x * 1024.0) as i64;
                let want = ((q + 180 * 1024).rem_euclid(360 * 1024) - 180 * 1024) as f64 / 1024.0;
                assert_eq!(lon, want, "{x}");
            }
        }
    }
}
