//! The ITRF2020 plate motion model (Altamimi et al. 2023, GRL 50,
//! e2023GL106373): rigid-plate rotation poles, the origin rate bias, and the
//! deformation zones where rigid-plate velocities do not apply (Bird 2003
//! PB2002 orogens).

use crate::helmert::ARCSEC;

/// Plate codes and rotation rates (arc-seconds per year, about X, Y, Z),
/// as in PROJ's data/ITRF2020 (the IGN ITRF2020-PMM.dat values).
pub const PLATES: &[(&str, &str, [f64; 3])] = &[
    ("AMUR", "Amurian", [-0.000131, -0.000551, 0.000837]),
    ("ANTA", "Antarctic", [-0.000269, -0.000312, 0.000678]),
    ("ARAB", "Arabian", [0.001129, -0.000146, 0.001438]),
    ("AUST", "Australian", [0.001487, 0.001175, 0.001223]),
    ("CARB", "Caribbean", [0.000207, -0.001422, 0.000726]),
    ("EURA", "Eurasian", [-0.000085, -0.000519, 0.000753]),
    ("INDI", "Indian", [0.001137, 0.000013, 0.001444]),
    ("NAZC", "Nazca", [-0.000327, -0.001561, 0.001605]),
    ("NOAM", "North American", [0.000045, -0.000666, -0.000098]),
    ("NUBI", "Nubian", [0.000090, -0.000585, 0.000717]),
    ("PCFC", "Pacific", [-0.000404, 0.001021, -0.002154]),
    ("SOAM", "South American", [-0.000261, -0.000282, -0.000157]),
    ("SOMA", "Somalian", [-0.000081, -0.000719, 0.000864]),
];

/// The origin rate bias (Table 2), meters per year, added to plate velocities.
pub const ORB: [f64; 3] = [0.00037, 0.00035, 0.00074];

/// The ITRF2020 velocity (m/yr) of ECEF point `x` riding `plate`: ω × X, plus the ORB if asked.
pub fn velocity(plate: &str, x: [f64; 3], with_orb: bool) -> Option<[f64; 3]> {
    let (_, _, w) = PLATES.iter().find(|(c, _, _)| *c == plate)?;
    let w = w.map(|r| r * ARCSEC);
    let v = [
        w[1] * x[2] - w[2] * x[1],
        w[2] * x[0] - w[0] * x[2],
        w[0] * x[1] - w[1] * x[0],
    ];
    Some(if with_orb {
        [0, 1, 2].map(|i| v[i] + ORB[i])
    } else {
        v
    })
}

const OROGENS: &str = include_str!("../data/orogens.txt");

/// The PB2002 deformation zone containing (lat, lon) in degrees, if any.
pub fn deformation_zone(lat: f64, lon: f64) -> Option<&'static str> {
    let mut name = "";
    for line in OROGENS.lines() {
        if let Some(n) = line.strip_prefix("@ ") {
            name = n;
        } else if !line.starts_with('#') && !line.is_empty() && inside(line, lat, lon) {
            return Some(name);
        }
    }
    None
}

/// Even-odd point-in-polygon on longitude and latitude.
fn inside(ring: &str, lat: f64, lon: f64) -> bool {
    let pts: Vec<(f64, f64)> = ring
        .split(' ')
        .filter_map(|p| {
            let (a, b) = p.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .collect();
    let mut hit = false;
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        if (y1 > lat) != (y2 > lat) && lon < x1 + (lat - y1) * (x2 - x1) / (y2 - y1) {
            hit = !hit;
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones() {
        // Near the San Andreas fault, in the Alps, and on the stable interior.
        assert_eq!(
            deformation_zone(35.0, -119.5),
            Some("Gorda-California-Nevada")
        );
        assert_eq!(deformation_zone(46.5, 10.0), Some("Alps"));
        assert_eq!(deformation_zone(39.0, -98.0), None);
        assert_eq!(deformation_zone(-25.0, 135.0), None);
    }
}
