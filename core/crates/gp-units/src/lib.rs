//! The base module: gp-base plus the units domain (units-and-quantities spec,
//! "Standalone unit conversion tools"). 22 operations, one per quantity family
//! plus fuel volume↔mass, slope, and a general normalizer, and the allow-listed
//! pair endpoints generated from them (tool-catalog counting rule).

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::parse;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, Stability,
    ToolDef,
};
use gp_base::units::{self, Quantity};

mod pairs;

const NIST_811: Reference = Reference {
    title: "Guide for the Use of the International System of Units (SI), NIST Special Publication 811",
    issuer: "National Institute of Standards and Technology",
    year: 2008,
    edition: "2008 edition",
    locator: "Appendix B.8, Factors for units listed alphabetically",
    url: "https://www.nist.gov/pml/special-publication-811",
};

const NIST_HB44: Reference = Reference {
    title: "Specifications, Tolerances, and Other Technical Requirements for Weighing and Measuring Devices, NIST Handbook 44",
    issuer: "National Institute of Standards and Technology",
    year: 2026,
    edition: "2026 edition",
    locator: "Appendix C, General Tables of Units of Measurement",
    url: "https://www.nist.gov/pml/owm/publications/nist-handbooks/handbook-44",
};

const ICAO_ANNEX5: Reference = Reference {
    title: "Annex 5 to the Convention on International Civil Aviation: Units of Measurement to be Used in Air and Ground Operations",
    issuer: "International Civil Aviation Organization",
    year: 2010,
    edition: "5th edition",
    locator: "Chapter 3 and Table 3-3",
    url: "https://store.icao.int/en/annex-5-units-of-measurement-to-be-used-in-the-air-and-ground-services",
};

const FR_2019_SURVEY_FOOT: Reference = Reference {
    title: "Deprecation of the United States (U.S.) Survey Foot, 84 FR 60101",
    issuer: "National Institute of Standards and Technology and National Oceanic and Atmospheric Administration",
    year: 2019,
    edition: "Federal Register notice, October 5, 2020 final (85 FR 62698)",
    locator: "Effective January 1, 2023",
    url: "https://www.federalregister.gov/documents/2020/10/05/2020-21902/deprecation-of-the-united-states-us-survey-foot",
};

const EXACT_SOURCE: &str = "Exact unit definitions (NIST SP 811 Appendix B)";
const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];
const CONVERT_WARNINGS: &[&str] = &["UNIT_ASSUMED", "LEGACY_UNIT", "EXPERIMENTAL_TOOL"];
/// The same list once a converter is past the bar.
const CONVERT_STABLE: &[&str] = &["UNIT_ASSUMED", "LEGACY_UNIT"];
/// Conversions are exact, so show enough digits to see small differences such
/// as the 2 ppm survey-foot offset.
const P8: Precision = Precision::Significant(8);

/// Declares one `units.<group>.convert` operation.
macro_rules! convert_op {
    // Still experimental: no prose yet, and the experimental warning stays.
    (
        $name:ident, $group:literal, $q:expr, $unit:literal, $title:literal, $summary:literal,
        aliases: [$($a:literal),*], refs: [$($r:expr),*], example: ($ex_title:literal, $ex:literal)
    ) => {
        convert_op!($name, $group, $q, $unit, $title, $summary,
            aliases: [$($a),*], refs: [$($r),*], example: ($ex_title, $ex),
            stability: Stability::Experimental,
            warnings: CONVERT_WARNINGS, when: "", limits: "",
            related: [Related { id: "units.quantity.normalize", reason: "alternative" }]);
    };
    // Promoted: its own when-to-use, limitations and neighbours.
    (
        $name:ident, $group:literal, $q:expr, $unit:literal, $title:literal, $summary:literal,
        aliases: [$($a:literal),*], refs: [$($r:expr),*], example: ($ex_title:literal, $ex:literal),
        stability: $stab:expr, warnings: $warn:expr, when: $when:literal, limits: $lim:literal,
        related: [$($rel:expr),*]
    ) => {
        pub static $name: ToolDef = ToolDef {
            stability: $stab,
            when_to_use: $when,
            limitations: $lim,
            id: concat!("units.", $group, ".convert"),
            title: $title,
            summary: $summary,
            aliases: &[$($a),*],
            keywords: &["convert", "conversion", $group],
            inputs: &[
                Field::new("value", "Value", concat!("A number, or a number with a unit, like 12 ", $unit),
                    Kind::Quantity { q: $q, unit: $unit }).required().core(),
                Field::new("from", "From unit", concat!("The unit of a bare number, like ", $unit), Kind::Unit($q)),
                Field::new("to", "To unit", "The unit to convert to", Kind::Unit($q)).required().core(),
            ],
            outputs: &[
                Field::new("converted", "Converted", "The value in the target unit", Kind::Quantity { q: $q, unit: $unit }).precision(P8),
                Field::new("input", "Input", "The value as read, in its unit", Kind::Quantity { q: $q, unit: $unit }).precision(P8),
            ],
            warnings: $warn,
            errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
            model: "Exact unit definitions",
            accuracy: "Exact to double precision: each conversion is one correctly rounded ratio of exact definitions",
            references: &[$($r),*],
            examples: &[Example { id: "primary", title: $ex_title, input: $ex, source: EXACT_SOURCE }],
            primary_example: "primary",
            visualization: TABLE,
            related: &[$($rel),*],
            sentence: "{input} is {converted}.",
            limits: &[("batchRows", 10_000)],
            run: run_convert,
            ..ToolDef::BLANK
        };
    };
}

fn run_convert(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let from = ctx.unit("from")?;
    let value = ctx
        .quantity_or("value", from)?
        .expect("required input checked");
    if let Some(f) = from
        && !core::ptr::eq(value.unit, f)
    {
        return Err(ToolError::invalid(
            "/from",
            format!(
                "The value says {} but from says {}.",
                value.unit.symbol, f.symbol
            ),
        )
        .hint("Give the unit once: either in the value or in from."));
    }
    // A temperature below absolute zero is not a temperature. Converting it
    // gives a negative kelvin, which is not a colder temperature but a
    // different kind of thing, and the input is far more likely to be a
    // difference that reached the wrong tool -- put -300 through here as a
    // reading and it comes back -26.85 K, which looks like an answer.
    if value.unit.quantity == QT::Temperature {
        let kelvin = units::by_symbol(QT::Temperature, "K").expect("K");
        if value.to(kelvin) < 0.0 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "That is below absolute zero, which is 0 K, -273.15 °C or -459.67 °F.",
            )
            .at("/value")
            .hint("For a change in temperature rather than a reading, use units.temperature-difference.convert."));
        }
    }
    let to = ctx.req_unit("to")?;
    let converted = ctx.emit("converted", value, to);
    Ok(Json::obj([
        ("input", value.to_json()),
        ("converted", converted),
    ]))
}

use Quantity as QT;

convert_op!(LENGTH, "length", QT::Length, "ft", "Length converter",
"Converts lengths and distances: m, km, ft, US survey ft, in, yd, mi, and NM.",
aliases: ["feet to meters", "length conversion"], refs: [NIST_811, NIST_HB44, FR_2019_SURVEY_FOOT],
example: ("5,280 ft in meters", r#"{"value":"5280 ft","to":"m"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for any distance that has to move between systems: a runway length in feet against a chart in meters, a leg in nautical miles against a road distance in statute miles, a survey dimension in feet against a design in millimeters. The two kinds of foot are both here and kept apart, which matters more than it sounds: US survey feet and international feet differ by two parts per million, which is a tenth of a millimeter over a meter and two feet across a state plane zone.",
limits: "A conversion is exact and a measurement is not, so this changes the unit and never improves the number: 5,280 ft becomes 1,609.344 m exactly, but if the 5,280 was good to a foot the answer is good to 0.3 m. The US survey foot was withdrawn for new work at the end of 2022 and is kept here only for reading existing records; a value carrying it is flagged. Nautical miles are the international 1,852 m exactly, not the old British or US ones, and the statute mile is the international one -- an old chart may mean neither.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.area.convert", reason: "alternative" },
    Related { id: "units.speed.convert", reason: "alternative" }
]);
convert_op!(AREA, "area", QT::Area, "ac", "Area converter",
    "Converts areas: m², km², hectares, acres, ft², mi², and NM².",
    aliases: ["area conversion"], refs: [NIST_811, NIST_HB44],
    example: ("One acre in square feet", r#"{"value":"1 ac","to":"ft2"}"#));
convert_op!(VOLUME, "volume", QT::Volume, "galUS", "Volume converter",
    "Converts volumes: m³, liters, US and imperial gallons, ft³, and yd³.",
    aliases: ["volume conversion"], refs: [NIST_811, NIST_HB44],
    example: ("50 US gallons in liters", r#"{"value":"50 galUS","to":"L"}"#));
convert_op!(MASS, "mass", QT::Mass, "lb", "Mass converter",
    "Converts masses: kg, g, lb, oz, and metric tonnes.",
    aliases: ["weight conversion"], refs: [NIST_811, NIST_HB44],
    example: ("2,550 lb in kilograms", r#"{"value":"2550 lb","to":"kg"}"#));
convert_op!(SPEED, "speed", QT::Speed, "kt", "Speed converter",
    "Converts speeds: knots, mph, km/h, m/s, and ft/s.",
    aliases: ["speed conversion", "knots converter"], refs: [NIST_811, ICAO_ANNEX5],
    example: ("100 knots in mph", r#"{"value":"100 kt","to":"mph"}"#));
convert_op!(VERTICAL_SPEED, "vertical-speed", QT::VerticalSpeed, "ft/min", "Vertical speed converter",
    "Converts climb and descent rates: ft/min, m/s, m/min, and ft/s.",
    aliases: ["climb rate conversion", "fpm to m/s"], refs: [NIST_811, ICAO_ANNEX5],
    example: ("500 ft/min in m/s", r#"{"value":"500 ft/min","to":"m/s"}"#));
convert_op!(ACCELERATION, "acceleration", QT::Acceleration, "g0", "Acceleration converter",
    "Converts accelerations: m/s², standard gravity (g), and ft/s².",
    aliases: ["g force conversion"], refs: [NIST_811],
    example: ("2 g in m/s²", r#"{"value":"2 g0","to":"m/s2"}"#));
convert_op!(PRESSURE, "pressure", QT::Pressure, "inHg", "Pressure converter",
    "Converts pressures: inHg, hPa, mbar, Pa, kPa, bar, psi, mmHg, and atm.",
    aliases: ["altimeter setting conversion", "inHg to hPa"], refs: [NIST_811, ICAO_ANNEX5],
    example: ("29.92 inHg in hPa", r#"{"value":"29.92 inHg","to":"hPa"}"#));
convert_op!(TEMPERATURE, "temperature", QT::Temperature, "degC", "Temperature converter",
"Converts temperatures between °C, °F, and K.",
aliases: ["celsius to fahrenheit", "temperature conversion"], refs: [NIST_811],
example: ("30 °C in °F", r#"{"value":"30 °C","to":"degF"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a temperature that is a reading rather than a change: an outside air temperature against a performance chart, a forecast in one scale against a limit in another, a surface temperature against an operating range. It is the tool for a point on the scale, and it applies the offsets those scales are built on, which is exactly what a change in temperature must not have.",
limits: "This converts temperatures, not differences between them, and the two are different conversions: 10 °C is 50 °F, but a rise of 10 °C is a rise of 18 °F. Using this where a deviation is meant -- an ISA deviation, a lapse, a spread -- puts the answer out by the whole offset, 32 degrees between Fahrenheit and Celsius, and the temperature-difference converter next door is the one for that. Below absolute zero is refused rather than converted, since no scale means anything there.",
related: [
    Related { id: "units.temperature-difference.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.pressure.convert", reason: "alternative" }
]);
convert_op!(TEMPERATURE_DIFFERENCE, "temperature-difference", QT::TemperatureDifference, "degC",
"Temperature difference converter",
"Converts temperature differences, like ISA deviation: a change of 1 °C is 1 K and 1.8 °F.",
aliases: ["isa deviation conversion"], refs: [NIST_811],
example: ("An ISA deviation of +18 °F in kelvins", r#"{"value":"18 degF","to":"K"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a change in temperature rather than a temperature: an ISA deviation, a lapse rate's worth of cooling, the spread between dew point and air temperature, a tolerance band. A difference carries only the size of the scale's degree and none of its offset, so a change of one degree Celsius is a change of one kelvin and of 1.8 degrees Fahrenheit -- which is why it is a separate tool and not a setting on the other one.",
limits: "A difference has no zero and no absolute meaning: this will happily convert a negative one, because a temperature can fall, and it cannot tell you whether the number you gave it was a reading by mistake. That is the error worth guarding against, and only you can: put 10 °C through here and it stays 10 K, where as a temperature it would be 283.15 K. Kelvin and Celsius degrees are the same size, so those two directions change nothing at all.",
related: [
    Related { id: "units.temperature.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "aviation.atmosphere.isa", reason: "next" }
]);
convert_op!(ANGLE, "angle", QT::Angle, "deg", "Angle converter",
    "Converts angles: degrees, radians, gons, arcminutes, arcseconds, turns, and the four kinds of mil.",
    aliases: ["degrees to radians", "mils conversion"], refs: [NIST_811],
    example: ("1,600 NATO mils in degrees", r#"{"value":"1600 mil-nato","to":"deg"}"#));
convert_op!(ANGULAR_RATE, "angular-rate", QT::AngularRate, "deg/s", "Angular rate converter",
    "Converts turn and rotation rates: °/s, °/min, rad/s, and rpm.",
    aliases: ["rate of turn conversion"], refs: [NIST_811],
    example: ("A standard-rate turn (3 °/s) in rpm", r#"{"value":"3 deg/s","to":"rpm"}"#));
convert_op!(TIME, "time", QT::Time, "min", "Time converter",
    "Converts durations: seconds, milliseconds, minutes, hours, and days.",
    aliases: ["duration conversion"], refs: [NIST_811],
    example: ("90 minutes in hours", r#"{"value":"90 min","to":"h"}"#));
convert_op!(ENERGY, "energy", QT::Energy, "Wh", "Energy converter",
    "Converts energy: J, kJ, MJ, Wh, and kWh.",
    aliases: ["watt hours conversion"], refs: [NIST_811],
    example: ("99 Wh in kilojoules", r#"{"value":"99 Wh","to":"kJ"}"#));
convert_op!(POWER, "power", QT::Power, "hp", "Power converter",
    "Converts power: W, kW, and mechanical horsepower (550 ft·lbf/s).",
    aliases: ["horsepower to kilowatts"], refs: [NIST_811],
    example: ("180 hp in kilowatts", r#"{"value":"180 hp","to":"kW"}"#));
convert_op!(DENSITY, "density", QT::Density, "lb/galUS", "Density converter",
    "Converts densities: kg/m³, g/cm³, lb/US gal, and lb/ft³.",
    aliases: ["fuel density conversion"], refs: [NIST_811],
    example: ("6.7 lb/gal in kg/L", r#"{"value":"6.7 lb/galUS","to":"g/cm3"}"#));
convert_op!(FREQUENCY, "frequency", QT::Frequency, "MHz", "Frequency converter",
    "Converts frequencies: Hz, kHz, MHz, and GHz.",
    aliases: ["frequency conversion"], refs: [NIST_811],
    example: ("2.4 GHz in MHz", r#"{"value":"2.4 GHz","to":"MHz"}"#));
convert_op!(DATA_RATE, "data-rate", QT::DataRate, "Mbit/s", "Data rate converter",
    "Converts data rates: bit/s, kbit/s, Mbit/s, and Gbit/s.",
    aliases: ["bandwidth conversion"], refs: [NIST_811],
    example: ("20 Mbit/s in kbit/s", r#"{"value":"20 Mbit/s","to":"kbit/s"}"#));

pub static CHARGE: ToolDef = ToolDef {
    id: "units.charge.convert",
    title: "Battery charge converter",
    summary: "Converts battery charge between mAh, Ah, and coulombs, and to watt-hours at a stated voltage.",
    aliases: &["mah to wh", "battery capacity conversion"],
    keywords: &["battery", "mAh", "Wh", "convert"],
    inputs: &[
        Field::new(
            "value",
            "Charge",
            "A charge, like 5000 mAh",
            Kind::Quantity {
                q: QT::ElectricCharge,
                unit: "mAh",
            },
        )
        .required()
        .core(),
        Field::new(
            "from",
            "From unit",
            "The unit of a bare number, like mAh",
            Kind::Unit(QT::ElectricCharge),
        ),
        Field::new(
            "to",
            "To unit",
            "The charge unit to convert to, like Ah",
            Kind::Unit(QT::ElectricCharge),
        )
        .required()
        .core(),
        Field::new(
            "voltage",
            "Voltage",
            "Nominal pack voltage for energy, like 22.2 V",
            Kind::Quantity {
                q: QT::ElectricPotential,
                unit: "V",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "converted",
            "Converted",
            "The charge in the target unit",
            Kind::Quantity {
                q: QT::ElectricCharge,
                unit: "mAh",
            },
        )
        .precision(P8),
        Field::new(
            "input",
            "Input",
            "The charge as read",
            Kind::Quantity {
                q: QT::ElectricCharge,
                unit: "mAh",
            },
        )
        .precision(P8),
        Field::new(
            "energy",
            "Energy",
            "Charge × voltage, when a voltage is given",
            Kind::Quantity {
                q: QT::Energy,
                unit: "Wh",
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
    ],
    warnings: CONVERT_WARNINGS,
    model: "Exact unit definitions; energy = charge × nominal voltage",
    accuracy: "Exact to double precision for charge. Energy is as accurate as the nominal voltage, which varies with state of charge",
    references: &[NIST_811],
    examples: &[Example {
        id: "primary",
        title: "A 5,000 mAh 6S pack (22.2 V) in Ah and Wh",
        input: r#"{"value":"5000 mAh","to":"Ah","voltage":"22.2 V"}"#,
        source: EXACT_SOURCE,
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[Related {
        id: "units.energy.convert",
        reason: "next",
    }],
    sentence: "{input} is {converted}.",
    limits: &[("batchRows", 10_000)],
    run: run_charge,
    ..ToolDef::BLANK
};

fn run_charge(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut out = match run_convert(ctx)? {
        Json::Obj(o) => o,
        _ => unreachable!("run_convert returns an object"),
    };
    if let Some(v) = ctx.quantity("voltage")? {
        let from = ctx.unit("from")?;
        let charge = ctx.quantity_or("value", from)?.expect("required");
        let joules = charge.base() * v.base();
        let j = units::by_symbol(QT::Energy, "J").expect("J");
        out.push((
            "energy".into(),
            ctx.out(
                "energy",
                Q {
                    value: joules,
                    unit: j,
                },
            ),
        ));
    }
    Ok(Json::Obj(out))
}

/// Density units paired with the volume and mass units that make them exact.
const DENSITY_PARTS: &[(&str, &str, &str)] = &[
    ("lb/galUS", "galUS", "lb"),
    ("kg/m3", "m3", "kg"),
    ("g/cm3", "L", "kg"),
    ("lb/ft3", "ft3", "lb"),
];

const FUELS: &[(&str, &str, f64)] = &[
    ("avgas-100ll", "avgas 100LL", 6.0),
    ("jet-a", "Jet-A (at 15 °C)", 6.7),
];

pub static FUEL: ToolDef = ToolDef {
    id: "units.fuel.convert",
    title: "Fuel volume and weight",
    summary: "Converts fuel between volume and weight using a density you choose or a nominal fuel type.",
    aliases: &["fuel weight calculator", "gallons to pounds"],
    keywords: &["fuel", "avgas", "jet-a", "weight", "gallons", "pounds"],
    inputs: &[
        Field::new(
            "volume",
            "Volume",
            "Fuel volume, like 50 galUS",
            Kind::Quantity {
                q: QT::Volume,
                unit: "galUS",
            },
        )
        .core(),
        Field::new(
            "mass",
            "Weight",
            "Fuel weight, like 300 lb",
            Kind::Quantity {
                q: QT::Mass,
                unit: "lb",
            },
        )
        .core(),
        Field::new(
            "fuel",
            "Fuel type",
            "A nominal fuel: avgas-100ll or jet-a",
            Kind::Choice(&["avgas-100ll", "jet-a"]),
        )
        .core(),
        Field::new(
            "density",
            "Density",
            "Measured density, like 6.02 lb/galUS",
            Kind::Quantity {
                q: QT::Density,
                unit: "lb/galUS",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "mass",
            "Weight",
            "Fuel weight",
            Kind::Quantity {
                q: QT::Mass,
                unit: "lb",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "volume",
            "Volume",
            "Fuel volume",
            Kind::Quantity {
                q: QT::Volume,
                unit: "galUS",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "density",
            "Density used",
            "The density applied",
            Kind::Quantity {
                q: QT::Density,
                unit: "lb/galUS",
            },
        )
        .precision(Precision::Significant(3)),
    ],
    warnings: &["NOMINAL_VALUE_USED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "mass = volume × density",
    accuracy: "Exact for a given density. Nominal fuel densities vary about ±3% with temperature and batch",
    references: &[
        NIST_811,
        Reference {
            title: "Pilot's Handbook of Aeronautical Knowledge, FAA-H-8083-25C",
            issuer: "Federal Aviation Administration",
            year: 2023,
            edition: "FAA-H-8083-25C",
            locator: "Chapter 10, Weight and Balance (avgas 6 lb/gal, jet fuel 6.7 lb/gal)",
            url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak",
        },
    ],
    examples: &[Example {
        id: "primary",
        title: "50 US gallons of avgas in pounds",
        input: r#"{"volume":"50 galUS","fuel":"avgas-100ll"}"#,
        source: "FAA-H-8083-25C Chapter 10 (6 lb/gal)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[Related {
        id: "units.density.convert",
        reason: "alternative",
    }],
    sentence: "{volume} of fuel weighs {mass} at {density}.",
    limits: &[("batchRows", 10_000)],
    run: run_fuel,
    ..ToolDef::BLANK
};

fn run_fuel(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let volume = ctx.quantity("volume")?;
    let mass = ctx.quantity("mass")?;
    let fuel = ctx.choice("fuel")?;
    let density = ctx.quantity("density")?;
    let density = match (density, fuel) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/density",
                "Give a fuel type or a density, not both.",
            ));
        }
        (Some(d), None) => d,
        (None, Some(f)) => {
            let (_, name, lb_per_gal) = FUELS
                .iter()
                .find(|(id, _, _)| *id == f)
                .expect("choice is validated");
            ctx.warnings.push(
                Warning::new(
                    "NOMINAL_VALUE_USED",
                    format!(
                        "{} lb/US gal is a nominal planning density for {name}. Use a measured density for weight and balance.",
                        gp_base::num::format_f64(*lb_per_gal).unwrap_or_default()
                    ),
                )
                .at("/fuel"),
            );
            Q {
                value: *lb_per_gal,
                unit: units::by_symbol(QT::Density, "lb/galUS").expect("unit"),
            }
        }
        (None, None) => {
            return Err(ToolError::invalid("/density", "A fuel density is needed to convert volume and weight.")
                .hint("Choose a fuel type (avgas-100ll at 6.0 lb/gal or jet-a at 6.7 lb/gal, nominal planning values) or enter a measured density."));
        }
    };
    if density.value <= 0.0 {
        return Err(ToolError::invalid(
            "/density",
            "Density must be greater than zero.",
        ));
    }
    // Work in the density's own volume and mass units so 50 gal × 6 lb/gal is exactly 300 lb.
    let (_, vol_sym, mass_sym) = DENSITY_PARTS
        .iter()
        .find(|(d, _, _)| *d == density.unit.symbol)
        .expect("every density unit has parts");
    let vol_unit = units::by_symbol(QT::Volume, vol_sym).expect("unit");
    let mass_unit = units::by_symbol(QT::Mass, mass_sym).expect("unit");
    let (v, m) = match (volume, mass) {
        (Some(v), None) => {
            let vv = v.to(vol_unit);
            (
                Q {
                    value: vv,
                    unit: vol_unit,
                },
                Q {
                    value: vv * density.value,
                    unit: mass_unit,
                },
            )
        }
        (None, Some(m)) => {
            let mm = m.to(mass_unit);
            (
                Q {
                    value: mm / density.value,
                    unit: vol_unit,
                },
                Q {
                    value: mm,
                    unit: mass_unit,
                },
            )
        }
        _ => {
            return Err(ToolError::invalid(
                "/volume",
                "Give exactly one of volume or weight.",
            ));
        }
    };
    if v.value < 0.0 {
        return Err(ToolError::invalid(
            if volume.is_some() { "/volume" } else { "/mass" },
            "Fuel cannot be negative.",
        ));
    }
    Ok(Json::obj([
        ("mass", ctx.out("mass", m)),
        ("volume", ctx.out("volume", v)),
        ("density", ctx.out("density", density)),
    ]))
}

const SLOPE_FROM: &[&str] = &["ratio", "percent", "permille", "degrees"];

pub static SLOPE: ToolDef = ToolDef {
    id: "units.slope.convert",
    title: "Slope and grade converter",
    summary: "Converts a slope between rise/run ratio, percent grade, per mille, and degrees.",
    aliases: &["grade calculator", "percent grade to degrees"],
    keywords: &[
        "slope", "grade", "gradient", "incline", "ramp", "percent", "degrees",
    ],
    inputs: &[
        Field::new(
            "value",
            "Slope",
            "The slope, like 8.33",
            Kind::Number {
                min: -1e9,
                max: 1e9,
            },
        )
        .required()
        .core(),
        Field::new(
            "from",
            "Given as",
            "ratio, percent, permille, or degrees",
            Kind::Choice(SLOPE_FROM),
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "degrees",
            "Angle",
            "The angle above horizontal",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("unbounded"),
        Field::new(
            "ratio",
            "Rise/run",
            "Rise over run",
            Kind::Quantity {
                q: QT::Slope,
                unit: "ratio",
            },
        )
        .precision(Precision::Significant(4)),
        Field::new(
            "percent",
            "Percent grade",
            "Rise over run × 100",
            Kind::Quantity {
                q: QT::Slope,
                unit: "%",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "permille",
            "Per mille",
            "Rise over run × 1000",
            Kind::Quantity {
                q: QT::Slope,
                unit: "‰",
            },
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "ratio = tan(angle); percent = 100 × ratio",
    accuracy: "Exact for ratio, percent, and per mille; degrees to double precision (libm tan/atan)",
    references: &[NIST_811],
    examples: &[Example {
        id: "primary",
        title: "A 1:12 wheelchair ramp (about 8.33%) in degrees",
        input: r#"{"value":8.33,"from":"percent"}"#,
        source: "Definition of grade: angle = atan(0.0833) = 4.76°; a 1:12 ramp is 8.33%",
    }],
    primary_example: "primary",
    visualization: TABLE,
    sentence: "A grade of {percent} is a slope of {degrees}.",
    limits: &[("batchRows", 10_000)],
    run: run_slope,
    ..ToolDef::BLANK
};

fn run_slope(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = ctx.number("value")?.expect("required");
    let from = ctx.choice("from")?.expect("required");
    let slope_unit = |s| units::by_symbol(QT::Slope, s).expect("slope unit");
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let (grade, angle) = match from {
        "degrees" => {
            if !(v > -90.0 && v < 90.0) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "A slope angle must be between -90° and 90°; 90° is vertical.",
                )
                .at("/value"));
            }
            let ratio = libm::tan(v.to_radians());
            (
                Q {
                    value: ratio,
                    unit: slope_unit("ratio"),
                },
                v,
            )
        }
        other => {
            let sym = match other {
                "ratio" => "ratio",
                "percent" => "%",
                _ => "‰",
            };
            let q = Q {
                value: v,
                unit: slope_unit(sym),
            };
            (q, libm::atan(q.base()).to_degrees())
        }
    };
    Ok(Json::obj([
        ("ratio", ctx.emit("ratio", grade, slope_unit("ratio"))),
        ("percent", ctx.emit("percent", grade, slope_unit("%"))),
        ("permille", ctx.emit("permille", grade, slope_unit("‰"))),
        (
            "degrees",
            ctx.emit(
                "degrees",
                Q {
                    value: angle,
                    unit: deg,
                },
                deg,
            ),
        ),
    ]))
}

const QUANTITY_IDS: &[&str] = &[
    "length",
    "distance",
    "area",
    "volume",
    "mass",
    "speed",
    "vertical-speed",
    "acceleration",
    "pressure",
    "temperature",
    "temperature-difference",
    "angle",
    "angular-rate",
    "time",
    "energy",
    "power",
    "electric-charge",
    "electric-potential",
    "density",
    "frequency",
    "data-rate",
    "slope",
];

pub static NORMALIZE: ToolDef = ToolDef {
    id: "units.quantity.normalize",
    title: "Normalize a unit-tagged value",
    summary: "Reads any value with a unit, like \"145 kts\", and returns it in canonical units (SI; degrees for angles).",
    aliases: &["unit parser", "to si units"],
    keywords: &["normalize", "si", "canonical", "parse unit"],
    inputs: &[
        Field::new(
            "value",
            "Value with unit",
            "A number and a unit, like 145 kts",
            Kind::AnyQuantity,
        )
        .required()
        .core(),
        Field::new(
            "quantity",
            "Quantity",
            "What the value measures, like speed",
            Kind::Choice(QUANTITY_IDS),
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "normalized",
            "Canonical value",
            "The value in the canonical unit",
            Kind::AnyQuantity,
        )
        .precision(P8),
        Field::new("input", "Input", "The value as read", Kind::AnyQuantity).precision(P8),
    ],
    warnings: CONVERT_WARNINGS,
    model: "Exact unit definitions",
    accuracy: "Exact to double precision",
    references: &[NIST_811],
    examples: &[Example {
        id: "primary",
        title: "145 kts in m/s",
        input: r#"{"value":"145 kts","quantity":"speed"}"#,
        source: EXACT_SOURCE,
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[Related {
        id: "units.speed.convert",
        reason: "alternative",
    }],
    sentence: "{input} is {normalized} in canonical units.",
    limits: &[("batchRows", 10_000)],
    run: run_normalize,
    ..ToolDef::BLANK
};

fn run_normalize(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let q = Quantity::from_id(ctx.choice("quantity")?.expect("required"))
        .expect("choice lists quantities");
    let Some(serde_json::Value::String(text)) = ctx.raw("value").cloned() else {
        return Err(ToolError::invalid(
            "/value",
            "value must be a string with a unit, like \"145 kts\".",
        ));
    };
    let base = units::base_unit(q);
    let (_, unit_text) = parse::split_number_unit(text.trim(), ctx.options.format);
    if unit_text.is_empty() {
        return Err(
            ToolError::new(ErrorCode::UnitMismatch, format!("\"{text}\" has no unit."))
                .at("/value")
                .hint(format!("Add a unit, like \"{text} {}\".", base.symbol)),
        );
    }
    let t = parse::parse_tagged(&text, q, base, ctx.options.format, "/value")?;
    ctx.warnings.extend(t.warnings);
    let v = Q {
        value: t.value,
        unit: t.unit,
    };
    let normalized = ctx.emit("normalized", v, base);
    Ok(Json::obj([
        ("input", v.to_json()),
        ("normalized", normalized),
    ]))
}

/// Every tool in the base module, operations first.
pub static TOOLS: &[&ToolDef] = &[
    &LENGTH,
    &AREA,
    &VOLUME,
    &MASS,
    &SPEED,
    &VERTICAL_SPEED,
    &ACCELERATION,
    &PRESSURE,
    &TEMPERATURE,
    &TEMPERATURE_DIFFERENCE,
    &ANGLE,
    &ANGULAR_RATE,
    &TIME,
    &ENERGY,
    &POWER,
    &CHARGE,
    &FUEL,
    &DENSITY,
    &FREQUENCY,
    &DATA_RATE,
    &SLOPE,
    &NORMALIZE,
    &pairs::KT_TO_MPH,
    &pairs::MPH_TO_KT,
    &pairs::KT_TO_KMH,
    &pairs::KMH_TO_KT,
    &pairs::MPS_TO_KT,
    &pairs::KT_TO_MPS,
    &pairs::FT_TO_M,
    &pairs::M_TO_FT,
    &pairs::NM_TO_KM,
    &pairs::KM_TO_NM,
    &pairs::NM_TO_MI,
    &pairs::MI_TO_NM,
    &pairs::MI_TO_KM,
    &pairs::KM_TO_MI,
    &pairs::FTUS_TO_M,
    &pairs::M_TO_FTUS,
    &pairs::FTUS_TO_FT,
    &pairs::IN_TO_MM,
    &pairs::MM_TO_IN,
    &pairs::INHG_TO_HPA,
    &pairs::HPA_TO_INHG,
    &pairs::PSI_TO_KPA,
    &pairs::KPA_TO_PSI,
    &pairs::C_TO_F,
    &pairs::F_TO_C,
    &pairs::GAL_TO_L,
    &pairs::L_TO_GAL,
    &pairs::LB_TO_KG,
    &pairs::KG_TO_LB,
    &pairs::AC_TO_HA,
    &pairs::HA_TO_AC,
    &pairs::FT2_TO_AC,
    &pairs::FPM_TO_MPS,
    &pairs::DEG_TO_RAD,
    &pairs::RAD_TO_DEG,
];

pub static REGISTRY: Registry = Registry {
    module: "base",
    tools: TOOLS,
};

gp_base::export_module!("base", REGISTRY);
