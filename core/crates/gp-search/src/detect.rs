//! Paste-to-detect (build-web-experience W5 "Paste-to-detect"): recognizes what
//! a pasted value could be, by syntax only, and names the tool that decodes it
//! and the tools worth opening with it. Hosts run each decoder to confirm the
//! interpretation (a candidate whose decoder fails is dropped), so this module
//! needs no geodesy or indexing code. Every plausible reading is returned, in a
//! fixed order: geohash first, as the spec's ambiguity scenario requires.

use gp_base::json::Json;

/// A tool to offer: its id, fixed inputs, and inputs copied from the decoder's
/// result (`field`, `result path`).
struct Offer {
    id: &'static str,
    input: fn(&str) -> Vec<(&'static str, Json)>,
    from: &'static [(&'static str, &'static str)],
}

struct Kind {
    kind: &'static str,
    label: &'static str,
    matches: fn(&str) -> bool,
    decoder: &'static str,
    /// The decoder's input for the value.
    decode: fn(&str) -> Vec<(&'static str, Json)>,
    offers: &'static [Offer],
}

const LATLON: &[(&str, &str)] = &[("lat", "result.lat.value"), ("lon", "result.lon.value")];
const GEOHASH_ALPHABET: &str = "0123456789bcdefghjkmnpqrstuvwxyz";

fn none(_: &str) -> Vec<(&'static str, Json)> {
    Vec::new()
}

/// Tools that take a point, filled from the decoded latitude and longitude.
const AT_POINT: &[Offer] = &[
    Offer {
        id: "geodesy.utm.forward",
        input: none,
        from: LATLON,
    },
    Offer {
        id: "geodesy.magnetic.declination",
        input: none,
        from: LATLON,
    },
    Offer {
        id: "geodesy.geoid.geoid-height",
        input: none,
        from: LATLON,
    },
    Offer {
        id: "time.sun.position",
        input: none,
        from: LATLON,
    },
    Offer {
        id: "indexing.h3.lat-lng-to-cell",
        input: none,
        from: LATLON,
    },
];

fn is_h3(s: &str) -> bool {
    s.len() == 15
        && s.bytes().all(|b| b.is_ascii_hexdigit())
        && s.starts_with('8')
        && u8::from_str_radix(&s[1..2], 16).is_ok_and(|r| r <= 15)
}

/// Short all-letter words ("utm", "tas") are valid geohashes too, so a
/// geohash must contain a digit unless it is at least 7 characters long.
fn is_geohash(s: &str) -> bool {
    (2..=12).contains(&s.len())
        && s.chars()
            .all(|c| GEOHASH_ALPHABET.contains(c.to_ascii_lowercase()))
        && (s.len() >= 7 || s.bytes().any(|b| b.is_ascii_digit()))
}

fn is_quadkey(s: &str) -> bool {
    (1..=23).contains(&s.len()) && s.bytes().all(|b| (b'0'..=b'3').contains(&b))
}

fn is_xyz(s: &str) -> bool {
    let parts: Vec<&str> = s.split('/').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 8 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_plus_code(s: &str) -> bool {
    let alphabet = "23456789CFGHJMPQRVWX";
    s.contains('+')
        && (4..=16).contains(&s.len())
        && s.chars()
            .all(|c| c == '+' || c == '0' || alphabet.contains(c.to_ascii_uppercase()))
}

fn is_altimeter(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && matches!(b[0], b'A' | b'Q' | b'a' | b'q')
        && b[1..].iter().all(u8::is_ascii_digit)
}

fn is_metar(s: &str) -> bool {
    // Reports are written in capitals, but a person types "metar kden …".
    let upper = s.to_ascii_uppercase();
    let words: Vec<&str> = upper.split_whitespace().collect();
    let rest = match words.first() {
        Some(&"METAR" | &"SPECI") => &words[1..],
        _ => &words[..],
    };
    rest.len() >= 3
        && rest[0].len() == 4
        && rest[0].bytes().all(|b| b.is_ascii_uppercase())
        && rest[1].len() == 7
        && rest[1].ends_with('Z')
        && rest[1][..6].bytes().all(|b| b.is_ascii_digit())
}

/// Two or more numbers, or a grid reference: worth asking the coordinate parser.
fn is_coordinate_like(s: &str) -> bool {
    // Numbers bounded by separators or hemisphere letters ("N40.4 W79.9"), so
    // digits inside codes like "9q8yy" or "A2992" do not count.
    let b = s.as_bytes();
    let bound = |i: Option<usize>| {
        i.and_then(|i| b.get(i))
            .is_none_or(|c| !c.is_ascii_alphanumeric() || b"NSEWnsew".contains(c))
    };
    let mut numbers = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if bound(start.checked_sub(1)) && bound(Some(i)) {
                numbers += 1;
            }
        } else {
            i += 1;
        }
    }
    // An MGRS reference: a one- or two-digit zone, a latitude band, and two
    // 100 km square letters ("18T WL 80669 23543").
    let packed: Vec<u8> = s
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let lead = packed.iter().take_while(|b| b.is_ascii_digit()).count();
    let grid = (1..=2).contains(&lead)
        && packed.len() >= lead + 3
        && b"CDEFGHJKLMNPQRSTUVWX".contains(&packed[lead])
        && b"ABCDEFGHJKLMNPQRSTUVWXYZ".contains(&packed[lead + 1])
        && b"ABCDEFGHJKLMNPQRSTUV".contains(&packed[lead + 2]);
    (numbers >= 2 || grid) && s.len() <= 80
}

const KINDS: &[Kind] = &[
    Kind {
        kind: "geohash",
        label: "Geohash",
        matches: is_geohash,
        decoder: "indexing.geohash.decode",
        decode: geohash,
        offers: &[
            Offer {
                id: "indexing.geohash.decode",
                input: geohash,
                from: &[],
            },
            Offer {
                id: "indexing.geohash.neighbors",
                input: geohash,
                from: &[],
            },
        ],
    },
    Kind {
        kind: "h3",
        label: "H3 cell",
        matches: is_h3,
        decoder: "indexing.h3.cell-info",
        decode: cell,
        offers: &[
            Offer {
                id: "indexing.h3.cell-info",
                input: cell,
                from: &[],
            },
            Offer {
                id: "indexing.h3.grid-disk",
                input: cell,
                from: &[],
            },
            Offer {
                id: "indexing.h3.parent",
                input: cell,
                from: &[],
            },
            Offer {
                id: "indexing.h3.children",
                input: cell,
                from: &[],
            },
        ],
    },
    Kind {
        kind: "quadkey",
        label: "Map tile quadkey",
        matches: is_quadkey,
        decoder: "indexing.tile.bounds",
        decode: tile,
        offers: &[Offer {
            id: "indexing.tile.bounds",
            input: tile,
            from: &[],
        }],
    },
    Kind {
        kind: "xyz",
        label: "Map tile z/x/y",
        matches: is_xyz,
        decoder: "indexing.tile.bounds",
        decode: tile,
        offers: &[Offer {
            id: "indexing.tile.bounds",
            input: tile,
            from: &[],
        }],
    },
    Kind {
        kind: "plus-code",
        label: "Plus Code",
        matches: is_plus_code,
        decoder: "indexing.plus-code.decode",
        decode: code,
        offers: &[Offer {
            id: "indexing.plus-code.decode",
            input: code,
            from: &[],
        }],
    },
    Kind {
        kind: "altimeter",
        label: "Altimeter setting",
        matches: is_altimeter,
        decoder: "units.pressure.convert",
        decode: altimeter_setting,
        offers: &[
            Offer {
                id: "aviation.altimetry.pressure-altitude",
                input: altimeter,
                from: &[],
            },
            Offer {
                id: "aviation.altimetry.density-altitude",
                input: altimeter,
                from: &[],
            },
        ],
    },
    Kind {
        kind: "metar",
        label: "METAR report",
        matches: is_metar,
        decoder: "aviation.weather.metar-decode",
        decode: report,
        offers: &[Offer {
            id: "aviation.weather.metar-decode",
            input: report,
            from: &[],
        }],
    },
    Kind {
        kind: "coordinates",
        label: "Coordinates",
        matches: is_coordinate_like,
        decoder: "geodesy.parse.coordinates",
        decode: text,
        offers: AT_POINT,
    },
];

fn geohash(v: &str) -> Vec<(&'static str, Json)> {
    vec![("geohash", Json::str(v))]
}
fn cell(v: &str) -> Vec<(&'static str, Json)> {
    vec![("cell", Json::str(v.to_ascii_lowercase()))]
}
fn tile(v: &str) -> Vec<(&'static str, Json)> {
    vec![("tile", Json::str(v))]
}
fn code(v: &str) -> Vec<(&'static str, Json)> {
    vec![("code", Json::str(v.to_ascii_uppercase()))]
}
fn altimeter(v: &str) -> Vec<(&'static str, Json)> {
    vec![("altimeter", Json::str(v.to_ascii_uppercase()))]
}
fn text(v: &str) -> Vec<(&'static str, Json)> {
    vec![("text", Json::str(v))]
}
/// A METAR altimeter group as a pressure: A2992 is 29.92 inHg, Q1013 is 1013 hPa.
fn altimeter_setting(v: &str) -> Vec<(&'static str, Json)> {
    let (unit, value, to) = if v.starts_with(['A', 'a']) {
        ("inHg", format!("{}.{}", &v[1..3], &v[3..5]), "hPa")
    } else {
        ("hPa", v[1..].to_owned(), "inHg")
    };
    vec![
        ("value", Json::str(format!("{value} {unit}"))),
        ("to", Json::str(to)),
    ]
}
fn report(v: &str) -> Vec<(&'static str, Json)> {
    vec![("report", Json::str(v.to_ascii_uppercase()))]
}

fn obj(pairs: Vec<(&'static str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

/// Formats structured enough that recognizing one decides the tool: a METAR,
/// an H3 cell, a Plus Code, an altimeter group. Looser ones (a geohash is any
/// short word in its alphabet, so "denver" qualifies) only ever suggest.
const DECISIVE: &[&str] = &["metar", "h3", "plus-code", "altimeter"];

/// The decoder and its input when the whole query is a decisive format: what
/// the search should open, with the value already in it.
pub fn decisive(query: &str) -> Option<(&'static str, Vec<(&'static str, Json)>)> {
    let v = query.trim();
    let k = KINDS
        .iter()
        .find(|k| DECISIVE.contains(&k.kind) && (k.matches)(v))?;
    let value = if k.kind == "h3" {
        v.to_ascii_lowercase()
    } else {
        v.to_owned()
    };
    Some((k.decoder, (k.decode)(&value)))
}

/// Candidates for a pasted value: `{kind, label, value, decoder: {id, input}, offers: [{id, input, from}]}`.
pub fn candidates(query: &str) -> Json {
    let v = query.trim();
    let mut out = Vec::new();
    if v.is_empty() {
        return Json::Arr(out);
    }
    for k in KINDS {
        if !(k.matches)(v) {
            continue;
        }
        let value = if k.kind == "h3" {
            v.to_ascii_lowercase()
        } else {
            v.to_owned()
        };
        let offers = k
            .offers
            .iter()
            .map(|o| {
                Json::obj([
                    ("id", Json::str(o.id)),
                    ("input", obj((o.input)(v))),
                    (
                        "from",
                        Json::Obj(
                            o.from
                                .iter()
                                .map(|(f, p)| ((*f).to_owned(), Json::str(*p)))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect();
        out.push(Json::obj([
            ("kind", Json::str(k.kind)),
            ("label", Json::str(k.label)),
            ("value", Json::str(&value)),
            (
                "decoder",
                Json::obj([
                    ("id", Json::str(k.decoder)),
                    ("input", obj((k.decode)(&value))),
                ]),
            ),
            ("offers", Json::Arr(offers)),
        ]));
    }
    Json::Arr(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(q: &str) -> Vec<String> {
        let Json::Arr(list) = candidates(q) else {
            unreachable!()
        };
        list.iter()
            .map(|c| match c {
                Json::Obj(p) => match &p.iter().find(|(k, _)| k == "kind").unwrap().1 {
                    Json::Str(s) => s.clone(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
            .collect()
    }

    #[test]
    fn recognizes_each_kind() {
        assert_eq!(kinds("8928308280fffff"), ["h3"]);
        assert_eq!(kinds("9q8yy"), ["geohash"]);
        assert_eq!(kinds("0231"), ["geohash", "quadkey"]);
        assert_eq!(kinds("12/1137/2551"), ["xyz", "coordinates"]);
        assert_eq!(kinds("849VCWC8+R9"), ["plus-code"]);
        assert_eq!(kinds("A2992"), ["altimeter"]);
        assert_eq!(
            kinds("KDEN 181753Z 23012KT 10SM FEW120 28/03 A3012"),
            ["metar", "coordinates"] // the host drops readings its decoder rejects
        );
        assert_eq!(kinds("40.4461, -79.9822"), ["coordinates"]);
        assert_eq!(kinds("18T WL 80669 23543"), ["coordinates"]);
        assert!(kinds("density altitude").is_empty());
        assert!(kinds("utm").is_empty() && kinds("gps").is_empty());
        assert!(kinds("").is_empty());
    }
}
