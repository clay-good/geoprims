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
example: ("One acre in square feet", r#"{"value":"1 ac","to":"ft2"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for any area that crosses systems: a parcel in acres against a plan in hectares, a footprint in square feet against square metres, a survey block in square nautical miles. Both acres are here and kept apart, which matters for land records: the international acre and the US survey acre differ by four parts per million, about 16 square centimetres on an acre and a square metre on a section.",
limits: "An area converts exactly; a measured area does not become more accurate for being converted. The US survey foot and its acre were withdrawn for new work at the end of 2022 and are kept for reading existing records, which is why a value using them is flagged. A square nautical mile is the square of the international nautical mile and is not an official unit of anything -- it appears because airspace and search areas are quoted in it. Nothing here computes the area of a shape; that is the geometry tools.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.length.convert", reason: "parent" },
    Related { id: "geometry.area.polygon", reason: "next" }
]);
convert_op!(VOLUME, "volume", QT::Volume, "galUS", "Volume converter",
"Converts volumes: m³, liters, US and imperial gallons, ft³, and yd³.",
aliases: ["volume conversion"], refs: [NIST_811, NIST_HB44],
example: ("50 US gallons in liters", r#"{"value":"50 galUS","to":"L"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for volumes that cross systems: fuel in US gallons against litres, a tank in imperial gallons, earthworks in cubic yards against cubic metres, a container in cubic feet. Fuel is the common case and the two gallons are the common trap, so both are here under names that cannot be confused for each other.",
limits: "The US and imperial gallons are different units and neither is called just gallon here: an imperial gallon is 4.54609 L and a US gallon is 3.785411784 L, so reading one as the other is a 20% error -- enough to matter on any tank. Volumes convert exactly, but a volume of fuel is not a mass of fuel; that needs a density, which is the fuel converter next door. Nothing here accounts for temperature, and fuel volume changes with it.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.fuel.convert", reason: "next" },
    Related { id: "units.density.convert", reason: "alternative" }
]);
convert_op!(MASS, "mass", QT::Mass, "lb", "Mass converter",
"Converts masses: kg, g, lb, oz, and metric tonnes.",
aliases: ["weight conversion"], refs: [NIST_811, NIST_HB44],
example: ("2,550 lb in kilograms", r#"{"value":"2550 lb","to":"kg"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for any weight that has to move between systems: an aircraft or vehicle weight in pounds against a limit in kilograms, a payload in ounces, a load in tonnes. Weight and balance, freight manifests and equipment lists are quoted in whichever system the document came from, and the limit is usually in the other one.",
limits: "These are masses, not forces. A pound here is the international avoirdupois pound of mass, not the pound-force, and the two are the same number only where gravity is standard -- which is why the pressure converter builds psi from this pound times standard gravity rather than treating them as interchangeable. The tonne is the metric tonne of 1,000 kg, not the short ton of 2,000 lb nor the long ton of 2,240 lb, neither of which is offered, so a document saying only ton has to be read before it is converted.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.density.convert", reason: "next" },
    Related { id: "units.fuel.convert", reason: "next" }
]);
convert_op!(SPEED, "speed", QT::Speed, "kt", "Speed converter",
"Converts speeds: knots, mph, km/h, m/s, and ft/s.",
aliases: ["speed conversion", "knots converter"], refs: [NIST_811, ICAO_ANNEX5],
example: ("100 knots in mph", r#"{"value":"100 kt","to":"mph"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this wherever a speed crosses between trades: an airspeed or a boat speed in knots against a groundspeed in km/h, a road limit in mph, a wind in m/s, a descent in ft/s. The geodetic rates are here too -- mm/yr and m/yr -- because plate motion and subsidence are quoted in them and belong on the same scale as everything else.",
limits: "A speed is a rate and nothing more: this cannot tell airspeed from groundspeed, true from indicated, or a speed through the water from a speed over it. Those distinctions are what the navigation and aviation tools carry. The knot here is the international one, 1,852 m per hour exactly; an old chart or a British source may mean the Admiralty knot, which is about 0.06% larger and will not announce itself.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.length.convert", reason: "alternative" },
    Related { id: "units.vertical-speed.convert", reason: "alternative" }
]);
convert_op!(VERTICAL_SPEED, "vertical-speed", QT::VerticalSpeed, "ft/min", "Vertical speed converter",
"Converts climb and descent rates: ft/min, m/s, m/min, and ft/s.",
aliases: ["climb rate conversion", "fpm to m/s"], refs: [NIST_811, ICAO_ANNEX5],
example: ("500 ft/min in m/s", r#"{"value":"500 ft/min","to":"m/s"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a rate of climb or descent that has to cross systems: an instrument or a clearance in feet per minute against a performance figure or a glider variometer in metres per second. Keep the sign -- a descent is a negative climb -- and the direction comes through the conversion with it.",
limits: "This is a vertical rate, not a horizontal one, and knots are deliberately not offered: a climb rate in knots is almost always a ground speed misread. Nothing here knows about air or ground: an indicated rate of climb is what the instrument reads in the air mass it is in, and a rate through a descending air mass is not a rate over the ground. Converting a rate does not convert a gradient either -- a feet-per-nautical-mile figure needs a speed as well, which the slope converter is for.",
related: [
    Related { id: "units.speed.convert", reason: "alternative" },
    Related { id: "units.slope.convert", reason: "next" },
    Related { id: "units.quantity.normalize", reason: "alternative" }
]);
convert_op!(ACCELERATION, "acceleration", QT::Acceleration, "g0", "Acceleration converter",
"Converts accelerations: m/s², standard gravity (g), and ft/s².",
aliases: ["g force conversion"], refs: [NIST_811],
example: ("2 g in m/s²", r#"{"value":"2 g0","to":"m/s2"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a load factor, a braking or launch figure, or a vibration limit quoted in one system and needed in another: a 2 g manoeuvring limit in m/s², a sensor reading in ft/s² against a specification in g.",
limits: "The g here is standard gravity, the defined constant of 9.80665 m/s². It is not the local acceleration of gravity, which runs from about 9.78 at the equator to 9.83 at the poles and falls with height -- a few parts in a thousand, which matters for gravimetry and does not for a load factor. A load factor is also not the same thing as an acceleration in a straight line: it is the ratio of lift to weight, and in a steady turn the aircraft is not speeding up at all.",
related: [
    Related { id: "units.speed.convert", reason: "alternative" },
    Related { id: "units.vertical-speed.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" }
]);
convert_op!(PRESSURE, "pressure", QT::Pressure, "inHg", "Pressure converter",
"Converts pressures: inHg, hPa, mbar, Pa, kPa, bar, psi, mmHg, and atm.",
aliases: ["altimeter setting conversion", "inHg to hPa"], refs: [NIST_811, ICAO_ANNEX5],
example: ("29.92 inHg in hPa", r#"{"value":"29.92 inHg","to":"hPa"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this wherever a pressure has to cross between the systems that report it: an altimeter setting in inches of mercury against a QNH in hectopascals, a tyre or hydraulic pressure in psi against a gauge in bar, a blood-pressure or manometer reading in millimetres of mercury against pascals. Aviation runs on inHg in North America and hPa almost everywhere else, and the two appear side by side on the same flight.",
limits: "This converts the size of a pressure and not what it is measured against: it cannot tell absolute pressure from gauge pressure, and a psi that meant psig is still 14.7 psi away from what it looks like here. The mercury units are the conventional ones -- inHg is exactly 3386.389 Pa and mmHg exactly 133.322387415 Pa by definition, not mercury weighed at a particular temperature -- so a laboratory manometer corrected for its own mercury will differ in the seventh digit. Altitude, standard atmospheres and pressure altitude are the atmosphere tools, not this.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.density.convert", reason: "alternative" },
    Related { id: "aviation.atmosphere.isa", reason: "next" }
]);
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
example: ("1,600 NATO mils in degrees", r#"{"value":"1600 mil-nato","to":"deg"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this wherever an angle crosses between trades: a bearing in degrees against a calculation in radians, a survey angle in gons, a sight in mils, a catalogue position in milliarcseconds, a slope in milliradians. The three military mils are all here and kept apart, which is the point — they divide the circle 6,400, 6,000 and 6,300 ways, so the same number of mils is a different angle depending on whose manual it came from.",
limits: "This converts the size of an angle and knows nothing about what it points at: it will not wrap a bearing into 0-360, decide whether an angle is measured from north or from east, or tell a heading from a course. Those are the coordinate and bearing tools. Radians and milliradians involve pi, which is not a ratio of whole numbers, so those conversions carry a slightly looser tolerance than the rest -- still far below any angle anyone measures. A mil given without saying which mil is the one error this tool cannot catch for you.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.angular-rate.convert", reason: "next" },
    Related { id: "geodesy.parse.angle-arithmetic", reason: "alternative" }
]);
convert_op!(ANGULAR_RATE, "angular-rate", QT::AngularRate, "deg/s", "Angular rate converter",
"Converts turn and rotation rates: °/s, °/min, rad/s, rpm, and arcsec or mas per year.",
aliases: ["rate of turn conversion"], refs: [NIST_811],
example: ("A standard-rate turn (3 °/s) in rpm", r#"{"value":"3 deg/s","to":"rpm"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a rate of turn or rotation: a standard-rate turn in degrees per second against a gyro output in rad/s, a shaft or rotor speed in rpm, or a plate-motion and station-velocity rotation published in milliarcseconds per year.",
limits: "The year in arcsec/yr and mas/yr is the Julian year of exactly 365.25 days, which is what published velocities are quoted against; a calendar year would move the answer by up to a fifth of a percent. This converts a rate, not a turn: how fast something rotates, not how far it has rotated, which is the angle converter. And a rate of turn is not a radius -- an aircraft turning at 3 °/s sweeps a different circle at every speed, so a turn radius needs the speed as well.",
related: [
    Related { id: "units.angle.convert", reason: "next" },
    Related { id: "units.time.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" }
]);
convert_op!(TIME, "time", QT::Time, "min", "Time converter",
"Converts durations: seconds, milliseconds, minutes, hours, and days.",
aliases: ["duration conversion"], refs: [NIST_811],
example: ("90 minutes in hours", r#"{"value":"90 min","to":"h"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a duration: a flight time, an endurance, a logged interval, a timeout. Give it a span in one unit and get the same span in another, from milliseconds to days, which is what a fuel calculation or schedule arithmetic needs.",
limits: "This converts durations, not times of day and not dates. A day here is exactly 86,400 seconds, which is what a duration of one day means; it is not the length of a particular calendar day, and a day containing a leap second or a change of clocks is longer or shorter than this by a second or an hour. Anything that has to know what happened on a given date -- a time zone, a UTC offset, a leap second -- belongs to the time domain and not here.",
related: [
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.speed.convert", reason: "alternative" },
    Related { id: "time.scale.utc-offset", reason: "next" }
]);
convert_op!(ENERGY, "energy", QT::Energy, "Wh", "Energy converter",
"Converts energy: J, kJ, MJ, Wh, kWh, and foot pounds-force.",
aliases: ["watt hours conversion"], refs: [NIST_811],
example: ("99 Wh in kilojoules", r#"{"value":"99 Wh","to":"kJ"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this when a capacity, a consumption or a quantity of work is quoted in one system and needed in another: a battery in watt hours against a limit in joules, a daily use in kilowatt hours against a supply in megajoules, a torque-wrench or fastener figure in foot pounds-force. Specification sheets and regulations each pick their own unit and rarely the same one.",
limits: "This converts energy, not power: a kilowatt hour is an amount of energy and a kilowatt is a rate, and the two are a quantity and its per-hour, so the power converter next door is the one for a rate. A foot pound-force of energy is numerically the same as a pound-foot of torque and is not the same thing -- torque is a moment, energy is work, and nothing here will stop you converting one as the other. Battery capacities quoted in mAh are a charge, not an energy, and need a voltage before they can come here; the charge converter handles those.",
related: [
    Related { id: "units.power.convert", reason: "next" },
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.charge.convert", reason: "alternative" }
]);
convert_op!(POWER, "power", QT::Power, "hp", "Power converter",
"Converts power: W, kW, and mechanical horsepower (550 ft·lbf/s).",
aliases: ["horsepower to kilowatts"], refs: [NIST_811],
example: ("180 hp in kilowatts", r#"{"value":"180 hp","to":"kW"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for any rating that has to cross systems: an engine or motor in horsepower against a limit in kilowatts, a generator or heater in watts, a pump curve quoted either way. Vehicle, aircraft and equipment ratings are published in whichever unit the market expects and compared against a specification in the other.",
limits: "The horsepower here is the mechanical one of 550 ft·lbf/s, which is what a US or UK rating means. The metric horsepower (PS or ch) is 735.49875 W, 1.4% smaller, and the boiler horsepower is about thirteen times larger; neither is offered, because a document that says only horsepower has to be read before it is converted rather than guessed at here. Power is a rate, not an amount: a kilowatt for an hour is a kilowatt hour, and the energy converter is the one for that."
,
related: [
    Related { id: "units.energy.convert", reason: "next" },
    Related { id: "units.quantity.normalize", reason: "alternative" },
    Related { id: "units.time.convert", reason: "alternative" }
]);
convert_op!(DENSITY, "density", QT::Density, "lb/galUS", "Density converter",
"Converts densities: kg/m³, g/cm³, lb/US gal, and lb/ft³.",
aliases: ["fuel density conversion"], refs: [NIST_811],
example: ("6.7 lb/gal in kg/L", r#"{"value":"6.7 lb/galUS","to":"g/cm3"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this when a density is quoted in one system and needed in another: a fuel at 6.7 lb per US gallon against a figure in kg/L, a material at 62.4 lb per cubic foot against one in kg/m³. It is the step before the fuel converter, which needs a density to turn a volume into a weight.",
limits: "The gallon here is the US liquid gallon. A pound per imperial gallon is 20% smaller and is not offered, so a figure that says only lb/gal has to be read before it is converted. A density is a property of a substance at a temperature, and nothing here knows either one: fuel expands as it warms, by roughly a tenth of a percent per degree Celsius for Jet A, so a density measured at one temperature is not the density at another. And g/cm³ and kg/L are the same unit under two names, not two units that happen to agree.",
related: [
    Related { id: "units.fuel.convert", reason: "next" },
    Related { id: "units.mass.convert", reason: "alternative" },
    Related { id: "units.volume.convert", reason: "alternative" }
]);
convert_op!(FREQUENCY, "frequency", QT::Frequency, "MHz", "Frequency converter",
"Converts frequencies: Hz, kHz, MHz, GHz, and drift rates in ppm or ppb per year.",
aliases: ["frequency conversion"], refs: [NIST_811],
example: ("2.4 GHz in MHz", r#"{"value":"2.4 GHz","to":"MHz"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a radio or clock frequency that has to move between prefixes -- a 2.4 GHz link in MHz, an emergency frequency of 121.5 MHz in kHz, a 433 MHz beacon in Hz -- or for an oscillator drift rate quoted in parts per million or billion per year.",
limits: "The prefixes are decimal: a megahertz is a million hertz, not 2^20. That is the SI meaning and the one radio has always used, but software that quotes storage in binary multiples can make it look otherwise, and the two differ by 4.9% at mega. The year behind ppm/yr and ppb/yr is the Julian year of exactly 365.25 days. A frequency is not a wavelength either: turning one into the other needs a propagation speed, which is not an input here.",
related: [
    Related { id: "units.time.convert", reason: "alternative" },
    Related { id: "units.data-rate.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" }
]);
convert_op!(DATA_RATE, "data-rate", QT::DataRate, "Mbit/s", "Data rate converter",
"Converts data rates: bit/s, kbit/s, Mbit/s, and Gbit/s.",
aliases: ["bandwidth conversion"], refs: [NIST_811],
example: ("20 Mbit/s in kbit/s", r#"{"value":"20 Mbit/s","to":"kbit/s"}"#),
stability: Stability::Stable, warnings: CONVERT_STABLE,
when: "Use this for a link or telemetry rate quoted in one prefix and needed in another: a 20 Mbit/s downlink in kbit/s, a gigabit backhaul in megabits, a modem or radio rate in bits per second. Datasheets, contracts and monitoring tools each pick their own prefix, and the figure being compared against is rarely in the same one.",
limits: "These are bits, not bytes, and nothing here divides by eight: a rate in bytes per second is a different quantity and has to be converted before it comes here. The prefixes are decimal, so a megabit per second is 10^6 bit/s and not 2^20 -- the reading networks have always used, and the one IEC 80000-13 gives, with Ki, Mi and Gi reserved for the binary multiples; the two differ by 4.9% at mega and 7.4% at giga. A link rate is also not a throughput: framing, retransmission and protocol overhead all sit between them.",
related: [
    Related { id: "units.frequency.convert", reason: "alternative" },
    Related { id: "units.time.convert", reason: "alternative" },
    Related { id: "units.quantity.normalize", reason: "alternative" }
]);

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
    stability: Stability::Stable,
    when_to_use: "Use this for a battery capacity that has to cross units, and for the watt-hour figure a shipping rule or a spec sheet asks for: a 5,000 mAh pack in ampere hours, or in watt hours once you give it the pack's nominal voltage.",
    limitations: "A charge is not an energy. The watt-hour figure is charge times the nominal voltage you supply, which is a convention for the pack rather than a measurement -- the real voltage falls as the pack discharges, so the delivered energy is typically a few percent either side and is reported to four significant figures for that reason. Give no voltage and no energy is reported, which is the honest answer rather than a guess. Nothing here knows about chemistry, C-rate, temperature or state of health, all of which change what a pack actually delivers.",
    warnings: CONVERT_STABLE,
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
    related: &[
        Related {
            id: "units.energy.convert",
            reason: "next",
        },
        Related {
            id: "units.power.convert",
            reason: "alternative",
        },
        Related {
            id: "units.quantity.normalize",
            reason: "alternative",
        },
    ],
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
