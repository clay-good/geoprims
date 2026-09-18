//! NREL Solar Position Algorithm (Reda and Andreas 2008, NREL/TP-560-34302):
//! topocentric zenith and azimuth to ±0.0003° from −2000 to 6000, following
//! the published steps (and pvlib's port, which the tables come from).

use libm::{asin, atan, atan2, cos, sin, tan};

use crate::spa_tables::*;

const RAD: f64 = core::f64::consts::PI / 180.0;

/// ΔT = TT − UT (s) by the Espenak and Meeus (2006) polynomials used by NASA
/// eclipse predictions. For 2005–2050 it runs a few seconds above observed
/// values; 1 s of ΔT moves the sun about 0.000_000_5°, far inside SPA's bound.
pub fn delta_t(year: f64, month: f64) -> f64 {
    let y = year + (month - 0.5) / 12.0;
    let p = |t: f64, c: &[f64]| c.iter().rev().fold(0.0, |acc, k| acc * t + k);
    if year < -500.0 || year >= 2150.0 {
        -20.0 + 32.0 * ((y - 1820.0) / 100.0).powi(2)
    } else if year < 500.0 {
        p(
            y / 100.0,
            &[
                10583.6,
                -1014.41,
                33.78311,
                -5.952053,
                -0.1798452,
                0.022174192,
                0.0090316521,
            ],
        )
    } else if year < 1600.0 {
        p(
            (y - 1000.0) / 100.0,
            &[
                1574.2,
                -556.01,
                71.23472,
                0.319781,
                -0.8503463,
                -0.005050998,
                0.0083572073,
            ],
        )
    } else if year < 1700.0 {
        p(y - 1600.0, &[120.0, -0.9808, -0.01532, 1.0 / 7129.0])
    } else if year < 1800.0 {
        p(
            y - 1700.0,
            &[8.83, 0.1603, -0.0059285, 0.00013336, -1.0 / 1_174_000.0],
        )
    } else if year < 1860.0 {
        p(
            y - 1800.0,
            &[
                13.72,
                -0.332447,
                0.0068612,
                0.0041116,
                -0.00037436,
                0.0000121272,
                -0.0000001699,
                0.000000000875,
            ],
        )
    } else if year < 1900.0 {
        p(
            y - 1860.0,
            &[
                7.62,
                0.5737,
                -0.251754,
                0.01680668,
                -0.0004473624,
                1.0 / 233_174.0,
            ],
        )
    } else if year < 1920.0 {
        p(
            y - 1900.0,
            &[-2.79, 1.494119, -0.0598939, 0.0061966, -0.000197],
        )
    } else if year < 1941.0 {
        p(y - 1920.0, &[21.20, 0.84493, -0.076100, 0.0020936])
    } else if year < 1961.0 {
        p(y - 1950.0, &[29.07, 0.407, -1.0 / 233.0, 1.0 / 2547.0])
    } else if year < 1986.0 {
        p(y - 1975.0, &[45.45, 1.067, -1.0 / 260.0, -1.0 / 718.0])
    } else if year < 2005.0 {
        p(
            y - 2000.0,
            &[
                63.86,
                0.3345,
                -0.060374,
                0.0017275,
                0.000651814,
                0.00002373599,
            ],
        )
    } else if year < 2050.0 {
        p(y - 2000.0, &[62.92, 0.32217, 0.005589])
    } else {
        -20.0 + 32.0 * ((y - 1820.0) / 100.0).powi(2) - 0.5628 * (2150.0 - y)
    }
}

fn series(t: &[[f64; 3]], x: f64) -> f64 {
    t.iter().map(|r| r[0] * cos(r[1] + r[2] * x)).sum()
}

fn poly(terms: &[f64], x: f64) -> f64 {
    terms.iter().rev().fold(0.0, |acc, v| acc * x + v)
}

/// Observer and conditions for one SPA evaluation.
#[derive(Clone, Copy, Debug)]
pub struct Observer {
    pub lat: f64,
    pub lon: f64,
    /// Meters above the ellipsoid (SPA uses it for parallax only).
    pub elevation: f64,
    /// Annual average local pressure (mbar) and temperature (°C) for refraction.
    pub pressure: f64,
    pub temperature: f64,
    /// Apparent refraction at sunrise and sunset (deg), 0.5667 by default.
    pub atmos_refract: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Spa {
    pub zenith: f64,
    pub azimuth: f64,
    /// Elevation without refraction.
    pub elevation_true: f64,
    pub declination: f64,
    pub right_ascension: f64,
    pub hour_angle: f64,
    /// Equation of time (minutes).
    pub eot: f64,
}

/// Solar position for Julian day `jd` on the UT1 scale and ΔT seconds.
pub fn position(jd: f64, delta_t: f64, o: Observer) -> Spa {
    let jde = jd + delta_t / 86_400.0;
    let jc = (jd - 2_451_545.0) / 36_525.0;
    let jce = (jde - 2_451_545.0) / 36_525.0;
    let jme = jce / 10.0;
    let l = poly(
        &[
            series(L0, jme),
            series(L1, jme),
            series(L2, jme),
            series(L3, jme),
            series(L4, jme),
            series(L5, jme),
        ],
        jme,
    ) / 1e8;
    let l = (l / RAD).rem_euclid(360.0);
    let b = poly(&[series(B0, jme), series(B1, jme)], jme) / 1e8 / RAD;
    let r = poly(
        &[
            series(R0, jme),
            series(R1, jme),
            series(R2, jme),
            series(R3, jme),
            series(R4, jme),
        ],
        jme,
    ) / 1e8;
    let theta = (l + 180.0).rem_euclid(360.0);
    let beta = -b;
    let x = [
        poly(
            &[297.850_36, 445_267.111_480, -0.001_914_2, 1.0 / 189_474.0],
            jce,
        ),
        poly(
            &[357.527_72, 35_999.050_340, -0.000_160_3, -1.0 / 300_000.0],
            jce,
        ),
        poly(
            &[134.962_98, 477_198.867_398, 0.008_697_2, 1.0 / 56_250.0],
            jce,
        ),
        poly(
            &[93.271_91, 483_202.017_538, -0.003_682_5, 1.0 / 327_270.0],
            jce,
        ),
        poly(
            &[125.044_52, -1_934.136_261, 0.002_070_8, 1.0 / 450_000.0],
            jce,
        ),
    ];
    let (mut dpsi, mut deps) = (0.0, 0.0);
    for (abcd, y) in NUTATION_ABCD.iter().zip(NUTATION_Y) {
        let arg = (0..5).map(|i| y[i] * x[i]).sum::<f64>() * RAD;
        dpsi += (abcd[0] + abcd[1] * jce) * sin(arg);
        deps += (abcd[2] + abcd[3] * jce) * cos(arg);
    }
    let (dpsi, deps) = (dpsi / 36_000_000.0, deps / 36_000_000.0);
    let u = jme / 10.0;
    let eps0 = poly(
        &[
            84_381.448, -4_680.93, -1.55, 1_999.25, -51.38, -249.67, -39.05, 7.12, 27.87, 5.79,
            2.45,
        ],
        u,
    );
    let eps = eps0 / 3600.0 + deps;
    let dtau = -20.4898 / (3600.0 * r);
    let lambda = theta + dpsi + dtau;
    let v0 = (280.460_618_37 + 360.985_647_366_29 * (jd - 2_451_545.0) + 0.000_387_933 * jc * jc
        - jc.powi(3) / 38_710_000.0)
        .rem_euclid(360.0);
    let v = v0 + dpsi * cos(eps * RAD);
    let (lr, er, br) = (lambda * RAD, eps * RAD, beta * RAD);
    let alpha = (atan2(sin(lr) * cos(er) - tan(br) * sin(er), cos(lr)) / RAD).rem_euclid(360.0);
    let delta = asin(sin(br) * cos(er) + cos(br) * sin(er) * sin(lr)) / RAD;
    let h = (v + o.lon - alpha).rem_euclid(360.0);
    let xi = 8.794 / (3600.0 * r);
    let phi = o.lat * RAD;
    let uu = atan(0.996_647_19 * tan(phi));
    let xt = cos(uu) + o.elevation / 6_378_140.0 * cos(phi);
    let yt = 0.996_647_19 * sin(uu) + o.elevation / 6_378_140.0 * sin(phi);
    let (xir, hr, dr) = (xi * RAD, h * RAD, delta * RAD);
    let dalpha = atan2(-xt * sin(xir) * sin(hr), cos(dr) - xt * sin(xir) * cos(hr)) / RAD;
    let delta_p = atan2(
        (sin(dr) - yt * sin(xir)) * cos(dalpha * RAD),
        cos(dr) - xt * sin(xir) * cos(hr),
    ) / RAD;
    let hp = h - dalpha;
    let (dpr, hpr) = (delta_p * RAD, hp * RAD);
    let e0 = asin(sin(phi) * sin(dpr) + cos(phi) * cos(dpr) * cos(hpr)) / RAD;
    let de = if e0 >= -(0.266_67 + o.atmos_refract) {
        (o.pressure / 1010.0) * (283.0 / (273.0 + o.temperature)) * 1.02
            / (60.0 * tan((e0 + 10.3 / (e0 + 5.11)) * RAD))
    } else {
        0.0
    };
    let e = e0 + de;
    let gamma =
        (atan2(sin(hpr), cos(hpr) * sin(phi) - tan(dpr) * cos(phi)) / RAD).rem_euclid(360.0);
    let m = poly(
        &[
            280.466_456_7,
            360_007.698_277_9,
            0.030_320_28,
            1.0 / 49_931.0,
            -1.0 / 15_300.0,
            -1.0 / 2_000_000.0,
        ],
        jme,
    );
    let mut eot = (m - 0.005_718_3 - alpha + dpsi * cos(eps * RAD)).rem_euclid(360.0) * 4.0;
    if eot > 20.0 {
        eot -= 1440.0;
    }
    Spa {
        zenith: 90.0 - e,
        azimuth: (gamma + 180.0).rem_euclid(360.0),
        elevation_true: e0,
        declination: delta_p,
        right_ascension: alpha + dalpha,
        hour_angle: hp,
        eot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked example in Reda and Andreas (2008), Table A5.1:
    /// 2003-10-17 12:30:30 LST (UTC−7), ΔT 67 s.
    #[test]
    fn nrel_test_point() {
        let jd = 2_452_929.5 + 70_230.0 / 86_400.0; // 2003-10-17 19:30:30 UT
        let o = Observer {
            lat: 39.742_476,
            lon: -105.1786,
            elevation: 1830.14,
            pressure: 820.0,
            temperature: 11.0,
            atmos_refract: 0.5667,
        };
        let s = position(jd, 67.0, o);
        assert!((s.zenith - 50.111_62).abs() < 1e-5, "{}", s.zenith);
        assert!((s.azimuth - 194.340_24).abs() < 1e-5, "{}", s.azimuth);
        assert!((s.eot - 14.641_503).abs() < 1e-5, "{}", s.eot);
    }

    #[test]
    fn delta_t_modern() {
        assert!((delta_t(2000.0, 1.0) - 63.8).abs() < 0.2);
        assert!((60.0..80.0).contains(&delta_t(2026.0, 9.0)));
    }
}
