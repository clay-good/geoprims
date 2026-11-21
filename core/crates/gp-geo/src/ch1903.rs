//! CH1903+ to ETRS89 and back by swisstopo's own three-parameter
//! transformation (add-international-grids, geodesy/national-grids, "Canada,
//! Europe, and the other national paths").
//!
//! swisstopo publishes the step as a geocentric translation and nothing more
//! (Formulas and constants, section 1.4): the Swiss frame CHTRS95 is tied to
//! ETRS89, and CH1903+ sits a fixed 674.374 m, 15.056 m and 405.346 m from it
//! along the ECEF axes. It warns that the same parameters applied to CH1903,
//! rather than CH1903+, can be wrong by up to 1.6 m, because of the local
//! distortions of that network -- which is what CHENyx06 removes.

use crate::ellipsoid::Ellipsoid;
use crate::frames;

/// The translation from ETRS89 to CH1903+, metres along X, Y and Z.
pub const TO_CH1903_PLUS: [f64; 3] = [-674.374, -15.056, -405.346];

/// Bessel 1841, which CH1903+ is defined on, with swisstopo's own values.
pub fn bessel() -> Ellipsoid {
    Ellipsoid {
        id: "bessel1841",
        name: "Bessel 1841",
        a: 6_377_397.155,
        f: 1.0 / 299.152_812_85,
    }
}

/// GRS 80, which ETRS89 is defined on.
pub fn grs80() -> Ellipsoid {
    crate::beta2007::grs80()
}

/// The geocentric coordinates of a point on its own ellipsoid, metres.
pub fn ecef(ell: &Ellipsoid, lat: f64, lon: f64, h: f64) -> (f64, f64, f64) {
    frames::to_ecef(ell, lat.to_radians(), lon.to_radians(), h)
}

/// One step of the transformation: the point it reached, in degrees and
/// metres, and the geocentric coordinates on each side of the translation,
/// which is what swisstopo's own worked example prints.
pub struct Step {
    pub point: (f64, f64, f64),
    pub from: (f64, f64, f64),
    pub to: (f64, f64, f64),
}

/// ETRS89 latitude, longitude (degrees) and ellipsoidal height to CH1903+, or
/// back.
pub fn transform(lat: f64, lon: f64, h: f64, to_ch1903_plus: bool) -> Option<Step> {
    let (from, to) = if to_ch1903_plus {
        (grs80(), bessel())
    } else {
        (bessel(), grs80())
    };
    let a = ecef(&from, lat, lon, h);
    let sign = if to_ch1903_plus { 1.0 } else { -1.0 };
    let b = (
        a.0 + sign * TO_CH1903_PLUS[0],
        a.1 + sign * TO_CH1903_PLUS[1],
        a.2 + sign * TO_CH1903_PLUS[2],
    );
    let (phi, lam, hh) = frames::from_ecef(&to, b.0, b.1, b.2)?;
    Some(Step {
        point: (phi.to_degrees(), lam.to_degrees(), hh),
        from: a,
        to: b,
    })
}
