//! Vincenty (1975) inverse and direct, kept as legacy comparison methods
//! (navigation/geodesic "Vincenty methods are available and honest"):
//! convergence tolerance 1e-12 rad, at most 200 iterations.

use libm::{atan, atan2, cos, sin, sqrt, tan};

pub const TOLERANCE: f64 = 1e-12;
pub const MAX_ITERATIONS: usize = 200;

/// Distance (m) and forward azimuths (deg) or None if the iteration does not converge.
pub fn inverse(
    a: f64,
    f: f64,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
) -> Option<(f64, f64, f64)> {
    let b = a * (1.0 - f);
    let l = (lon2 - lon1).to_radians();
    let u1 = atan((1.0 - f) * tan(lat1.to_radians()));
    let u2 = atan((1.0 - f) * tan(lat2.to_radians()));
    let (su1, cu1, su2, cu2) = (sin(u1), cos(u1), sin(u2), cos(u2));
    let mut lambda = l;
    for _ in 0..MAX_ITERATIONS {
        let (sl, cl) = (sin(lambda), cos(lambda));
        let t1 = cu2 * sl;
        let t2 = cu1 * su2 - su1 * cu2 * cl;
        let sin_sigma = sqrt(t1 * t1 + t2 * t2);
        if sin_sigma == 0.0 {
            return Some((0.0, 0.0, 0.0));
        }
        let cos_sigma = su1 * su2 + cu1 * cu2 * cl;
        let sigma = atan2(sin_sigma, cos_sigma);
        let sin_alpha = cu1 * cu2 * sl / sin_sigma;
        let cos2_alpha = 1.0 - sin_alpha * sin_alpha;
        let cos_2sm = if cos2_alpha == 0.0 {
            0.0
        } else {
            cos_sigma - 2.0 * su1 * su2 / cos2_alpha
        };
        let c = f / 16.0 * cos2_alpha * (4.0 + f * (4.0 - 3.0 * cos2_alpha));
        let prev = lambda;
        lambda = l
            + (1.0 - c)
                * f
                * sin_alpha
                * (sigma
                    + c * sin_sigma * (cos_2sm + c * cos_sigma * (-1.0 + 2.0 * cos_2sm * cos_2sm)));
        if (lambda - prev).abs() < TOLERANCE {
            let u_sq = cos2_alpha * (a * a - b * b) / (b * b);
            let big_a =
                1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
            let big_b = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
            let d_sigma = big_b
                * sin_sigma
                * (cos_2sm
                    + big_b / 4.0
                        * (cos_sigma * (-1.0 + 2.0 * cos_2sm * cos_2sm)
                            - big_b / 6.0
                                * cos_2sm
                                * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                                * (-3.0 + 4.0 * cos_2sm * cos_2sm)));
            let s = b * big_a * (sigma - d_sigma);
            let (sl, cl) = (sin(lambda), cos(lambda));
            let az1 = atan2(cu2 * sl, cu1 * su2 - su1 * cu2 * cl).to_degrees();
            let az2 = atan2(cu1 * sl, -su1 * cu2 + cu1 * su2 * cl).to_degrees();
            return Some((s, az1, az2));
        }
    }
    None
}

/// Destination (deg) and final azimuth (deg), or None if the iteration does not converge.
pub fn direct(a: f64, f: f64, lat1: f64, lon1: f64, az1: f64, s: f64) -> Option<(f64, f64, f64)> {
    let b = a * (1.0 - f);
    let alpha1 = az1.to_radians();
    let (sa1, ca1) = (sin(alpha1), cos(alpha1));
    let tan_u1 = (1.0 - f) * tan(lat1.to_radians());
    let cu1 = 1.0 / sqrt(1.0 + tan_u1 * tan_u1);
    let su1 = tan_u1 * cu1;
    let sigma1 = atan2(tan_u1, ca1);
    let sin_alpha = cu1 * sa1;
    let cos2_alpha = 1.0 - sin_alpha * sin_alpha;
    let u_sq = cos2_alpha * (a * a - b * b) / (b * b);
    let big_a = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let big_b = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
    let mut sigma = s / (b * big_a);
    for _ in 0..MAX_ITERATIONS {
        let cos_2sm = cos(2.0 * sigma1 + sigma);
        let (ss, cs) = (sin(sigma), cos(sigma));
        let d_sigma = big_b
            * ss
            * (cos_2sm
                + big_b / 4.0
                    * (cs * (-1.0 + 2.0 * cos_2sm * cos_2sm)
                        - big_b / 6.0
                            * cos_2sm
                            * (-3.0 + 4.0 * ss * ss)
                            * (-3.0 + 4.0 * cos_2sm * cos_2sm)));
        let prev = sigma;
        sigma = s / (b * big_a) + d_sigma;
        if (sigma - prev).abs() < TOLERANCE {
            let (ss, cs) = (sin(sigma), cos(sigma));
            let cos_2sm = cos(2.0 * sigma1 + sigma);
            let x = su1 * ss - cu1 * cs * ca1;
            let lat2 = atan2(
                su1 * cs + cu1 * ss * ca1,
                (1.0 - f) * sqrt(sin_alpha * sin_alpha + x * x),
            );
            let lambda = atan2(ss * sa1, cu1 * cs - su1 * ss * ca1);
            let c = f / 16.0 * cos2_alpha * (4.0 + f * (4.0 - 3.0 * cos2_alpha));
            let l = lambda
                - (1.0 - c)
                    * f
                    * sin_alpha
                    * (sigma + c * ss * (cos_2sm + c * cs * (-1.0 + 2.0 * cos_2sm * cos_2sm)));
            let az2 = atan2(sin_alpha, -x).to_degrees();
            return Some((lat2.to_degrees(), lon1 + l.to_degrees(), az2));
        }
    }
    None
}
