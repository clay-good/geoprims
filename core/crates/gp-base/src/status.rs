//! Status phrases (`ux/glanceable-results`, "Status phrases without false
//! assurance"). A tool that judges a result against a threshold says one of
//! three things, and a tool that applies a published standard's own
//! conformance test says one of two. Nothing else: never "safe", "unsafe",
//! "legal", or "approved", because none of those follow from the arithmetic.

/// Words a status phrase may never use: they promise more than a calculation
/// can tell the reader.
pub const BANNED: &[&str] = &[
    "safe",
    "unsafe",
    "legal",
    "illegal",
    "approved",
    "unapproved",
];

/// The default share of the limit that reads as "Near" (`x-near-margin`).
pub const NEAR_MARGIN: f64 = 0.10;

/// The banned word `text` uses, if any. Longer words that merely contain one
/// ("safety", "legality") are not matches.
pub fn banned_word(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    BANNED.iter().copied().find(|w| {
        lower
            .match_indices(w)
            .any(|(i, _)| whole_word(&lower, i, w.len()))
    })
}

fn whole_word(text: &str, i: usize, len: usize) -> bool {
    let before = text[..i]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric());
    let after = text[i + len..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric());
    before && after
}

/// "Within / Near / Beyond your <limit>". `label` names the limit as the
/// reader set it ("15 kt crosswind limit"); `near` is the share of the limit
/// that reads as "Near".
pub fn threshold(value: f64, limit: f64, near: f64, label: &str) -> String {
    if value > limit {
        format!("Beyond your {label}")
    } else if value >= limit * (1.0 - near) {
        format!("Near your {label}")
    } else {
        format!("Within your {label}")
    }
}

/// "Meets / Does not meet <class>", for a standard that defines its own
/// conformance test.
pub fn conformance(meets: bool, class: &str) -> String {
    if meets {
        format!("Meets {class}")
    } else {
        format!("Does not meet {class}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: &str = "15 kt crosswind limit";

    #[test]
    fn three_phrases_around_a_limit() {
        assert_eq!(
            threshold(18.0, 15.0, NEAR_MARGIN, LIMIT),
            "Beyond your 15 kt crosswind limit"
        );
        assert_eq!(
            threshold(15.0, 15.0, NEAR_MARGIN, LIMIT),
            "Near your 15 kt crosswind limit"
        );
        assert_eq!(
            threshold(13.5, 15.0, NEAR_MARGIN, LIMIT),
            "Near your 15 kt crosswind limit"
        );
        assert_eq!(
            threshold(13.4, 15.0, NEAR_MARGIN, LIMIT),
            "Within your 15 kt crosswind limit"
        );
    }

    #[test]
    fn conformance_phrases() {
        assert_eq!(conformance(true, "ASPRS Class I"), "Meets ASPRS Class I");
        assert_eq!(
            conformance(false, "ASPRS Class I"),
            "Does not meet ASPRS Class I"
        );
    }

    #[test]
    fn banned_words_are_caught_but_longer_words_are_not() {
        assert_eq!(banned_word("Within your safe limit"), Some("safe"));
        assert_eq!(banned_word("This is not legal advice"), Some("legal"));
        assert_eq!(banned_word("Beyond your 15 kt crosswind limit"), None);
        assert_eq!(banned_word("Within the safety margin"), None);
        assert_eq!(banned_word("Meets the legality review"), None);
    }

    #[test]
    fn every_phrase_this_module_makes_is_clean() {
        for text in [
            threshold(1.0, 10.0, NEAR_MARGIN, "limit"),
            threshold(10.0, 10.0, NEAR_MARGIN, "limit"),
            threshold(99.0, 10.0, NEAR_MARGIN, "limit"),
            conformance(true, "Class I"),
            conformance(false, "Class I"),
        ] {
            assert_eq!(banned_word(&text), None, "{text}");
        }
    }
}
