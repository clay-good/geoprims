//! Geomagnetic main-field models (geodesy/geomagnetism spec): WMM2025 and
//! IGRF-14, evaluated by the spherical-harmonic synthesis of the WMM Technical
//! Report (Chulliat et al.): geodetic → geocentric, Schmidt semi-normalized
//! Legendre functions, the field sum, a pole-safe east component, and the
//! rotation back to the geodetic frame. The coefficient files are embedded
//! unchanged from NCEI and IAGA.

use libm::{asin, atan2, cos, hypot, sin, sqrt};

static WMM2025_COF: &str = include_str!("../data/WMM2025.COF");
static IGRF14_TXT: &str = include_str!("../data/igrf14coeffs.txt");

/// Geomagnetic reference radius (km).
const A_REF: f64 = 6371.2;
/// WGS 84 semi-major axis (km) and flattening.
const WGS84_A: f64 = 6378.137;
const WGS84_F: f64 = 1.0 / 298.257_223_563;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Wmm2025,
    Igrf14,
}

impl Model {
    pub fn id(self) -> &'static str {
        match self {
            Model::Wmm2025 => "wmm2025",
            Model::Igrf14 => "igrf14",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::Wmm2025 => "WMM2025",
            Model::Igrf14 => "IGRF-14",
        }
    }

    /// Valid decimal years, inclusive.
    pub fn window(self) -> (f64, f64) {
        match self {
            Model::Wmm2025 => (2025.0, 2030.0),
            Model::Igrf14 => (1900.0, 2030.0),
        }
    }

    pub fn version(self) -> &'static str {
        match self {
            Model::Wmm2025 => "WMM2025 (2024-11-13)",
            Model::Igrf14 => "IGRF-14 (2024-12)",
        }
    }
}

/// Coefficients at one epoch: g, h (nT) and their rates (nT/yr), indexed n(n+1)/2 + m.
#[derive(Clone, Debug)]
pub struct Coeffs {
    pub n_max: usize,
    pub g: Vec<f64>,
    pub h: Vec<f64>,
    pub gd: Vec<f64>,
    pub hd: Vec<f64>,
}

fn idx(n: usize, m: usize) -> usize {
    n * (n + 1) / 2 + m
}

impl Coeffs {
    fn zero(n_max: usize) -> Coeffs {
        let k = idx(n_max, n_max) + 1;
        Coeffs {
            n_max,
            g: vec![0.0; k],
            h: vec![0.0; k],
            gd: vec![0.0; k],
            hd: vec![0.0; k],
        }
    }
}

fn parse_wmm() -> (f64, Coeffs) {
    let mut lines = WMM2025_COF.lines();
    let epoch: f64 = lines
        .next()
        .and_then(|l| l.split_whitespace().next())
        .and_then(|t| t.parse().ok())
        .expect("WMM header epoch");
    let mut c = Coeffs::zero(12);
    for l in lines {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 6 || f[0].starts_with("9999") {
            break;
        }
        let n: usize = f[0].parse().expect("n");
        let m: usize = f[1].parse().expect("m");
        let v: Vec<f64> = f[2..6]
            .iter()
            .map(|t| t.parse().expect("coefficient"))
            .collect();
        let i = idx(n, m);
        (c.g[i], c.h[i], c.gd[i], c.hd[i]) = (v[0], v[1], v[2], v[3]);
    }
    (epoch, c)
}

/// IGRF-14 table: epochs 1900..2025 by 5 years, then the 2025-30 secular variation.
struct Igrf {
    epochs: Vec<f64>,
    /// Per epoch: (g, h) coefficient vectors to degree 13.
    g: Vec<Vec<f64>>,
    h: Vec<Vec<f64>>,
    sv_g: Vec<f64>,
    sv_h: Vec<f64>,
}

fn parse_igrf() -> Igrf {
    let mut epochs = Vec::new();
    let k = idx(13, 13) + 1;
    let mut out = Igrf {
        epochs: Vec::new(),
        g: Vec::new(),
        h: Vec::new(),
        sv_g: vec![0.0; k],
        sv_h: vec![0.0; k],
    };
    for l in IGRF14_TXT.lines() {
        if l.starts_with('#') || l.starts_with("c/s") {
            continue;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        if l.starts_with("g/h") {
            epochs = f[3..f.len() - 1]
                .iter()
                .map(|t| t.parse().expect("epoch"))
                .collect();
            out.g = vec![vec![0.0; k]; epochs.len()];
            out.h = vec![vec![0.0; k]; epochs.len()];
            continue;
        }
        if f.len() < 4 {
            continue;
        }
        let n: usize = f[1].parse().expect("n");
        let m: usize = f[2].parse().expect("m");
        let vals: Vec<f64> = f[3..].iter().map(|t| t.parse().expect("value")).collect();
        let i = idx(n, m);
        let (tab, sv) = if f[0] == "g" {
            (&mut out.g, &mut out.sv_g)
        } else {
            (&mut out.h, &mut out.sv_h)
        };
        for (e, v) in vals[..epochs.len()].iter().enumerate() {
            tab[e][i] = *v;
        }
        sv[i] = vals[epochs.len()];
    }
    out.epochs = epochs;
    out
}

/// The coefficients of `model` at decimal year `t`, with their rates of change.
pub fn coeffs_at(model: Model, t: f64) -> Coeffs {
    match model {
        Model::Wmm2025 => {
            let (epoch, mut c) = parse_wmm();
            for i in 0..c.g.len() {
                c.g[i] += c.gd[i] * (t - epoch);
                c.h[i] += c.hd[i] * (t - epoch);
            }
            c
        }
        Model::Igrf14 => {
            let igrf = parse_igrf();
            let e = &igrf.epochs;
            let last = e.len() - 1;
            let mut c = Coeffs::zero(13);
            if t >= e[last] {
                for i in 0..c.g.len() {
                    c.gd[i] = igrf.sv_g[i];
                    c.hd[i] = igrf.sv_h[i];
                    c.g[i] = igrf.g[last][i] + c.gd[i] * (t - e[last]);
                    c.h[i] = igrf.h[last][i] + c.hd[i] * (t - e[last]);
                }
            } else {
                let j = e.iter().rposition(|x| *x <= t).unwrap_or(0);
                let span = e[j + 1] - e[j];
                for i in 0..c.g.len() {
                    c.gd[i] = (igrf.g[j + 1][i] - igrf.g[j][i]) / span;
                    c.hd[i] = (igrf.h[j + 1][i] - igrf.h[j][i]) / span;
                    c.g[i] = igrf.g[j][i] + c.gd[i] * (t - e[j]);
                    c.h[i] = igrf.h[j][i] + c.hd[i] * (t - e[j]);
                }
            }
            c
        }
    }
}

/// How IGRF-14 labels the span containing `t`.
pub fn igrf_span(t: f64) -> &'static str {
    if t < 1945.0 {
        "non-definitive (IGRF, before 1945)"
    } else if t <= 2020.0 {
        "definitive (DGRF, 1945-2020)"
    } else if t <= 2025.0 {
        "provisional (2020-2025)"
    } else {
        "predictive (2025-2030)"
    }
}

/// The field at one point: north X, east Y, down Z (nT) in the geodetic frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Geodetic latitude/longitude (degrees) and height above the WGS 84 ellipsoid
/// (km) to geocentric latitude (radians) and radius (km).
fn geocentric(lat: f64, h_km: f64) -> (f64, f64) {
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let (sl, cl) = (sin(lat.to_radians()), cos(lat.to_radians()));
    let rc = WGS84_A / sqrt(1.0 - e2 * sl * sl);
    let xp = (rc + h_km) * cl;
    let zp = (rc * (1.0 - e2) + h_km) * sl;
    let r = hypot(xp, zp);
    (asin(zp / r), r)
}

/// Schmidt semi-normalized P_n^m(sin φ') and dP/dφ' to degree `n_max`.
fn legendre(n_max: usize, x: f64) -> (Vec<f64>, Vec<f64>) {
    let k = idx(n_max, n_max) + 1;
    let z = sqrt((1.0 - x) * (1.0 + x));
    let mut p = vec![0.0; k];
    let mut dp = vec![0.0; k];
    p[0] = 1.0;
    for n in 1..=n_max {
        for m in 0..=n {
            let i = idx(n, m);
            if n == m {
                let i1 = idx(n - 1, m - 1);
                p[i] = z * p[i1];
                dp[i] = z * dp[i1] + x * p[i1];
            } else if n == 1 && m == 0 {
                let i1 = idx(0, 0);
                p[i] = x * p[i1];
                dp[i] = x * dp[i1] - z * p[i1];
            } else {
                let i2 = idx(n - 1, m);
                if m + 2 > n {
                    p[i] = x * p[i2];
                    dp[i] = x * dp[i2] - z * p[i2];
                } else {
                    let i1 = idx(n - 2, m);
                    let kk = (((n - 1) * (n - 1)) as f64 - (m * m) as f64)
                        / (((2 * n - 1) * (2 * n - 3)) as f64);
                    p[i] = x * p[i2] - kk * p[i1];
                    dp[i] = x * dp[i2] - z * p[i2] - kk * dp[i1];
                }
            }
        }
    }
    // Gauss-normalized recursions above; convert to Schmidt semi-normalized.
    let mut s = vec![0.0; k];
    s[0] = 1.0;
    for n in 1..=n_max {
        let i = idx(n, 0);
        s[i] = s[idx(n - 1, 0)] * (2 * n - 1) as f64 / n as f64;
        for m in 1..=n {
            let j = idx(n, m);
            let f = if m == 1 { 2.0 } else { 1.0 };
            s[j] = s[j - 1] * sqrt(((n - m + 1) as f64 * f) / (n + m) as f64);
        }
    }
    for i in 0..k {
        p[i] *= s[i];
        dp[i] *= -s[i];
    }
    (p, dp)
}

/// Sums the field (X, Y, Z in the geocentric frame) for coefficients `g`, `h`.
fn synth(n_max: usize, g: &[f64], h: &[f64], lat_c: f64, lon: f64, r: f64) -> Xyz {
    let (p, dp) = legendre(n_max, sin(lat_c));
    let lam = lon.to_radians();
    let (mut bx, mut by, mut bz) = (0.0, 0.0, 0.0);
    for n in 1..=n_max {
        let rr = libm::pow(A_REF / r, (n + 2) as f64);
        for m in 0..=n {
            let i = idx(n, m);
            let (sm, cm) = (sin(m as f64 * lam), cos(m as f64 * lam));
            let gc = g[i] * cm + h[i] * sm;
            bz -= rr * gc * (n + 1) as f64 * p[i];
            by += rr * (g[i] * sm - h[i] * cm) * m as f64 * p[i];
            bx -= rr * gc * dp[i];
        }
    }
    let cp = cos(lat_c);
    if cp.abs() > 1e-10 {
        by /= cp;
    } else {
        // At the geocentric pole: the m = 1 terms alone, with P_n^1/cos φ' by recursion.
        by = 0.0;
        let sp = sin(lat_c);
        let mut ps = vec![0.0; n_max + 1];
        ps[0] = 1.0;
        let mut q1 = 1.0;
        let (s1, c1) = (sin(lam), cos(lam));
        for n in 1..=n_max {
            let i = idx(n, 1);
            let q2 = q1 * (2 * n - 1) as f64 / n as f64;
            let q3 = q2 * sqrt((n * 2) as f64 / (n + 1) as f64);
            q1 = q2;
            ps[n] = if n == 1 {
                ps[0]
            } else {
                let k = (((n - 1) * (n - 1)) as f64 - 1.0) / (((2 * n - 1) * (2 * n - 3)) as f64);
                sp * ps[n - 1] - k * ps[n - 2]
            };
            let rr = libm::pow(A_REF / r, (n + 2) as f64);
            by += rr * (g[i] * s1 - h[i] * c1) * ps[n] * q3;
        }
    }
    Xyz {
        x: bx,
        y: by,
        z: bz,
    }
}

/// The main field and its secular variation at geodetic `lat`, `lon` (degrees),
/// height `h_km` above the WGS 84 ellipsoid, for coefficients `c`.
pub fn field(c: &Coeffs, lat: f64, lon: f64, h_km: f64) -> (Xyz, Xyz) {
    let (lat_c, r) = geocentric(lat, h_km);
    let psi = lat_c - lat.to_radians();
    let rot = |b: Xyz| Xyz {
        x: b.x * cos(psi) - b.z * sin(psi),
        y: b.y,
        z: b.x * sin(psi) + b.z * cos(psi),
    };
    (
        rot(synth(c.n_max, &c.g, &c.h, lat_c, lon, r)),
        rot(synth(c.n_max, &c.gd, &c.hd, lat_c, lon, r)),
    )
}

/// The seven elements and their rates. Angles in degrees (per year).
#[derive(Clone, Copy, Debug)]
pub struct Elements {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub h: f64,
    pub f: f64,
    pub d: f64,
    pub i: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub dh: f64,
    pub df: f64,
    pub dd: f64,
    pub di: f64,
}

pub fn elements(b: Xyz, sv: Xyz) -> Elements {
    let h = hypot(b.x, b.y);
    let f = hypot(h, b.z);
    let dh = (b.x * sv.x + b.y * sv.y) / h;
    let df = (b.x * sv.x + b.y * sv.y + b.z * sv.z) / f;
    Elements {
        x: b.x,
        y: b.y,
        z: b.z,
        h,
        f,
        d: atan2(b.y, b.x).to_degrees(),
        i: atan2(b.z, h).to_degrees(),
        dx: sv.x,
        dy: sv.y,
        dz: sv.z,
        dh,
        df,
        dd: ((b.x * sv.y - b.y * sv.x) / (h * h)).to_degrees(),
        di: ((h * sv.z - b.z * dh) / (f * f)).to_degrees(),
    }
}

/// WMM2025 declination uncertainty (degrees), √(0.26² + (5417/H)²), capped at 180°.
pub fn wmm_declination_uncertainty(h_nt: f64) -> f64 {
    let u = sqrt(0.26 * 0.26 + (5417.0 / h_nt) * (5417.0 / h_nt));
    u.min(180.0)
}

/// Decimal year of a proleptic Gregorian date, as the WMM software computes it:
/// year + (day of year − 1) / days in year.
pub fn decimal_year(y: i32, m: u32, d: u32) -> Option<f64> {
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let dim = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m) || d == 0 || d > dim[m as usize - 1] {
        return None;
    }
    let doy: u32 = dim[..m as usize - 1].iter().sum::<u32>() + d;
    Some(y as f64 + (doy - 1) as f64 / if leap { 366.0 } else { 365.0 })
}
