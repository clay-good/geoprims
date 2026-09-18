//! Allow-listed pair endpoints (tool-catalog "Generated endpoints must be
//! meaningful"). Each pair is its parent operation with `to` preset and the
//! input unit defaulted; it carries no separate math. A pair is added only
//! with a justification naming the real task it serves.

use gp_base::tool::{Example, Field, Kind, Related, ToolDef};
use gp_base::units::Quantity as QT;

use super::*;

macro_rules! pair {
    ($name:ident, $parent:ident, $group:literal, $q:expr, $slug:literal, $from:literal, $to:literal,
     $title:literal, $example:literal, $why:literal) => {
        pub static $name: ToolDef = ToolDef {
            id: concat!("units.", $group, ".", $slug),
            title: $title,
            summary: concat!(
                "Converts ",
                $from,
                " to ",
                $to,
                " using exact unit definitions."
            ),
            keywords: &[$from, $to, "convert"],
            inputs: &[Field::new(
                "value",
                "Value",
                concat!("A number in ", $from, ", like ", $example),
                Kind::Quantity { q: $q, unit: $from },
            )
            .required()
            .core()],
            outputs: $parent.outputs,
            warnings: $parent.warnings,
            model: $parent.model,
            accuracy: $parent.accuracy,
            references: $parent.references,
            examples: &[Example {
                id: "primary",
                title: concat!($example, " ", $from, " in ", $to),
                input: concat!("{\"value\":", $example, "}"),
                source: EXACT_SOURCE,
            }],
            primary_example: "primary",
            visualization: $parent.visualization,
            related: &[Related {
                id: concat!("units.", $group, ".convert"),
                reason: "parent",
            }],
            composed_of: &[concat!("units.", $group, ".convert")],
            parent: Some(&$parent),
            preset: &[("to", concat!("\"", $to, "\""))],
            justification: $why,
            sentence: $parent.sentence,
            limits: $parent.limits,
            run: $parent.run,
            ..ToolDef::BLANK
        };
    };
}

pair!(
    KT_TO_MPH,
    SPEED,
    "speed",
    QT::Speed,
    "kt-to-mph",
    "kt",
    "mph",
    "Knots to mph",
    "100",
    "Pilots compare airspeed with ground vehicles and older US aircraft manuals in mph."
);
pair!(
    MPH_TO_KT,
    SPEED,
    "speed",
    QT::Speed,
    "mph-to-kt",
    "mph",
    "kt",
    "Mph to knots",
    "120",
    "Older US aircraft manuals (pre-1976 POHs) state speeds in mph; pilots convert them to knots."
);
pair!(
    KT_TO_KMH,
    SPEED,
    "speed",
    QT::Speed,
    "kt-to-kmh",
    "kt",
    "km/h",
    "Knots to km/h",
    "100",
    "Drone and glider operators outside the US plan in km/h against forecasts in knots."
);
pair!(
    KMH_TO_KT,
    SPEED,
    "speed",
    QT::Speed,
    "kmh-to-kt",
    "km/h",
    "kt",
    "Km/h to knots",
    "50",
    "Converting drone and wind speeds given in km/h to aviation knots."
);
pair!(
    MPS_TO_KT,
    SPEED,
    "speed",
    QT::Speed,
    "mps-to-kt",
    "m/s",
    "kt",
    "M/s to knots",
    "10",
    "METARs in many countries and drone specs give wind in m/s; pilots think in knots."
);
pair!(
    KT_TO_MPS,
    SPEED,
    "speed",
    QT::Speed,
    "kt-to-mps",
    "kt",
    "m/s",
    "Knots to m/s",
    "20",
    "Drone wind limits are published in m/s while aviation forecasts use knots."
);
pair!(
    FT_TO_M,
    LENGTH,
    "length",
    QT::Length,
    "ft-to-m",
    "ft",
    "m",
    "Feet to meters",
    "5280",
    "Altitudes and elevations in feet must be converted for metric charts and drone rules."
);
pair!(
    M_TO_FT,
    LENGTH,
    "length",
    QT::Length,
    "m-to-ft",
    "m",
    "ft",
    "Meters to feet",
    "120",
    "Drone altitude limits (120 m in EASA rules) are compared with US 400 ft limits."
);
pair!(
    NM_TO_KM,
    LENGTH,
    "length",
    QT::Length,
    "nm-to-km",
    "NM",
    "km",
    "Nautical miles to kilometers",
    "100",
    "Converting aviation and marine distances for metric maps."
);
pair!(
    KM_TO_NM,
    LENGTH,
    "length",
    QT::Length,
    "km-to-nm",
    "km",
    "NM",
    "Kilometers to nautical miles",
    "100",
    "Converting metric map distances to aviation and marine nautical miles."
);
pair!(
    NM_TO_MI,
    LENGTH,
    "length",
    QT::Length,
    "nm-to-mi",
    "NM",
    "mi",
    "Nautical miles to statute miles",
    "100",
    "Visibility is in statute miles while distances are in nautical miles on US charts."
);
pair!(
    MI_TO_NM,
    LENGTH,
    "length",
    QT::Length,
    "mi-to-nm",
    "mi",
    "NM",
    "Statute miles to nautical miles",
    "100",
    "Converting road or statute distances to nautical miles for flight planning."
);
pair!(
    MI_TO_KM,
    LENGTH,
    "length",
    QT::Length,
    "mi-to-km",
    "mi",
    "km",
    "Miles to kilometers",
    "10",
    "Everyday conversion of statute miles to kilometers."
);
pair!(
    KM_TO_MI,
    LENGTH,
    "length",
    QT::Length,
    "km-to-mi",
    "km",
    "mi",
    "Kilometers to miles",
    "10",
    "Everyday conversion of kilometers to statute miles."
);
pair!(
    FTUS_TO_M,
    LENGTH,
    "length",
    QT::Length,
    "ftus-to-m",
    "ftUS",
    "m",
    "US survey feet to meters",
    "1000000",
    "Legacy State Plane coordinates in US survey feet must be converted to meters."
);
pair!(
    M_TO_FTUS,
    LENGTH,
    "length",
    QT::Length,
    "m-to-ftus",
    "m",
    "ftUS",
    "Meters to US survey feet",
    "304800.6096",
    "Writing legacy deliverables that still require US survey feet."
);
pair!(
    FTUS_TO_FT,
    LENGTH,
    "length",
    QT::Length,
    "ftus-to-ft",
    "ftUS",
    "ft",
    "US survey feet to international feet",
    "1000000",
    "The 2 ppm survey-foot difference causes real errors on large coordinates after the 2023 deprecation."
);
pair!(
    IN_TO_MM,
    LENGTH,
    "length",
    QT::Length,
    "in-to-mm",
    "in",
    "mm",
    "Inches to millimeters",
    "1",
    "Sensor, propeller, and fastener sizes are quoted in inches and millimeters."
);
pair!(
    MM_TO_IN,
    LENGTH,
    "length",
    QT::Length,
    "mm-to-in",
    "mm",
    "in",
    "Millimeters to inches",
    "25.4",
    "Converting metric sensor and part sizes to inches."
);
pair!(
    INHG_TO_HPA,
    PRESSURE,
    "pressure",
    QT::Pressure,
    "inhg-to-hpa",
    "inHg",
    "hPa",
    "InHg to hPa",
    "29.92",
    "US altimeter settings are in inHg; most of the world uses hPa."
);
pair!(
    HPA_TO_INHG,
    PRESSURE,
    "pressure",
    QT::Pressure,
    "hpa-to-inhg",
    "hPa",
    "inHg",
    "HPa to inHg",
    "1013.25",
    "Pilots flying abroad convert QNH in hPa to inHg for US altimeters."
);
pair!(
    PSI_TO_KPA,
    PRESSURE,
    "pressure",
    QT::Pressure,
    "psi-to-kpa",
    "psi",
    "kPa",
    "Psi to kPa",
    "30",
    "Tire and oxygen pressures are specified in psi and kPa."
);
pair!(
    KPA_TO_PSI,
    PRESSURE,
    "pressure",
    QT::Pressure,
    "kpa-to-psi",
    "kPa",
    "psi",
    "KPa to psi",
    "200",
    "Converting metric pressure specs to psi."
);
pair!(
    C_TO_F,
    TEMPERATURE,
    "temperature",
    QT::Temperature,
    "c-to-f",
    "degC",
    "degF",
    "Celsius to Fahrenheit",
    "15",
    "METAR temperatures are in °C; many US pilots and crews think in °F."
);
pair!(
    F_TO_C,
    TEMPERATURE,
    "temperature",
    QT::Temperature,
    "f-to-c",
    "degF",
    "degC",
    "Fahrenheit to Celsius",
    "59",
    "Converting local temperatures in °F to °C for performance charts."
);
pair!(
    GAL_TO_L,
    VOLUME,
    "volume",
    QT::Volume,
    "gal-to-l",
    "galUS",
    "L",
    "US gallons to liters",
    "50",
    "Fuel quantities cross between US gallons and liters when refueling abroad."
);
pair!(
    L_TO_GAL,
    VOLUME,
    "volume",
    QT::Volume,
    "l-to-gal",
    "L",
    "galUS",
    "Liters to US gallons",
    "100",
    "Converting fuel uplift in liters to US gallons."
);
pair!(
    LB_TO_KG,
    MASS,
    "mass",
    QT::Mass,
    "lb-to-kg",
    "lb",
    "kg",
    "Pounds to kilograms",
    "2550",
    "Aircraft weights in pounds are converted to kilograms for EASA and ICAO paperwork."
);
pair!(
    KG_TO_LB,
    MASS,
    "mass",
    QT::Mass,
    "kg-to-lb",
    "kg",
    "lb",
    "Kilograms to pounds",
    "25",
    "Drone takeoff weights (25 kg limits) are compared with US pound limits."
);
pair!(
    AC_TO_HA,
    AREA,
    "area",
    QT::Area,
    "ac-to-ha",
    "ac",
    "ha",
    "Acres to hectares",
    "40",
    "Land and survey areas convert between acres and hectares."
);
pair!(
    HA_TO_AC,
    AREA,
    "area",
    QT::Area,
    "ha-to-ac",
    "ha",
    "ac",
    "Hectares to acres",
    "10",
    "Converting metric parcel areas to acres."
);
pair!(
    FT2_TO_AC,
    AREA,
    "area",
    QT::Area,
    "ft2-to-ac",
    "ft2",
    "ac",
    "Square feet to acres",
    "43560",
    "Parcel areas computed in square feet are reported in acres on deeds and plats."
);
pair!(
    FPM_TO_MPS,
    VERTICAL_SPEED,
    "vertical-speed",
    QT::VerticalSpeed,
    "fpm-to-mps",
    "ft/min",
    "m/s",
    "Ft/min to m/s",
    "500",
    "Climb rates in ft/min are compared with drone and glider climb rates in m/s."
);
pair!(
    DEG_TO_RAD,
    ANGLE,
    "angle",
    QT::Angle,
    "deg-to-rad",
    "deg",
    "rad",
    "Degrees to radians",
    "180",
    "Developers convert degrees to radians before calling trigonometric functions."
);
pair!(
    RAD_TO_DEG,
    ANGLE,
    "angle",
    QT::Angle,
    "rad-to-deg",
    "rad",
    "deg",
    "Radians to degrees",
    "1",
    "Developers convert library output in radians back to degrees."
);
