//! UTM and UPS (geodesy/projections spec): zone rules with the Norway and
//! Svalbard exceptions, forced zones, polar stereographic for UPS, and the
//! convergence and scale at each point. Follows GeographicLib's UTMUPS and
//! PolarStereographic.

use libm::{atan, atan2, exp, hypot, sin, sqrt, tan};

use crate::tm::{Tm, tauf, taupf};

pub const UTM_K0: f64 = 0.9996;
pub const UPS_K0: f64 = 0.994;
pub const UTM_FE: f64 = 500_000.0;
pub const UTM_FN_SOUTH: f64 = 10_000_000.0;
pub const UPS_FE: f64 = 2_000_000.0;

/// A projected position: zone (0 = UPS), hemisphere, easting, northing,
/// convergence (deg), and point scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub zone: u8,
    pub north: bool,
    pub easting: f64,
    pub northing: f64,
    pub convergence: f64,
    pub scale: f64,
}

/// The standard UTM zone for a point, with the Norway and Svalbard exceptions.
/// Points on a zone boundary go to the eastern zone. Longitude in [-180, 180).
pub fn standard_zone(lat: f64, lon: f64) -> u8 {
    let mut zone = (((lon + 180.0) / 6.0).floor() as i32).rem_euclid(60) + 1;
    if (56.0..64.0).contains(&lat) && (3.0..12.0).contains(&lon) {
        zone = 32;
    }
    if (72.0..=84.0).contains(&lat) && (0.0..42.0).contains(&lon) {
        zone = if lon < 9.0 {
            31
        } else if lon < 21.0 {
            33
        } else if lon < 33.0 {
            35
        } else {
            37
        };
    }
    zone as u8
}

/// True when latitude is inside the UTM domain (80° S to 84° N).
pub fn in_utm_domain(lat: f64) -> bool {
    (-80.0..=84.0).contains(&lat)
}

pub fn central_meridian(zone: u8) -> f64 {
    6.0 * f64::from(zone) - 183.0
}

/// UTM forward in a given zone (standard or forced).
pub fn utm_forward(a: f64, f: f64, lat: f64, lon: f64, zone: u8) -> Grid {
    let tm = Tm::new(a, f, UTM_K0);
    let dlon = crate_wrap(lon - central_meridian(zone));
    let (x, y, gamma, k) = tm.forward(lat, dlon);
    let north = lat >= 0.0;
    Grid {
        zone,
        north,
        easting: x + UTM_FE,
        northing: if north { y } else { y + UTM_FN_SOUTH },
        convergence: gamma,
        scale: k,
    }
}

/// UTM inverse: (lat, lon) in degrees.
pub fn utm_inverse(
    a: f64,
    f: f64,
    zone: u8,
    north: bool,
    easting: f64,
    northing: f64,
) -> (f64, f64) {
    let tm = Tm::new(a, f, UTM_K0);
    let y = if north {
        northing
    } else {
        northing - UTM_FN_SOUTH
    };
    let (lat, dlon) = tm.inverse(easting - UTM_FE, y);
    (lat, crate_wrap(dlon + central_meridian(zone)))
}

fn crate_wrap(lon: f64) -> f64 {
    gp_base::angle::wrap_lon(lon)
}

/// Polar stereographic constant c = √(1 − e²) · exp(e · atanh(e)).
fn ps_c(es: f64) -> f64 {
    sqrt(1.0 - es * es) * exp(es * libm::atanh(es))
}

/// UPS forward (north when `north`), with convergence and scale.
pub fn ups_forward(a: f64, f: f64, lat: f64, lon: f64, north: bool) -> Grid {
    let es = sqrt(f * (2.0 - f));
    let e2m = 1.0 - es * es;
    let c = ps_c(es);
    let lat_n = if north { lat } else { -lat };
    let tau = tan(lat_n.to_radians());
    let secant = hypot(1.0, tau);
    let taup = taupf(tau, es);
    let mut rho = hypot(1.0, taup) + taup.abs();
    rho = if taup >= 0.0 {
        if lat_n != 90.0 { 1.0 / rho } else { 0.0 }
    } else {
        rho
    };
    rho *= 2.0 * UPS_K0 * a / c;
    let k = if lat_n != 90.0 {
        rho / a * secant / sqrt(e2m + es * es / (secant * secant))
    } else {
        UPS_K0
    };
    let lam = lon.to_radians();
    let (x, y) = (
        rho * sin(lam),
        if north {
            -rho * libm::cos(lam)
        } else {
            rho * libm::cos(lam)
        },
    );
    Grid {
        zone: 0,
        north,
        easting: x + UPS_FE,
        northing: y + UPS_FE,
        convergence: if north { lon } else { -lon },
        scale: k,
    }
}

/// UPS inverse: (lat, lon) in degrees.
pub fn ups_inverse(a: f64, f: f64, north: bool, easting: f64, northing: f64) -> (f64, f64) {
    let es = sqrt(f * (2.0 - f));
    let c = ps_c(es);
    let x = easting - UPS_FE;
    let y = northing - UPS_FE;
    let y = if north { -y } else { y };
    let rho = hypot(x, y);
    let t = if rho != 0.0 {
        rho / (2.0 * UPS_K0 * a / c)
    } else {
        f64::EPSILON * f64::EPSILON
    };
    let taup = (1.0 / t - t) / 2.0;
    let tau = tauf(taup, es);
    let lat = atan(tau).to_degrees();
    let lon = atan2(x, y).to_degrees();
    (if north { lat } else { -lat }, lon)
}

/// UTM where the point is in the UTM domain, otherwise UPS.
pub fn forward_auto(a: f64, f: f64, lat: f64, lon: f64) -> Grid {
    if in_utm_domain(lat) {
        utm_forward(a, f, lat, lon, standard_zone(lat, lon))
    } else {
        ups_forward(a, f, lat, lon, lat > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;

    #[test]
    fn zone_rules() {
        assert_eq!(standard_zone(40.446_111, -79.982_222), 17);
        assert_eq!(standard_zone(60.0, 5.0), 32);
        assert_eq!(standard_zone(60.0, 2.9), 31);
        assert_eq!(standard_zone(78.0, 10.0), 33);
        assert_eq!(standard_zone(78.0, 8.9), 31);
        assert_eq!(standard_zone(0.0, -180.0), 1);
        assert_eq!(standard_zone(0.0, 179.999), 60);
        assert_eq!(standard_zone(0.0, -78.0), 18, "boundary goes east");
    }

    #[test]
    fn ups_north_scenario() {
        let g = ups_forward(A, F, 85.0, 0.0, true);
        assert!((g.easting - 2_000_000.0).abs() < 1e-9);
        assert!((g.northing - 1_444_542.609).abs() < 1e-3, "{}", g.northing);
        let (lat, lon) = ups_inverse(A, F, true, g.easting, g.northing);
        assert!((lat - 85.0).abs() < 1e-11 && lon.abs() < 1e-11);
    }

    #[test]
    fn ups_round_trips() {
        for (lat, lon, north) in [
            (84.0, 45.0, true),
            (89.9, -120.0, true),
            (-80.5, 0.0, false),
            (-88.0, 170.0, false),
        ] {
            let g = ups_forward(A, F, lat, lon, north);
            let (la, lo) = ups_inverse(A, F, north, g.easting, g.northing);
            assert!(
                (la - lat).abs() < 1e-10 && (lo - lon).abs() < 1e-9,
                "{lat} {lon} -> {la} {lo}"
            );
        }
        let pole = ups_forward(A, F, 90.0, 0.0, true);
        assert_eq!(
            (pole.easting, pole.northing, pole.scale),
            (2_000_000.0, 2_000_000.0, 0.994)
        );
    }

    #[test]
    fn utm_round_trip_south() {
        let g = utm_forward(A, F, -33.9, 151.2, standard_zone(-33.9, 151.2));
        assert_eq!(g.zone, 56);
        assert!(!g.north);
        let (la, lo) = utm_inverse(A, F, g.zone, g.north, g.easting, g.northing);
        assert!((la + 33.9).abs() < 1e-11 && (lo - 151.2).abs() < 1e-11);
    }
}
