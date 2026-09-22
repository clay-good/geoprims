//! Reference ellipsoids (geodesy datums spec) and the shared ellipsoid inputs
//! every geodesic tool accepts: a catalog id, or a custom semi-major axis and
//! inverse flattening.

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::tool::{Ctx, Field, Kind};
use gp_base::units::{self, Quantity};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoid {
    pub id: &'static str,
    pub name: &'static str,
    /// Semi-major axis, meters.
    pub a: f64,
    /// Flattening.
    pub f: f64,
}

/// The catalog. Parameters are the defining values (EPSG dataset).
pub const CATALOG: &[Ellipsoid] = &[
    Ellipsoid {
        id: "wgs84",
        name: "WGS 84",
        a: 6_378_137.0,
        f: 1.0 / 298.257_223_563,
    },
    Ellipsoid {
        id: "grs80",
        name: "GRS 1980",
        a: 6_378_137.0,
        f: 1.0 / 298.257_222_101,
    },
    Ellipsoid {
        id: "clarke1866",
        name: "Clarke 1866",
        a: 6_378_206.4,
        f: (6_378_206.4 - 6_356_583.8) / 6_378_206.4,
    },
    Ellipsoid {
        id: "intl1924",
        name: "International 1924",
        a: 6_378_388.0,
        f: 1.0 / 297.0,
    },
    Ellipsoid {
        id: "airy1830",
        name: "Airy 1830",
        a: 6_377_563.396,
        f: 1.0 / 299.324_964_6,
    },
    Ellipsoid {
        id: "bessel1841",
        name: "Bessel 1841",
        a: 6_377_397.155,
        f: 1.0 / 299.152_812_8,
    },
    Ellipsoid {
        id: "krassovsky1940",
        name: "Krassovsky 1940",
        a: 6_378_245.0,
        f: 1.0 / 298.3,
    },
];

pub const IDS: &[&str] = &[
    "wgs84",
    "grs80",
    "clarke1866",
    "intl1924",
    "airy1830",
    "bessel1841",
    "krassovsky1940",
];

/// Above this |f| the series method loses accuracy and GeodesicExact is required.
pub const SERIES_LIMIT: f64 = 0.02;

/// The ellipsoid input fields, for tools to append to their inputs.
pub const FIELDS: [Field; 3] = [
    Field::new(
        "ellipsoid",
        "Ellipsoid",
        "wgs84 (default), grs80, clarke1866, intl1924, airy1830, bessel1841, or krassovsky1940",
        Kind::Choice(IDS),
    )
    // Which ellipsoid a result is on is not an assumption to leave invisible
    // (ux/glanceable-results, "Progressive disclosure with visible defaults").
    .core(),
    Field::new(
        "a",
        "Custom semi-major axis",
        "With inverse flattening, like 3396190 m (Mars)",
        Kind::Quantity {
            q: Quantity::Length,
            unit: "m",
        },
    ),
    Field::new(
        "inverse_flattening",
        "Custom inverse flattening",
        "Like 169.894; 0 for a sphere",
        Kind::Number {
            min: 0.0,
            max: 1e12,
        },
    ),
];

impl Ellipsoid {
    /// Reads the ellipsoid inputs: a catalog id, or a custom (a, 1/f).
    pub fn from_ctx(ctx: &mut Ctx) -> Result<Ellipsoid, ToolError> {
        let custom_a = ctx.quantity("a")?;
        let inv_f = ctx.number("inverse_flattening")?;
        match (custom_a, inv_f) {
            (Some(a), Some(rf)) => {
                if ctx.is_set("ellipsoid") {
                    return Err(ToolError::invalid(
                        "/ellipsoid",
                        "Give a catalog ellipsoid or a custom a and inverse flattening, not both.",
                    ));
                }
                let a = a.to(units::by_symbol(Quantity::Length, "m").expect("m"));
                // From small moons to the gas giants: 1 km to 100,000 km.
                if !(1_000.0..=1e8).contains(&a) {
                    return Err(ToolError::new(
                        gp_base::ErrorCode::OutOfDomain,
                        "The semi-major axis must be between 1 km and 100,000 km.",
                    )
                    .at("/a"));
                }
                if rf != 0.0 && rf < 1.0 {
                    return Err(ToolError::invalid(
                        "/inverse_flattening",
                        "Inverse flattening must be 0 (a sphere) or at least 1.",
                    ));
                }
                Ok(Ellipsoid {
                    id: "custom",
                    name: "custom",
                    a,
                    f: if rf == 0.0 { 0.0 } else { 1.0 / rf },
                })
            }
            (None, None) => {
                let id = ctx.choice("ellipsoid")?.unwrap_or("wgs84");
                Ok(*CATALOG
                    .iter()
                    .find(|e| e.id == id)
                    .expect("choice lists the catalog"))
            }
            _ => Err(ToolError::invalid(
                "/a",
                "A custom ellipsoid needs both a and inverse_flattening.",
            )),
        }
    }

    /// The geodesic engine for this ellipsoid, or `UNSUPPORTED` when the
    /// flattening needs the exact method, which only the geodesic distance
    /// and destination tools use (`crate::exact`).
    pub fn geodesic(&self) -> Result<Geodesic, ToolError> {
        if self.f.abs() > SERIES_LIMIT {
            return Err(ToolError::new(
                ErrorCode::Unsupported,
                "This ellipsoid is too flattened for the series method (|f| > 0.02). The geodesic distance and destination tools use the exact method, for |f| up to 0.5; this tool does not.",
            )
            .at("/inverse_flattening"));
        }
        Ok(Geodesic::new(self.a, self.f))
    }

    /// "WGS 84" or "custom a = 3396190 m, 1/f = 169.894", for `meta.model`.
    pub fn describe(&self) -> String {
        if self.id == "custom" {
            let show = |x: f64| gp_base::num::format_f64(x).unwrap_or_default();
            let rf = if self.f == 0.0 {
                "0 (sphere)".to_owned()
            } else {
                show(1.0 / self.f)
            };
            format!("custom ellipsoid a = {} m, 1/f = {rf}", show(self.a))
        } else {
            self.name.to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_match() {
        assert_eq!(IDS.len(), CATALOG.len());
        for (id, e) in IDS.iter().zip(CATALOG) {
            assert_eq!(*id, e.id);
            assert!(e.f > 0.0 && e.f < SERIES_LIMIT);
        }
    }
}
