//! The catalog ranker (build-web-experience W5, add-seo-and-discoverability S4).
//! One deterministic ranker, compiled into its own lazily loaded `search`
//! module, so the command palette and `geoprims_search` rank identically.
//!
//! Scoring is integer-only: weighted fields (id, title, aliases, keywords,
//! group, summary), exact, stem, prefix, and one-edit typo matches, a coverage
//! factor for multi-word queries, and boosts for exact id, alias, and title
//! matches. Ties break by id.

mod detect;

use std::cell::RefCell;
use std::collections::HashMap;

use gp_base::envelope;
use gp_base::error::{ErrorCode, ToolError};
use gp_base::json::Json;
use gp_base::tool::Registry;
use serde_json::Value;

pub static REGISTRY: Registry = Registry {
    module: "search",
    tools: &[],
};

gp_base::export_module!("search", REGISTRY);

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub domain: String,
    pub stability: String,
    /// Weighted fields as vocabulary token ids.
    fields: Vec<(u32, Vec<u32>)>,
    /// Alias and keyword phrases, tokenized.
    phrases: Vec<Vec<u32>>,
    title_tokens: Vec<u32>,
}

const W_ID: u32 = 3;
const W_TITLE: u32 = 4;
const W_ALIAS: u32 = 5;
const W_KEYWORD: u32 = 3;
const W_GROUP: u32 = 2;
const W_SUMMARY: u32 = 1;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "of", "for", "and", "in", "on", "at", "is", "what", "how", "my", "me", "i",
    "with", "by",
];

/// Words kept for phrase matching ("knots to mph") that never make a match alone.
const CONNECTORS: &[&str] = &["to", "from"];

/// Lowercase alphanumeric tokens.
pub fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn stem(t: &str) -> &str {
    for (suffix, min) in [("ing", 6), ("ies", 5), ("es", 5), ("ed", 5), ("s", 4)] {
        if t.len() >= min
            && let Some(s) = t.strip_suffix(suffix)
        {
            return s;
        }
    }
    t
}

/// True when `a` and `b` differ by exactly one insertion, deletion, or substitution
/// (or an adjacent transposition).
fn one_edit(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a == b || a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let (ra, rb) = (&a[pre..], &b[pre..]);
    let suf = ra
        .iter()
        .rev()
        .zip(rb.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&ra[..ra.len() - suf], &rb[..rb.len() - suf]);
    (ma.len() <= 1 && mb.len() <= 1)
        || (ma.len() == 2 && mb.len() == 2 && ma[0] == mb[1] && ma[1] == mb[0])
}

/// Match quality of query token `q` against field token `t`, out of 100.
fn match_score(q: &str, t: &str) -> u32 {
    if q == t {
        100
    } else if stem(q) == stem(t) {
        90
    } else if q.len() >= 2 && t.starts_with(q) {
        70
    } else if q.chars().count() >= 4 && one_edit(q, t) {
        60
    } else if q.chars().count() >= 4
        && t.chars().count() > q.chars().count()
        && one_edit(q, &t.chars().take(q.chars().count()).collect::<String>())
    {
        50
    } else {
        0
    }
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Interns tokens so each distinct word is matched against a query once per
/// search, not once per entry (the palette's 16 ms budget at 1,000 entries).
#[derive(Default)]
struct Vocab {
    words: Vec<String>,
    ids: HashMap<String, u32>,
}

impl Vocab {
    fn intern(&mut self, words: Vec<String>) -> Vec<u32> {
        words
            .into_iter()
            .map(|w| {
                if let Some(&i) = self.ids.get(&w) {
                    return i;
                }
                let i = self.words.len() as u32;
                self.words.push(w.clone());
                self.ids.insert(w, i);
                i
            })
            .collect()
    }
}

impl Entry {
    /// Builds an entry from a catalog manifest (or any object with the same keys).
    fn from_manifest(m: &Value, vocab: &mut Vocab) -> Option<Entry> {
        let s = |k: &str| m[k].as_str().unwrap_or_default().to_owned();
        let id = s("id");
        if id.is_empty() {
            return None;
        }
        let aliases = strs(&m["aliases"]);
        let keywords = strs(&m["keywords"]);
        let mut t = |words: Vec<String>| vocab.intern(words);
        let fields = vec![
            (W_ID, t(tokens(&id))),
            (W_TITLE, t(tokens(&s("title")))),
            (W_ALIAS, t(aliases.iter().flat_map(|a| tokens(a)).collect())),
            (
                W_KEYWORD,
                t(keywords.iter().flat_map(|k| tokens(k)).collect()),
            ),
            (W_GROUP, t(tokens(&s("group")))),
            (W_SUMMARY, t(tokens(&s("summary")))),
        ];
        Some(Entry {
            title_tokens: t(tokens(&s("title"))),
            phrases: aliases
                .iter()
                .chain(&keywords)
                .map(|a| t(tokens(a)))
                .collect(),
            id,
            title: s("title"),
            summary: s("summary"),
            domain: s("domain"),
            stability: s("stability"),
            fields,
        })
    }

    /// The entry's score for a query whose tokens were matched against the
    /// vocabulary: `table[i][v]` is `match_score` of query token `i` and word
    /// `v`. `connector[i]` marks words that never make a match alone. 0 means no match.
    fn score(&self, table: &[Vec<u8>], connector: &[bool], raw_id: &str) -> u32 {
        if raw_id == self.id {
            return 1_000_000;
        }
        let mut total = 0;
        let mut matched = 0;
        for (row, &conn) in table.iter().zip(connector) {
            let best = self
                .fields
                .iter()
                .flat_map(|(w, ts)| ts.iter().map(move |&t| w * row[t as usize] as u32))
                .max()
                .unwrap_or(0);
            if best > 0 && !conn {
                matched += 1;
            }
            total += best;
        }
        if matched == 0 {
            return 0;
        }
        let mut score = total * matched / table.len() as u32;
        score += self
            .phrases
            .iter()
            .map(|p| phrase_boost(table, p, 1000))
            .max()
            .unwrap_or(0);
        score += phrase_boost(table, &self.title_tokens, 800);
        score
    }
}

/// Boost when the query matches a phrase word for word, in order: `full` for an
/// exact match, 60% of it when every word matches with typos or stems, and a
/// quarter of it when the query is a word-for-word prefix of the phrase.
fn phrase_boost(table: &[Vec<u8>], phrase: &[u32], full: u32) -> u32 {
    if table.is_empty() || table.len() > phrase.len() {
        return 0;
    }
    // match_score is 100 only for identical words.
    let exact = table
        .iter()
        .zip(phrase)
        .all(|(row, &b)| row[b as usize] == 100);
    let fuzzy = table
        .iter()
        .zip(phrase)
        .all(|(row, &b)| row[b as usize] >= 50);
    match (table.len() == phrase.len(), exact, fuzzy) {
        (true, true, _) => full,
        (true, false, true) => full * 6 / 10,
        (false, true, _) => full / 4,
        _ => 0,
    }
}

thread_local! {
    static INDEX: RefCell<(Vocab, Vec<Entry>)> = RefCell::new((Vocab::default(), Vec::new()));
}

/// Loads the index from a JSON array of manifests (or `{tools: [...]}`).
pub fn load(json: &str) -> String {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return envelope::failure(&ToolError::new(
                ErrorCode::InvalidInput,
                format!("The index is not JSON: {e}."),
            ));
        }
    };
    let list = if v.is_array() { &v } else { &v["tools"] };
    let mut vocab = Vocab::default();
    let entries: Vec<Entry> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| Entry::from_manifest(m, &mut vocab))
        .collect();
    let n = entries.len();
    INDEX.with(|i| *i.borrow_mut() = (vocab, entries));
    Json::obj([("ok", Json::Bool(true)), ("entries", Json::Num(n as f64))])
        .to_string()
        .expect("finite")
}

pub const DEFAULT_LIMIT: usize = 10;
pub const MAX_LIMIT: usize = 50;

/// Searches the loaded index. Input: `{query, domain?, limit?, includeExperimental?}`.
pub fn search(json: &str) -> String {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return envelope::failure(&ToolError::new(
                ErrorCode::InvalidInput,
                format!("The request is not JSON: {e}."),
            ));
        }
    };
    let Some(query) = v["query"].as_str().filter(|q| !q.trim().is_empty()) else {
        return envelope::failure(&ToolError::invalid(
            "/query",
            "query must be a non-empty string.",
        ));
    };
    let limit = match &v["limit"] {
        Value::Null => DEFAULT_LIMIT,
        l => match l.as_u64().filter(|n| (1..=MAX_LIMIT as u64).contains(n)) {
            Some(n) => n as usize,
            None => {
                return envelope::failure(&ToolError::invalid(
                    "/limit",
                    format!("limit must be an integer from 1 to {MAX_LIMIT}."),
                ));
            }
        },
    };
    let domain = v["domain"].as_str();
    let include_experimental = v["includeExperimental"].as_bool().unwrap_or(false);
    let q: Vec<String> = tokens(query)
        .into_iter()
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .collect();
    let q = if q.is_empty() { tokens(query) } else { q };
    let raw_id = query.trim().to_lowercase();
    INDEX.with(|index| {
        let index = index.borrow();
        let (vocab, entries) = &*index;
        let table: Vec<Vec<u8>> = q
            .iter()
            .map(|qt| {
                vocab
                    .words
                    .iter()
                    .map(|w| match_score(qt, w) as u8)
                    .collect()
            })
            .collect();
        let connector: Vec<bool> = q.iter().map(|t| CONNECTORS.contains(&t.as_str())).collect();
        let mut hits: Vec<(u32, &Entry)> = entries
            .iter()
            .filter(|e| domain.is_none_or(|d| e.domain == d))
            .map(|e| (e.score(&table, &connector, &raw_id), e))
            .filter(|(s, _)| *s > 0)
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
        let hidden = hits
            .iter()
            .filter(|(_, e)| !include_experimental && e.stability == "experimental")
            .count();
        let results: Vec<Json> = hits
            .iter()
            .filter(|(_, e)| include_experimental || e.stability != "experimental")
            .take(limit)
            .map(|(_, e)| {
                Json::obj([
                    ("id", Json::str(&e.id)),
                    ("title", Json::str(&e.title)),
                    ("summary", Json::str(&e.summary)),
                    ("domain", Json::str(&e.domain)),
                    ("stability", Json::str(&e.stability)),
                ])
            })
            .collect();
        let mut out = vec![("results".to_owned(), Json::Arr(results))];
        if hidden > 0 {
            out.push(("hiddenExperimental".to_owned(), Json::Num(hidden as f64)));
        }
        Json::obj([("ok", Json::Bool(true)), ("result", Json::Obj(out))])
            .to_string()
            .expect("finite")
    })
}

/// Paste-to-detect. Input: `{query}`; output: `{ok, result: {candidates}}`.
pub fn detect(json: &str) -> String {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return envelope::failure(&ToolError::new(
                ErrorCode::InvalidInput,
                format!("The request is not JSON: {e}."),
            ));
        }
    };
    let query = v["query"].as_str().unwrap_or_default();
    Json::obj([
        ("ok", Json::Bool(true)),
        (
            "result",
            Json::obj([("candidates", detect::candidates(query))]),
        ),
    ])
    .to_string()
    .expect("finite")
}

#[cfg(target_arch = "wasm32")]
mod exports {
    /// # Safety
    /// The range must be a readable UTF-8 buffer from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_search_load(ptr: *const u8, len: usize) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(ptr, len) } {
            Ok(s) => super::load(s),
            Err(_) => gp_base::abi::bad_utf8(),
        };
        gp_base::abi::set_out(&out)
    }

    /// # Safety
    /// The range must be a readable UTF-8 buffer from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_search(ptr: *const u8, len: usize) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(ptr, len) } {
            Ok(s) => super::search(s),
            Err(_) => gp_base::abi::bad_utf8(),
        };
        gp_base::abi::set_out(&out)
    }

    /// # Safety
    /// The range must be a readable UTF-8 buffer from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_detect(ptr: *const u8, len: usize) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(ptr, len) } {
            Ok(s) => super::detect(s),
            Err(_) => gp_base::abi::bad_utf8(),
        };
        gp_base::abi::set_out(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_one() {
        assert!(one_edit("densty", "density"));
        assert!(one_edit("densit", "density"));
        assert!(one_edit("dnesity", "density"));
        assert!(one_edit("denxity", "density"));
        assert!(!one_edit("dens", "density"));
        assert!(!one_edit("density", "density"));
    }

    #[test]
    fn matching_levels() {
        assert_eq!(match_score("alt", "altitude"), 70);
        assert_eq!(match_score("knots", "knot"), 90);
        assert_eq!(match_score("densty", "density"), 60);
        assert_eq!(match_score("x", "xyz"), 0);
    }

    fn index() {
        let manifests = serde_json::json!([
            {"id":"aviation.altimetry.density-altitude","title":"Density altitude","summary":"Density altitude from elevation, altimeter, and temperature.","domain":"aviation","group":"altimetry","aliases":["DA"],"keywords":["performance"],"stability":"stable"},
            {"id":"aviation.altimetry.pressure-altitude","title":"Pressure altitude","summary":"Pressure altitude from field elevation and altimeter setting.","domain":"aviation","group":"altimetry","aliases":["PA"],"keywords":[],"stability":"stable"},
            {"id":"aviation.airspeed.cas-to-tas","title":"CAS to TAS","summary":"True airspeed from calibrated airspeed.","domain":"aviation","group":"airspeed","aliases":["TAS","true airspeed"],"keywords":[],"stability":"stable"},
            {"id":"aviation.wind.correction-angle","title":"Wind correction angle","summary":"Heading correction for crosswind.","domain":"aviation","group":"wind","aliases":["WCA","E6B","wind triangle"],"keywords":[],"stability":"stable"},
            {"id":"units.speed.convert","title":"Speed converter","summary":"Converts speeds.","domain":"units","group":"speed","aliases":["knots converter"],"keywords":["convert"],"stability":"experimental"}
        ]);
        assert_eq!(load(&manifests.to_string()), r#"{"ok":true,"entries":5}"#);
    }

    fn top(query: &str) -> Vec<String> {
        let out: Value =
            serde_json::from_str(&search(&serde_json::json!({"query": query}).to_string()))
                .unwrap();
        out["result"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn palette_scenarios() {
        index();
        assert_eq!(top("densty alt")[0], "aviation.altimetry.density-altitude");
        assert_eq!(top("tas")[0], "aviation.airspeed.cas-to-tas");
        assert_eq!(top("wca")[0], "aviation.wind.correction-angle");
        assert_eq!(top("e6b")[0], "aviation.wind.correction-angle");
        assert_eq!(
            top("aviation.altimetry.pressure-altitude")[0],
            "aviation.altimetry.pressure-altitude"
        );
    }

    #[test]
    fn experimental_hidden_unless_requested() {
        index();
        let out: Value = serde_json::from_str(&search(r#"{"query":"knots converter"}"#)).unwrap();
        assert_eq!(out["result"]["results"].as_array().unwrap().len(), 0);
        assert_eq!(out["result"]["hiddenExperimental"], 1);
        let out: Value = serde_json::from_str(&search(
            r#"{"query":"knots converter","includeExperimental":true}"#,
        ))
        .unwrap();
        assert_eq!(out["result"]["results"][0]["id"], "units.speed.convert");
    }

    #[test]
    fn bad_requests() {
        index();
        for (req, field) in [
            (r#"{"query":""}"#, "/query"),
            (r#"{"query":"x","limit":0}"#, "/limit"),
            (r#"{"query":"x","limit":51}"#, "/limit"),
        ] {
            let out: Value = serde_json::from_str(&search(req)).unwrap();
            assert_eq!(out["error"]["field"], field, "{req}");
        }
    }

    #[test]
    fn deterministic_ties_by_id() {
        index();
        assert_eq!(
            top("altitude"),
            vec![
                "aviation.altimetry.density-altitude",
                "aviation.altimetry.pressure-altitude"
            ]
        );
    }
}
