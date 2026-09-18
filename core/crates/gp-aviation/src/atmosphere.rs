//! The standard atmosphere engine (aviation/atmosphere spec, design AV1): one
//! layer-table evaluator on geopotential altitude for ICAO Doc 7488/3 (to 80 km)
//! and US Standard Atmosphere 1976 (to 84.852 km geopotential, 86 km geometric).
//! Layer base pressures are integrated from P0, not copied from rounded tables.
//! Every inverse (pressure → altitude, density → altitude) is closed-form per layer.

use libm::{exp, log, pow, sqrt};

pub const P0: f64 = 101_325.0;
pub const T0: f64 = 288.15;
pub const G0: f64 = 9.806_65;
pub const R: f64 = 287.052_87;
pub const GAMMA: f64 = 1.4;
/// Earth radius for geopotential conversion (ICAO Doc 7488).
pub const R0: f64 = 6_356_766.0;
pub const RHO0: f64 = P0 / (R * T0);
/// Lower limit of both models, geopotential meters.
pub const H_MIN: f64 = -5_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Icao,
    Us76,
}

impl Model {
    /// Upper limit, geopotential meters.
    pub fn h_max(self) -> f64 {
        match self {
            Model::Icao => 80_000.0,
            // 86 km geometric, the top of the 1976 lower atmosphere.
            Model::Us76 => geometric_to_geopotential(86_000.0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::Icao => "ICAO Standard Atmosphere (Doc 7488/3)",
            Model::Us76 => "US Standard Atmosphere 1976",
        }
    }
}

/// (base geopotential altitude m, lapse rate K/m, layer name). Base
/// temperatures are the defined values in `BASE_T`.
const LAYERS: [(f64, f64, &str); 7] = [
    (0.0, -0.0065, "troposphere"),
    (11_000.0, 0.0, "tropopause (isothermal)"),
    (20_000.0, 0.001, "lower stratosphere"),
    (32_000.0, 0.0028, "upper stratosphere"),
    (47_000.0, 0.0, "stratopause (isothermal)"),
    (51_000.0, -0.0028, "lower mesosphere"),
    (71_000.0, -0.002, "upper mesosphere"),
];

/// Layer base temperatures (K) as defined by ICAO Doc 7488 and US 1976.
const BASE_T: [f64; 7] = [288.15, 216.65, 216.65, 228.65, 270.65, 270.65, 214.65];

/// Base temperature and pressure of each layer; pressures integrated from P0.
fn bases() -> [(f64, f64); 7] {
    let mut out = [(T0, P0); 7];
    for i in 1..LAYERS.len() {
        let (hb, l, _) = LAYERS[i - 1];
        let dh = LAYERS[i].0 - hb;
        out[i] = (
            BASE_T[i],
            layer_pressure(out[i - 1].1, BASE_T[i - 1], l, dh),
        );
    }
    out
}

fn layer_pressure(pb: f64, tb: f64, l: f64, dh: f64) -> f64 {
    if l == 0.0 {
        pb * exp(-G0 * dh / (R * tb))
    } else {
        pb * pow((tb + l * dh) / tb, -G0 / (R * l))
    }
}

fn layer_of(h: f64) -> usize {
    LAYERS.iter().rposition(|(hb, _, _)| h >= *hb).unwrap_or(0)
}

/// The atmosphere at a geopotential altitude.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub h: f64,
    pub t: f64,
    pub p: f64,
    pub rho: f64,
    pub layer: &'static str,
}

/// ISA temperature, pressure, and density at geopotential altitude `h` (m).
/// The caller checks the model's range.
pub fn at(h: f64) -> State {
    let i = layer_of(h);
    let (hb, l, name) = LAYERS[i];
    let (tb, pb) = bases()[i];
    let t = tb + l * (h - hb);
    let p = layer_pressure(pb, tb, l, h - hb);
    State {
        h,
        t,
        p,
        rho: p / (R * t),
        layer: name,
    }
}

/// Geopotential altitude (m) where ISA pressure equals `p` (Pa).
pub fn altitude_for_pressure(p: f64) -> f64 {
    let b = bases();
    let i = b.iter().rposition(|(_, pb)| p <= *pb).unwrap_or(0);
    let (hb, l, _) = LAYERS[i];
    let (tb, pb) = b[i];
    if l == 0.0 {
        hb + R * tb / G0 * log(pb / p)
    } else {
        let t = tb * pow(p / pb, -R * l / G0);
        hb + (t - tb) / l
    }
}

/// Geopotential altitude (m) where ISA density equals `rho` (kg/m³).
pub fn altitude_for_density(rho: f64) -> f64 {
    let b = bases();
    let rho_b = |i: usize| b[i].1 / (R * b[i].0);
    let i = (0..LAYERS.len())
        .rposition(|i| rho <= rho_b(i))
        .unwrap_or(0);
    let (hb, l, _) = LAYERS[i];
    let tb = b[i].0;
    if l == 0.0 {
        hb - R * tb / G0 * log(rho / rho_b(i))
    } else {
        let t = tb * pow(rho / rho_b(i), 1.0 / (-G0 / (R * l) - 1.0));
        hb + (t - tb) / l
    }
}

pub fn geometric_to_geopotential(z: f64) -> f64 {
    R0 * z / (R0 + z)
}

pub fn geopotential_to_geometric(h: f64) -> f64 {
    R0 * h / (R0 - h)
}

pub fn speed_of_sound(t: f64) -> f64 {
    sqrt(GAMMA * R * t)
}

/// Dynamic viscosity by Sutherland's law (ICAO Doc 7488 constants).
pub fn dynamic_viscosity(t: f64) -> f64 {
    1.458e-6 * pow(t, 1.5) / (t + 110.4)
}

/// Gravity at geometric altitude `z` (inverse-square from g0 at r0).
pub fn gravity(z: f64) -> f64 {
    let k = R0 / (R0 + z);
    G0 * k * k
}

/// US 1976 molecular-weight ratio M/M0 from 80 to 86 km geometric (Table 8),
/// every 0.5 km. Kinetic temperature T = TM · (M/M0); pressure and density use
/// the molecular-scale temperature TM and are unaffected.
const US76_M_RATIO: [f64; 13] = [
    1.000_000, 0.999_996, 0.999_989, 0.999_971, 0.999_941, 0.999_909, 0.999_870, 0.999_829,
    0.999_786, 0.999_741, 0.999_694, 0.999_641, 0.999_579,
];

/// M/M0 at geometric altitude `z` (m), linearly interpolated; 1 below 80 km.
pub fn us76_molecular_weight_ratio(z: f64) -> f64 {
    if z <= 80_000.0 {
        return 1.0;
    }
    let x = ((z - 80_000.0) / 500.0).min(12.0);
    let i = (x as usize).min(11);
    let f = x - i as f64;
    US76_M_RATIO[i] + f * (US76_M_RATIO[i + 1] - US76_M_RATIO[i])
}

/// Saturation vapor pressure over water (Pa) at `t_c` °C: the Magnus form with
/// Alduchov and Eskridge (1996) coefficients, within 0.4% from -40 to 50 °C.
pub fn vapor_pressure(t_c: f64) -> f64 {
    610.94 * exp(17.625 * t_c / (t_c + 243.04))
}

/// Virtual temperature (K) of moist air with vapor pressure `e` at pressure `p`.
pub fn virtual_temperature(t: f64, e: f64, p: f64) -> f64 {
    t / (1.0 - 0.378 * e / p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_thousand_feet() {
        let s = at(3048.0);
        assert!((s.t - 268.338).abs() < 1e-3);
        assert!((s.p - 69_681.6).abs() < 0.1, "{}", s.p);
        assert!((s.rho / 0.904_637 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn tropopause_and_bases() {
        let s = at(11_000.0);
        assert_eq!(s.t, 216.65);
        assert!((s.p - 22_632.06).abs() < 0.05, "{}", s.p);
        assert_eq!(s.layer, "tropopause (isothermal)");
        let b = bases();
        for (i, want) in [101_325.0, 22_632.06, 5_474.89, 868.02, 110.91, 66.94, 3.956]
            .iter()
            .enumerate()
        {
            assert!(
                (b[i].1 / want - 1.0).abs() < 2e-4,
                "layer {i}: {} vs {want}",
                b[i].1
            );
        }
    }

    #[test]
    fn inverses_round_trip_every_layer() {
        let mut h = -5_000.0;
        while h <= 84_852.0 {
            let s = at(h);
            assert!(
                (altitude_for_pressure(s.p) - h).abs() < 1e-6,
                "pressure at {h}"
            );
            assert!(
                (altitude_for_density(s.rho) - h).abs() < 1e-6,
                "density at {h}"
            );
            h += 250.0;
        }
    }

    #[test]
    fn geometric_geopotential() {
        assert!((geometric_to_geopotential(11_000.0) - 10_980.998).abs() < 1e-3);
        assert!(
            (geopotential_to_geometric(geometric_to_geopotential(86_000.0)) - 86_000.0).abs()
                < 1e-6
        );
    }
}
