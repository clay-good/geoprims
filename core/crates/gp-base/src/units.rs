//! The unit registry (units-and-quantities spec).
//!
//! Every defined constant is stored as an exact rational so that a conversion
//! between two units uses one correctly rounded ratio (for example
//! kt → mph = 1852000/1609344) instead of accumulating error through SI.
//! Sources: NIST SP 811 (2008) Appendix B, NIST Handbook 44 (2026) Appendix C,
//! BIPM SI Brochure 9th ed., ICAO Annex 5 (5th ed.).
//!
//! Angles use degrees as the registry base because degrees are the boundary unit
//! and keep every angle constant except the radian exact.

/// A physical quantity a numeric field can hold (`x-quantity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quantity {
    /// Heights, elevations, short lengths.
    Length,
    /// Horizontal ranges. Same units as `Length`; unit profiles pick NM, km, or mi.
    Distance,
    Area,
    Volume,
    Mass,
    Speed,
    VerticalSpeed,
    Acceleration,
    Pressure,
    Temperature,
    TemperatureDifference,
    Angle,
    AngularRate,
    Time,
    Energy,
    Power,
    ElectricCharge,
    ElectricPotential,
    Density,
    Frequency,
    DataRate,
    Slope,
    Dimensionless,
}

impl Quantity {
    pub const ALL: [Quantity; 23] = [
        Self::Length,
        Self::Distance,
        Self::Area,
        Self::Volume,
        Self::Mass,
        Self::Speed,
        Self::VerticalSpeed,
        Self::Acceleration,
        Self::Pressure,
        Self::Temperature,
        Self::TemperatureDifference,
        Self::Angle,
        Self::AngularRate,
        Self::Time,
        Self::Energy,
        Self::Power,
        Self::ElectricCharge,
        Self::ElectricPotential,
        Self::Density,
        Self::Frequency,
        Self::DataRate,
        Self::Slope,
        Self::Dimensionless,
    ];

    /// The `x-quantity` identifier.
    pub fn id(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Distance => "distance",
            Self::Area => "area",
            Self::Volume => "volume",
            Self::Mass => "mass",
            Self::Speed => "speed",
            Self::VerticalSpeed => "vertical-speed",
            Self::Acceleration => "acceleration",
            Self::Pressure => "pressure",
            Self::Temperature => "temperature",
            Self::TemperatureDifference => "temperature-difference",
            Self::Angle => "angle",
            Self::AngularRate => "angular-rate",
            Self::Time => "time",
            Self::Energy => "energy",
            Self::Power => "power",
            Self::ElectricCharge => "electric-charge",
            Self::ElectricPotential => "electric-potential",
            Self::Density => "density",
            Self::Frequency => "frequency",
            Self::DataRate => "data-rate",
            Self::Slope => "slope",
            Self::Dimensionless => "dimensionless",
        }
    }

    pub fn from_id(id: &str) -> Option<Quantity> {
        Self::ALL.into_iter().find(|q| q.id() == id)
    }

    /// Plain-language name with article, for messages ("expects a speed").
    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "a length",
            Self::Distance => "a distance",
            Self::Area => "an area",
            Self::Volume => "a volume",
            Self::Mass => "a mass",
            Self::Speed => "a speed",
            Self::VerticalSpeed => "a vertical speed",
            Self::Acceleration => "an acceleration",
            Self::Pressure => "a pressure",
            Self::Temperature => "a temperature",
            Self::TemperatureDifference => "a temperature difference",
            Self::Angle => "an angle",
            Self::AngularRate => "an angular rate",
            Self::Time => "a time",
            Self::Energy => "an energy",
            Self::Power => "a power",
            Self::ElectricCharge => "an electric charge",
            Self::ElectricPotential => "a voltage",
            Self::Density => "a density",
            Self::Frequency => "a frequency",
            Self::DataRate => "a data rate",
            Self::Slope => "a slope",
            Self::Dimensionless => "a plain number",
        }
    }

    /// The quantity whose units this one shares (distance is measured in lengths).
    pub fn dimension(self) -> Quantity {
        match self {
            Self::Distance => Self::Length,
            q => q,
        }
    }
}

/// An exact rational with a positive denominator, in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    pub num: i128,
    pub den: i128,
}

const fn gcd(mut a: i128, mut b: i128) -> i128 {
    if a < 0 {
        a = -a;
    }
    if b < 0 {
        b = -b;
    }
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Ratio {
    pub const fn new(num: i128, den: i128) -> Ratio {
        let g = gcd(num, den);
        let g = if g == 0 { 1 } else { g };
        let s = if den < 0 { -1 } else { 1 };
        Ratio {
            num: s * num / g,
            den: s * den / g,
        }
    }

    pub const ZERO: Ratio = Ratio { num: 0, den: 1 };

    pub fn times(self, o: Ratio) -> Ratio {
        let a = Ratio::new(self.num, o.den);
        let b = Ratio::new(o.num, self.den);
        Ratio::new(a.num * b.num, a.den * b.den)
    }

    pub fn over(self, o: Ratio) -> Ratio {
        self.times(Ratio::new(o.den, o.num))
    }

    pub fn minus(self, o: Ratio) -> Ratio {
        let g = gcd(self.den, o.den);
        let l = self.den / g * o.den;
        Ratio::new(self.num * (l / self.den) - o.num * (l / o.den), l)
    }

    /// The correctly rounded binary64 value when both parts are exactly representable.
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

/// How a unit maps to its registry base unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scale {
    /// base = (x + offset) × factor, both exact. Offsets are nonzero only for
    /// absolute temperatures.
    Exact { factor: Ratio, offset: Ratio },
    /// base = x × factor, where the factor is irrational (involves π).
    Approx(f64),
}

#[derive(Debug, PartialEq)]
pub struct Unit {
    /// Canonical symbol, the value of `x-unit` and of `"unit"` in results.
    pub symbol: &'static str,
    pub quantity: Quantity,
    pub scale: Scale,
    /// Accepted spellings besides the symbol. Resolution is case-sensitive and
    /// scoped to the field's quantity.
    pub aliases: &'static [&'static str],
}

const fn ex(num: i128, den: i128) -> Scale {
    Scale::Exact {
        factor: Ratio::new(num, den),
        offset: Ratio::ZERO,
    }
}

const fn aff(num: i128, den: i128, off_num: i128, off_den: i128) -> Scale {
    Scale::Exact {
        factor: Ratio::new(num, den),
        offset: Ratio::new(off_num, off_den),
    }
}

const DEG_PER_RAD: f64 = 180.0 / core::f64::consts::PI;

const fn u(
    symbol: &'static str,
    quantity: Quantity,
    scale: Scale,
    aliases: &'static [&'static str],
) -> Unit {
    Unit {
        symbol,
        quantity,
        scale,
        aliases,
    }
}

use Quantity as Q;

// 1 ft = 0.3048 m; 1 lb = 0.45359237 kg; g0 = 9.80665 m/s² (all exact by definition).
const LBF_NUM: i128 = 45_359_237 * 980_665; // lbf in N × 10^13
const E13: i128 = 10_000_000_000_000;

/// The registry. The first unit listed for a quantity is its base unit (factor 1).
pub static UNITS: &[Unit] = &[
    // Length (base m)
    u(
        "m",
        Q::Length,
        ex(1, 1),
        &["meter", "meters", "metre", "metres"],
    ),
    u(
        "km",
        Q::Length,
        ex(1000, 1),
        &["kilometer", "kilometers", "kilometre", "kilometres"],
    ),
    u("cm", Q::Length, ex(1, 100), &["centimeter", "centimeters"]),
    u("mm", Q::Length, ex(1, 1000), &["millimeter", "millimeters"]),
    u(
        "Mm",
        Q::Length,
        ex(1_000_000, 1),
        &["megameter", "megameters"],
    ),
    u(
        "ft",
        Q::Length,
        ex(3048, 10_000),
        &["feet", "foot", "'", "′"],
    ),
    u(
        "ftUS",
        Q::Length,
        ex(1200, 3937),
        &["usft", "US-ft", "survey-ft"],
    ),
    u(
        "in",
        Q::Length,
        ex(254, 10_000),
        &["inch", "inches", "\"", "″"],
    ),
    u("yd", Q::Length, ex(9144, 10_000), &["yard", "yards"]),
    u(
        "mi",
        Q::Length,
        ex(1_609_344, 1000),
        &["mile", "miles", "SM", "sm"],
    ),
    u(
        "NM",
        Q::Length,
        ex(1852, 1),
        &["nmi", "NMI", "nautical-mile", "nautical-miles"],
    ),
    // Area (base m²)
    u("m2", Q::Area, ex(1, 1), &["m²", "sq-m"]),
    u("km2", Q::Area, ex(1_000_000, 1), &["km²", "sq-km"]),
    u("ha", Q::Area, ex(10_000, 1), &["hectare", "hectares"]),
    u(
        "ac",
        Q::Area,
        ex(40_468_564_224, 10_000_000),
        &["acre", "acres"],
    ),
    u(
        "ft2",
        Q::Area,
        ex(9_290_304, 100_000_000),
        &["ft²", "sq-ft", "sqft"],
    ),
    u(
        "mi2",
        Q::Area,
        ex(1_609_344 * 1_609_344, 1_000_000),
        &["mi²", "sq-mi"],
    ),
    u("NM2", Q::Area, ex(1852 * 1852, 1), &["NM²"]),
    // Volume (base m³)
    u("m3", Q::Volume, ex(1, 1), &["m³", "cu-m"]),
    u(
        "L",
        Q::Volume,
        ex(1, 1000),
        &["l", "liter", "liters", "litre", "litres"],
    ),
    u(
        "mL",
        Q::Volume,
        ex(1, 1_000_000),
        &["ml", "milliliter", "milliliters"],
    ),
    u(
        "galUS",
        Q::Volume,
        ex(3_785_411_784, 1_000_000_000_000),
        &["gal", "USG", "usg", "gallon", "gallons"],
    ),
    u("galImp", Q::Volume, ex(454_609, 100_000_000), &["imp-gal"]),
    u(
        "ft3",
        Q::Volume,
        ex(3048 * 3048 * 3048, 1_000_000_000_000),
        &["ft³", "cu-ft", "cf"],
    ),
    u(
        "yd3",
        Q::Volume,
        ex(9144 * 9144 * 9144, 1_000_000_000_000),
        &["yd³", "cu-yd", "CY"],
    ),
    // Mass (base kg)
    u("kg", Q::Mass, ex(1, 1), &["kilogram", "kilograms"]),
    u("g", Q::Mass, ex(1, 1000), &["gram", "grams"]),
    u(
        "lb",
        Q::Mass,
        ex(45_359_237, 100_000_000),
        &["lbs", "lbm", "pound", "pounds"],
    ),
    u(
        "oz",
        Q::Mass,
        ex(45_359_237, 1_600_000_000),
        &["ounce", "ounces"],
    ),
    u("t", Q::Mass, ex(1000, 1), &["tonne", "tonnes"]),
    // Speed (base m/s)
    u("m/s", Q::Speed, ex(1, 1), &["mps"]),
    u("km/h", Q::Speed, ex(1000, 3600), &["kmh", "kph", "km/hr"]),
    u(
        "kt",
        Q::Speed,
        ex(1852, 3600),
        &["kts", "kn", "knot", "knots", "KT", "KTS"],
    ),
    u("mph", Q::Speed, ex(1_609_344, 3_600_000), &["mi/h"]),
    u("ft/s", Q::Speed, ex(3048, 10_000), &["fps"]),
    // Vertical speed (base m/s)
    u("m/s", Q::VerticalSpeed, ex(1, 1), &["mps"]),
    u(
        "ft/min",
        Q::VerticalSpeed,
        ex(3048, 600_000),
        &["fpm", "FPM"],
    ),
    u("m/min", Q::VerticalSpeed, ex(1, 60), &[]),
    u("ft/s", Q::VerticalSpeed, ex(3048, 10_000), &["fps"]),
    // Acceleration (base m/s²)
    u("m/s2", Q::Acceleration, ex(1, 1), &["m/s²"]),
    u("g0", Q::Acceleration, ex(980_665, 100_000), &["g", "G"]),
    u("ft/s2", Q::Acceleration, ex(3048, 10_000), &["ft/s²"]),
    // Pressure (base Pa)
    u("Pa", Q::Pressure, ex(1, 1), &["pascal", "pascals"]),
    u(
        "hPa",
        Q::Pressure,
        ex(100, 1),
        &["hectopascal", "hectopascals"],
    ),
    u("kPa", Q::Pressure, ex(1000, 1), &[]),
    u(
        "mbar",
        Q::Pressure,
        ex(100, 1),
        &["mb", "millibar", "millibars"],
    ),
    u("bar", Q::Pressure, ex(100_000, 1), &[]),
    u(
        "inHg",
        Q::Pressure,
        ex(3_386_389, 1000),
        &["inhg", "\"Hg", "″Hg", "in-Hg"],
    ),
    u(
        "mmHg",
        Q::Pressure,
        ex(133_322_387_415, 1_000_000_000),
        &["torr"],
    ),
    u("psi", Q::Pressure, ex(LBF_NUM, 100_000 * 64_516), &[]),
    u("atm", Q::Pressure, ex(101_325, 1), &[]),
    // Temperature (base K)
    u("K", Q::Temperature, aff(1, 1, 0, 1), &["kelvin"]),
    u(
        "degC",
        Q::Temperature,
        aff(1, 1, 27_315, 100),
        &["°C", "℃", "C", "celsius"],
    ),
    u(
        "degF",
        Q::Temperature,
        aff(5, 9, 45_967, 100),
        &["°F", "℉", "F", "fahrenheit"],
    ),
    // Temperature difference (base K)
    u("K", Q::TemperatureDifference, ex(1, 1), &["kelvin"]),
    u(
        "degC",
        Q::TemperatureDifference,
        ex(1, 1),
        &["°C", "℃", "C", "celsius"],
    ),
    u(
        "degF",
        Q::TemperatureDifference,
        ex(5, 9),
        &["°F", "℉", "F", "fahrenheit"],
    ),
    // Angle (base deg)
    u("deg", Q::Angle, ex(1, 1), &["°", "degree", "degrees"]),
    u(
        "rad",
        Q::Angle,
        Scale::Approx(DEG_PER_RAD),
        &["radian", "radians"],
    ),
    u("gon", Q::Angle, ex(9, 10), &["grad", "gradian", "gradians"]),
    u("arcmin", Q::Angle, ex(1, 60), &["'", "′"]),
    u("arcsec", Q::Angle, ex(1, 3600), &["\"", "″"]),
    u("mil-nato", Q::Angle, ex(360, 6400), &[]),
    u("mil-warsaw", Q::Angle, ex(360, 6000), &[]),
    u("mil-sweden", Q::Angle, ex(360, 6300), &[]),
    u(
        "mrad",
        Q::Angle,
        Scale::Approx(DEG_PER_RAD / 1000.0),
        &["milliradian", "milliradians"],
    ),
    u("turn", Q::Angle, ex(360, 1), &["rev", "revolution"]),
    // Angular rate (base deg/s)
    u("deg/s", Q::AngularRate, ex(1, 1), &["°/s"]),
    u("deg/min", Q::AngularRate, ex(1, 60), &["°/min"]),
    u("rad/s", Q::AngularRate, Scale::Approx(DEG_PER_RAD), &[]),
    u("rpm", Q::AngularRate, ex(6, 1), &["rev/min"]),
    // Time (base s)
    u("s", Q::Time, ex(1, 1), &["sec", "second", "seconds"]),
    u("ms", Q::Time, ex(1, 1000), &["millisecond", "milliseconds"]),
    u("min", Q::Time, ex(60, 1), &["minute", "minutes"]),
    u("h", Q::Time, ex(3600, 1), &["hr", "hour", "hours"]),
    u("d", Q::Time, ex(86_400, 1), &["day", "days"]),
    // Energy (base J)
    u("J", Q::Energy, ex(1, 1), &["joule", "joules"]),
    u("kJ", Q::Energy, ex(1000, 1), &[]),
    u("MJ", Q::Energy, ex(1_000_000, 1), &[]),
    u("Wh", Q::Energy, ex(3600, 1), &[]),
    u("kWh", Q::Energy, ex(3_600_000, 1), &[]),
    // Power (base W); mechanical horsepower = 550 ft·lbf/s exactly
    u("W", Q::Power, ex(1, 1), &["watt", "watts"]),
    u("kW", Q::Power, ex(1000, 1), &[]),
    u(
        "hp",
        Q::Power,
        ex(550 * 3048 * LBF_NUM, 10_000 * E13),
        &["HP", "horsepower"],
    ),
    // Electric charge (base C)
    u("C", Q::ElectricCharge, ex(1, 1), &["coulomb", "coulombs"]),
    u("mAh", Q::ElectricCharge, ex(36, 10), &[]),
    u("Ah", Q::ElectricCharge, ex(3600, 1), &[]),
    // Electric potential (base V)
    u("V", Q::ElectricPotential, ex(1, 1), &["volt", "volts"]),
    u("mV", Q::ElectricPotential, ex(1, 1000), &[]),
    u("kV", Q::ElectricPotential, ex(1000, 1), &[]),
    // Density (base kg/m³)
    u("kg/m3", Q::Density, ex(1, 1), &["kg/m³"]),
    u("g/cm3", Q::Density, ex(1000, 1), &["g/cm³", "g/cc", "kg/L"]),
    u(
        "lb/galUS",
        Q::Density,
        ex(45_359_237 * 10_000, 3_785_411_784),
        &["lb/gal", "lb/USG"],
    ),
    u(
        "lb/ft3",
        Q::Density,
        ex(
            45_359_237 * 1_000_000_000_000,
            100_000_000 * 3048 * 3048 * 3048,
        ),
        &["lb/ft³", "pcf"],
    ),
    // Frequency (base Hz)
    u("Hz", Q::Frequency, ex(1, 1), &["hertz"]),
    u("kHz", Q::Frequency, ex(1000, 1), &[]),
    u("MHz", Q::Frequency, ex(1_000_000, 1), &[]),
    u("GHz", Q::Frequency, ex(1_000_000_000, 1), &[]),
    // Data rate (base bit/s)
    u("bit/s", Q::DataRate, ex(1, 1), &["bps"]),
    u("kbit/s", Q::DataRate, ex(1000, 1), &["kbps", "kb/s"]),
    u(
        "Mbit/s",
        Q::DataRate,
        ex(1_000_000, 1),
        &["Mbps", "Mb/s", "mb"],
    ),
    u(
        "Gbit/s",
        Q::DataRate,
        ex(1_000_000_000, 1),
        &["Gbps", "Gb/s"],
    ),
    // Slope (base rise/run ratio); slope in degrees is nonlinear and lives in the slope tool
    u("ratio", Q::Slope, ex(1, 1), &["rise/run"]),
    u("%", Q::Slope, ex(1, 100), &["percent", "pct"]),
    u("‰", Q::Slope, ex(1, 1000), &["permille", "per-mille"]),
    // Dimensionless
    u("1", Q::Dimensionless, ex(1, 1), &[]),
];

/// The registry base unit for a quantity.
pub fn base_unit(q: Quantity) -> &'static Unit {
    let d = q.dimension();
    UNITS
        .iter()
        .find(|u| u.quantity == d)
        .expect("every quantity has a base unit")
}

/// All units a field of quantity `q` accepts.
pub fn units_of(q: Quantity) -> impl Iterator<Item = &'static Unit> {
    let d = q.dimension();
    UNITS.iter().filter(move |u| u.quantity == d)
}

/// Finds a unit by exact symbol or alias within quantity `q`.
pub fn lookup(q: Quantity, text: &str) -> Option<&'static Unit> {
    units_of(q).find(|u| u.symbol == text || u.aliases.contains(&text))
}

/// Finds a unit by canonical symbol in any quantity (first match in registry order).
pub fn any_by_symbol(symbol: &str) -> Option<&'static Unit> {
    UNITS.iter().find(|u| u.symbol == symbol)
}

/// Finds a unit by its canonical symbol within quantity `q`.
pub fn by_symbol(q: Quantity, symbol: &str) -> Option<&'static Unit> {
    units_of(q).find(|u| u.symbol == symbol)
}

/// Converts `x` from unit `a` to unit `b` (same quantity).
///
/// Exact units combine into one reduced ratio and offset first, so the only
/// rounding is `x × ratio (+ offset)`.
pub fn convert(x: f64, a: &Unit, b: &Unit) -> f64 {
    debug_assert_eq!(a.quantity, b.quantity, "convert across quantities");
    if core::ptr::eq(a, b) {
        return x;
    }
    match (a.scale, b.scale) {
        (
            Scale::Exact {
                factor: fa,
                offset: oa,
            },
            Scale::Exact {
                factor: fb,
                offset: ob,
            },
        ) => {
            // y = (x + oa)·fa/fb − ob = x·r + (oa·r − ob)
            let r = fa.over(fb);
            let c = oa.times(r).minus(ob);
            if c.num == 0 {
                return x * r.to_f64();
            }
            // Over a common denominator, y = (x·P + Q) / D with integer P, Q, D, so
            // integer-valued inputs (32 °F) give the correctly rounded answer (273.15 K).
            let d = r.den / gcd(r.den, c.den) * c.den;
            let p = r.num * (d / r.den);
            let q = c.num * (d / c.den);
            (x * p as f64 + q as f64) / d as f64
        }
        _ => x * factor_f64(a.scale) / factor_f64(b.scale),
    }
}

fn factor_f64(s: Scale) -> f64 {
    match s {
        Scale::Exact { factor, .. } => factor.to_f64(),
        Scale::Approx(f) => f,
    }
}

/// Converts `x` in unit `a` to its quantity's base unit.
pub fn to_base(x: f64, a: &Unit) -> f64 {
    convert(x, a, base_unit(a.quantity))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(q: Quantity, s: &str) -> &'static Unit {
        by_symbol(q, s).unwrap_or_else(|| panic!("no unit {s}"))
    }

    fn conv(q: Quantity, x: f64, a: &str, b: &str) -> f64 {
        convert(x, unit(q, a), unit(q, b))
    }

    #[test]
    fn constants_table_rows_are_exact() {
        assert_eq!(conv(Q::Length, 1.0, "ft", "m"), 0.3048);
        assert_eq!(conv(Q::Length, 1.0, "ftUS", "m"), 1200.0 / 3937.0);
        assert_eq!(conv(Q::Length, 1.0, "NM", "m"), 1852.0);
        assert_eq!(conv(Q::Length, 1.0, "mi", "m"), 1609.344);
        assert_eq!(conv(Q::Speed, 1.0, "kt", "m/s"), 1852.0 / 3600.0);
        assert_eq!(conv(Q::Mass, 1.0, "lb", "kg"), 0.45359237);
        assert_eq!(conv(Q::Volume, 1.0, "galUS", "L"), 3.785411784);
        assert_eq!(conv(Q::Pressure, 1.0, "inHg", "Pa"), 3386.389);
        assert_eq!(conv(Q::Pressure, 1.0, "hPa", "Pa"), 100.0);
        assert_eq!(conv(Q::Pressure, 1.0, "mbar", "Pa"), 100.0);
        // Exactly 8896443230521/1290320000 Pa = 6894.75729316836134… Pa; this is its nearest binary64.
        assert_eq!(conv(Q::Pressure, 1.0, "psi", "Pa"), 6894.757293168362);
        assert_eq!(conv(Q::Acceleration, 1.0, "g0", "m/s2"), 9.80665);
        assert_eq!(conv(Q::Temperature, 32.0, "degF", "K"), 273.15);
        assert_eq!(conv(Q::Temperature, 0.0, "degC", "K"), 273.15);
        assert_eq!(conv(Q::Temperature, 100.0, "degC", "degF"), 212.0);
        assert_eq!(conv(Q::Temperature, -40.0, "degF", "degC"), -40.0);
    }

    #[test]
    fn nautical_mile_exact() {
        assert_eq!(conv(Q::Distance, 1.0, "NM", "m"), 1852.0);
    }

    #[test]
    fn survey_foot_distinct() {
        let d = conv(Q::Length, 1e6, "ftUS", "m") - conv(Q::Length, 1e6, "ft", "m");
        assert!((d - 0.609601219).abs() <= 1e-9, "{d}");
    }

    #[test]
    fn kt_to_mph_pair_vector() {
        assert_eq!(conv(Q::Speed, 100.0, "kt", "mph"), 115.07794480235425);
    }

    #[test]
    fn mixed_unit_scenario_to_si() {
        assert_eq!(conv(Q::Length, 5280.0, "ft", "m"), 1609.344);
        assert_eq!(conv(Q::Temperature, 30.0, "degC", "K"), 303.15);
        let pa = conv(Q::Pressure, 29.92, "inHg", "Pa");
        assert!((pa - 101_320.76).abs() <= 0.01, "{pa}");
    }

    #[test]
    fn temperature_difference_is_not_offset() {
        assert_eq!(conv(Q::TemperatureDifference, 18.0, "degF", "K"), 10.0);
        assert_eq!(conv(Q::TemperatureDifference, 1.0, "degC", "degF"), 1.8);
    }

    #[test]
    fn every_quantity_has_a_base_unit_with_factor_one() {
        for q in Quantity::ALL {
            let b = base_unit(q);
            assert_eq!(b.scale, ex(1, 1), "{}", q.id());
            assert_eq!(Quantity::from_id(q.id()), Some(q));
        }
    }

    #[test]
    fn no_alias_is_ambiguous_within_a_quantity() {
        for q in Quantity::ALL {
            let mut seen: Vec<&str> = Vec::new();
            for u in units_of(q) {
                for name in core::iter::once(&u.symbol).chain(u.aliases) {
                    assert!(!seen.contains(name), "{name} twice in {}", q.id());
                    seen.push(name);
                }
            }
        }
    }

    #[test]
    fn exact_ratios_fit_binary64_exactly() {
        // A ratio part above 2^53 would round when converted, silently losing exactness.
        for a in UNITS {
            for b in units_of(a.quantity) {
                if let (Scale::Exact { factor: fa, .. }, Scale::Exact { factor: fb, .. }) =
                    (a.scale, b.scale)
                {
                    let r = fa.over(fb);
                    let limit = 1i128 << 53;
                    if r.num.abs() > limit || r.den > limit {
                        // Allowed, but then the result must still be within 1 ulp of the ratio.
                        let approx = fa.to_f64() / fb.to_f64();
                        assert!(
                            ((r.to_f64() - approx) / approx).abs() < 1e-15,
                            "{} → {}",
                            a.symbol,
                            b.symbol
                        );
                    }
                }
            }
        }
    }
}
