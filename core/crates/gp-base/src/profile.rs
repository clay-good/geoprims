//! Output unit profiles (units-and-quantities spec, "Output unit selection").

use crate::units::{self, Quantity, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Si,
    /// ft, kt, NM, °C, and inHg (or hPa with `aviation-hpa`).
    Aviation {
        hpa: bool,
    },
    UsCustomary,
    SurveyMetric,
    /// Legacy US survey feet; every ftUS output carries `LEGACY_UNIT`.
    SurveyUs,
}

impl Profile {
    pub const IDS: [&'static str; 6] = [
        "si",
        "aviation",
        "aviation-hpa",
        "us-customary",
        "survey-metric",
        "survey-us",
    ];

    pub fn from_id(id: &str) -> Option<Profile> {
        Some(match id {
            "si" => Profile::Si,
            "aviation" => Profile::Aviation { hpa: false },
            "aviation-hpa" => Profile::Aviation { hpa: true },
            "us-customary" => Profile::UsCustomary,
            "survey-metric" => Profile::SurveyMetric,
            "survey-us" => Profile::SurveyUs,
            _ => return None,
        })
    }

    /// The unit this profile uses for quantity `q`. Quantities a profile does not
    /// name use the registry base unit.
    pub fn unit_for(self, q: Quantity) -> &'static Unit {
        use Quantity as Q;
        let symbol = match (self, q) {
            (Profile::Si, _) => None,
            (Profile::Aviation { .. }, Q::Length) => Some("ft"),
            (Profile::Aviation { .. }, Q::Distance) => Some("NM"),
            (Profile::Aviation { .. }, Q::Speed) => Some("kt"),
            (Profile::Aviation { .. }, Q::VerticalSpeed) => Some("ft/min"),
            (Profile::Aviation { hpa: false }, Q::Pressure) => Some("inHg"),
            (Profile::Aviation { hpa: true }, Q::Pressure) => Some("hPa"),
            (Profile::Aviation { .. }, Q::Temperature | Q::TemperatureDifference) => Some("degC"),
            (Profile::Aviation { .. }, Q::Mass) => Some("lb"),
            (Profile::Aviation { .. }, Q::Volume) => Some("galUS"),
            (Profile::Aviation { .. }, Q::VolumeFlow) => Some("galUS/h"),
            (Profile::Aviation { .. }, Q::Density) => Some("lb/galUS"),
            (Profile::UsCustomary, Q::Length) => Some("ft"),
            (Profile::UsCustomary, Q::Distance) => Some("mi"),
            (Profile::UsCustomary, Q::Area) => Some("ac"),
            (Profile::UsCustomary, Q::Volume) => Some("galUS"),
            (Profile::UsCustomary, Q::VolumeFlow) => Some("galUS/h"),
            (Profile::UsCustomary, Q::Mass) => Some("lb"),
            (Profile::UsCustomary, Q::Speed) => Some("mph"),
            (Profile::UsCustomary, Q::VerticalSpeed) => Some("ft/min"),
            (Profile::UsCustomary, Q::Acceleration) => Some("ft/s2"),
            (Profile::UsCustomary, Q::Pressure) => Some("psi"),
            (Profile::UsCustomary, Q::Temperature | Q::TemperatureDifference) => Some("degF"),
            (Profile::UsCustomary, Q::Density) => Some("lb/ft3"),
            (Profile::UsCustomary, Q::Power) => Some("hp"),
            (Profile::SurveyMetric, Q::Temperature | Q::TemperatureDifference) => Some("degC"),
            (Profile::SurveyMetric, Q::Pressure) => Some("hPa"),
            (Profile::SurveyMetric, Q::Area) => Some("ha"),
            (Profile::SurveyUs, Q::Length | Q::Distance) => Some("ftUS"),
            (Profile::SurveyUs, Q::Area) => Some("ac"),
            (Profile::SurveyUs, Q::Volume) => Some("yd3"),
            (Profile::SurveyUs, Q::Temperature | Q::TemperatureDifference) => Some("degF"),
            (Profile::SurveyUs, Q::Pressure) => Some("inHg"),
            _ => None,
        };
        match symbol {
            Some(s) => units::by_symbol(q, s).expect("profile unit is registered"),
            None => units::base_unit(q),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviation_profile() {
        let p = Profile::from_id("aviation").unwrap();
        assert_eq!(p.unit_for(Quantity::Distance).symbol, "NM");
        assert_eq!(p.unit_for(Quantity::Length).symbol, "ft");
        assert_eq!(p.unit_for(Quantity::Speed).symbol, "kt");
        assert_eq!(p.unit_for(Quantity::Pressure).symbol, "inHg");
        assert_eq!(p.unit_for(Quantity::Temperature).symbol, "degC");
        assert_eq!(
            Profile::from_id("aviation-hpa")
                .unwrap()
                .unit_for(Quantity::Pressure)
                .symbol,
            "hPa"
        );
    }

    #[test]
    fn every_profile_covers_every_quantity() {
        for id in Profile::IDS {
            let p = Profile::from_id(id).unwrap();
            for q in Quantity::ALL {
                assert_eq!(p.unit_for(q).quantity, q.dimension(), "{id} {}", q.id());
            }
        }
        assert_eq!(Profile::Si.unit_for(Quantity::Angle).symbol, "deg");
        assert!(Profile::from_id("metric").is_none());
    }
}
