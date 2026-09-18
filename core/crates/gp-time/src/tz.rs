//! IANA time zones from an embedded tzdb snapshot (time-scales spec,
//! "Explicit time zones, deterministic data"): TZif v2+ (RFC 8536) with the
//! POSIX TZ footer for dates past the last listed transition. Never the
//! host's time-zone data.

use crate::civil::{days_from_civil, is_leap_year};

static BLOB: &[u8] = include_bytes!("../data/tzdb.bin");

/// The tzdb release, like "2026d".
pub fn version() -> &'static str {
    let n = usize::from(BLOB[4]);
    core::str::from_utf8(&BLOB[5..5 + n]).expect("ascii version")
}

fn u16le(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}

/// Zone names (canonical and links), sorted.
pub fn names() -> impl Iterator<Item = &'static str> {
    let (zones, _, mut at) = header();
    (0..zones).map(move |_| {
        let n = usize::from(BLOB[at]);
        let name = core::str::from_utf8(&BLOB[at + 1..at + 1 + n]).expect("ascii name");
        at += n + 3;
        name
    })
}

fn header() -> (usize, usize, usize) {
    let n = usize::from(BLOB[4]);
    let at = 5 + n;
    (u16le(BLOB, at), u16le(BLOB, at + 2), at + 4)
}

/// Looks up a zone by name (exact, then case-insensitive).
pub fn zone(name: &str) -> Option<Zone> {
    let (zones, blobs, mut at) = header();
    let mut found = None;
    for _ in 0..zones {
        let n = usize::from(BLOB[at]);
        let z = &BLOB[at + 1..at + 1 + n];
        let idx = u16le(BLOB, at + 1 + n);
        if z == name.as_bytes() {
            found = Some((idx, z));
            break;
        }
        if found.is_none() && z.eq_ignore_ascii_case(name.as_bytes()) {
            found = Some((idx, z));
        }
        at += n + 3;
    }
    let (idx, zname) = found?;
    let (_, _, mut p) = header();
    for _ in 0..zones {
        p += usize::from(BLOB[p]) + 3;
    }
    for i in 0..blobs {
        let len = u32::from_le_bytes([BLOB[p], BLOB[p + 1], BLOB[p + 2], BLOB[p + 3]]) as usize;
        if i == idx {
            let name = core::str::from_utf8(zname).expect("ascii");
            return Zone::parse(name, &BLOB[p + 4..p + 4 + len]);
        }
        p += 4 + len;
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalType {
    /// Seconds east of UTC.
    pub utoff: i32,
    pub dst: bool,
    pub abbr: &'static str,
}

/// A POSIX TZ rule date: Jn (1–365, no Feb 29), n (0–365), or Mm.w.d.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RuleDate {
    Julian1(u16),
    Julian0(u16),
    Month(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rule {
    std: LocalType,
    dst: Option<(LocalType, RuleDate, i32, RuleDate, i32)>,
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub name: &'static str,
    times: Vec<i64>,
    idx: Vec<u8>,
    types: Vec<LocalType>,
    footer: Option<Rule>,
}

fn be32(b: &[u8], at: usize) -> i64 {
    i64::from(i32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]))
}

impl Zone {
    fn parse(name: &'static str, b: &'static [u8]) -> Option<Zone> {
        if &b[..4] != b"TZif" {
            return None;
        }
        let counts = |at: usize| -> [usize; 6] {
            core::array::from_fn(|i| be32(b, at + 20 + 4 * i) as usize)
        };
        let [isut, isstd, leap, timecnt, typecnt, charcnt] = counts(0);
        // Skip the v1 block to the v2+ header.
        let v2 = 44 + timecnt * 5 + typecnt * 6 + charcnt + leap * 8 + isstd + isut;
        let [isut, isstd, leap, timecnt, typecnt, charcnt] = counts(v2);
        let mut p = v2 + 44;
        let times: Vec<i64> = (0..timecnt)
            .map(|i| i64::from_be_bytes(b[p + 8 * i..p + 8 * i + 8].try_into().expect("8")))
            .collect();
        p += timecnt * 8;
        let idx = b[p..p + timecnt].to_vec();
        p += timecnt;
        let chars_at = p + typecnt * 6;
        let abbr = |i: usize| {
            let s = &b[chars_at + i..chars_at + charcnt];
            let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
            core::str::from_utf8(&s[..end]).unwrap_or("")
        };
        let types = (0..typecnt)
            .map(|i| {
                let q = p + 6 * i;
                LocalType {
                    utoff: be32(b, q) as i32,
                    dst: b[q + 4] != 0,
                    abbr: abbr(usize::from(b[q + 5])),
                }
            })
            .collect();
        p = chars_at + charcnt + leap * 12 + isstd + isut;
        let footer = b.get(p + 1..).and_then(|f| {
            let end = f.iter().position(|&c| c == b'\n')?;
            parse_posix(core::str::from_utf8(&f[..end]).ok()?)
        });
        Some(Zone {
            name,
            times,
            idx,
            types,
            footer,
        })
    }

    /// The local time type in effect at a UTC instant (Unix seconds).
    pub fn at(&self, t: i64) -> LocalType {
        match self.times.last() {
            Some(&last) if t >= last && self.footer.is_some() => {
                rule_at(self.footer.as_ref().expect("footer"), t)
            }
            None if self.footer.is_some() => rule_at(self.footer.as_ref().expect("footer"), t),
            _ => {
                let i = self.times.partition_point(|&x| x <= t);
                if i == 0 {
                    self.types[0]
                } else {
                    self.types[usize::from(self.idx[i - 1])]
                }
            }
        }
    }

    /// UTC instants whose local time is `local` (Unix seconds of the local
    /// wall clock): one normally, two in a fall-back overlap (earlier
    /// first), none in a spring-forward gap.
    pub fn from_local(&self, local: i64) -> Vec<i64> {
        let mut offs = [self.at(local - 86_400).utoff, self.at(local + 86_400).utoff];
        offs.sort_unstable_by(|a, b| b.cmp(a));
        let mut out: Vec<i64> = Vec::new();
        for o in offs {
            let t = local - i64::from(o);
            if self.at(t).utoff == o && !out.contains(&t) {
                out.push(t);
            }
        }
        out.sort_unstable();
        out
    }

    /// The first change of offset or DST after `t`, within ten years.
    pub fn next_change(&self, t: i64) -> Option<i64> {
        let now = self.at(t);
        let i = self.times.partition_point(|&x| x <= t);
        for &x in &self.times[i..] {
            if self.at(x) != now {
                return Some(x);
            }
        }
        let rule = self.footer.as_ref()?;
        let (y, _, _) = crate::civil::civil_from_days(t.div_euclid(86_400));
        let mut edges: Vec<i64> = (y..=y + 10)
            .flat_map(|yy| rule_edges(rule, yy))
            .filter(|&e| e > t)
            .collect();
        edges.sort_unstable();
        edges.into_iter().find(|&e| self.at(e) != now)
    }
}

// ---------------------------------------------------------------- POSIX TZ

fn parse_posix(s: &'static str) -> Option<Rule> {
    let mut p = Posix { s, i: 0 };
    let std_name = p.name()?;
    let std_off = -p.offset()?;
    let std = LocalType {
        utoff: std_off,
        dst: false,
        abbr: std_name,
    };
    if p.done() {
        return Some(Rule { std, dst: None });
    }
    let dst_name = p.name()?;
    let dst_off = if p.peek() == Some(b',') || p.done() {
        std_off + 3600
    } else {
        -p.offset()?
    };
    let dst = LocalType {
        utoff: dst_off,
        dst: true,
        abbr: dst_name,
    };
    if !p.eat(b',') {
        return None;
    }
    let (d1, t1) = p.date_time()?;
    if !p.eat(b',') {
        return None;
    }
    let (d2, t2) = p.date_time()?;
    Some(Rule {
        std,
        dst: Some((dst, d1, t1, d2, t2)),
    })
}

struct Posix {
    s: &'static str,
    i: usize,
}

impl Posix {
    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }
    fn done(&self) -> bool {
        self.i >= self.s.len()
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn name(&mut self) -> Option<&'static str> {
        let b = self.s.as_bytes();
        if self.eat(b'<') {
            let start = self.i;
            while self.peek()? != b'>' {
                self.i += 1;
            }
            let n = &self.s[start..self.i];
            self.i += 1;
            return Some(n);
        }
        let start = self.i;
        while self.i < b.len() && b[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        (self.i - start >= 3).then(|| &self.s[start..self.i])
    }
    fn number(&mut self) -> Option<i32> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.s[start..self.i].parse().ok()
    }
    /// `[+-]hh[:mm[:ss]]` in seconds.
    fn offset(&mut self) -> Option<i32> {
        let sign = if self.eat(b'-') {
            -1
        } else {
            self.eat(b'+');
            1
        };
        let mut v = self.number()? * 3600;
        if self.eat(b':') {
            v += self.number()? * 60;
            if self.eat(b':') {
                v += self.number()?;
            }
        }
        Some(sign * v)
    }
    fn date_time(&mut self) -> Option<(RuleDate, i32)> {
        let d = if self.eat(b'J') {
            RuleDate::Julian1(self.number()? as u16)
        } else if self.eat(b'M') {
            let m = self.number()? as u8;
            self.eat(b'.');
            let w = self.number()? as u8;
            self.eat(b'.');
            RuleDate::Month(m, w, self.number()? as u8)
        } else {
            RuleDate::Julian0(self.number()? as u16)
        };
        let t = if self.eat(b'/') { self.offset()? } else { 7200 };
        Some((d, t))
    }
}

/// Day number of a rule date in year `y`.
fn rule_day(d: RuleDate, y: i64) -> i64 {
    let jan1 = days_from_civil(y, 1, 1);
    match d {
        RuleDate::Julian1(n) => {
            let n = i64::from(n);
            jan1 + n - 1 + i64::from(is_leap_year(y) && n >= 60)
        }
        RuleDate::Julian0(n) => jan1 + i64::from(n),
        RuleDate::Month(m, w, wd) => {
            let first = days_from_civil(y, u32::from(m), 1);
            // 1970-01-01 was a Thursday (4).
            let dow = (first + 4).rem_euclid(7);
            let mut day = first + (i64::from(wd) - dow).rem_euclid(7) + 7 * (i64::from(w) - 1);
            let next = if m == 12 {
                days_from_civil(y + 1, 1, 1)
            } else {
                days_from_civil(y, u32::from(m) + 1, 1)
            };
            while day >= next {
                day -= 7;
            }
            day
        }
    }
}

/// UTC instants when DST starts and ends in year `y`.
fn rule_edges(r: &Rule, y: i64) -> Vec<i64> {
    match r.dst {
        None => vec![],
        Some((dst, d1, t1, d2, t2)) => vec![
            rule_day(d1, y) * 86_400 + i64::from(t1) - i64::from(r.std.utoff),
            rule_day(d2, y) * 86_400 + i64::from(t2) - i64::from(dst.utoff),
        ],
    }
}

fn rule_at(r: &Rule, t: i64) -> LocalType {
    let Some((dst, ..)) = r.dst else {
        return r.std;
    };
    let (y, _, _) = crate::civil::civil_from_days((t + i64::from(r.std.utoff)).div_euclid(86_400));
    let e = rule_edges(r, y);
    let (start, end) = (e[0], e[1]);
    let in_dst = if start < end {
        t >= start && t < end
    } else {
        !(t >= end && t < start)
    };
    if in_dst { dst } else { r.std }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix(y: i64, m: u32, d: u32, h: i64, mi: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3600 + mi * 60
    }

    #[test]
    fn denver_and_chicago() {
        let den = zone("America/Denver").unwrap();
        assert_eq!(den.at(unix(2026, 7, 1, 18, 0)).utoff, -6 * 3600);
        assert_eq!(den.at(unix(2026, 1, 15, 18, 0)).abbr, "MST");
        // 02:30 on 2026-03-08 does not exist in Denver.
        assert!(den.from_local(unix(2026, 3, 8, 2, 30)).is_empty());
        // 01:30 on 2026-11-01 happens twice.
        assert_eq!(den.from_local(unix(2026, 11, 1, 1, 30)).len(), 2);
        let chi = zone("america/chicago").unwrap();
        assert_eq!(chi.name, "America/Chicago");
        assert_eq!(
            chi.from_local(unix(2026, 7, 1, 14, 5)),
            vec![unix(2026, 7, 1, 19, 5)]
        );
        // Far future: the POSIX footer carries the rules.
        assert_eq!(den.at(unix(2060, 7, 1, 18, 0)).abbr, "MDT");
        assert_eq!(
            den.next_change(unix(2026, 9, 18, 0, 0)),
            Some(unix(2026, 11, 1, 8, 0))
        );
    }

    #[test]
    fn southern_and_odd_zones() {
        let syd = zone("Australia/Sydney").unwrap();
        assert_eq!(syd.at(unix(2026, 1, 15, 0, 0)).utoff, 11 * 3600);
        assert_eq!(syd.at(unix(2026, 7, 15, 0, 0)).utoff, 10 * 3600);
        assert_eq!(
            zone("Asia/Kolkata")
                .unwrap()
                .at(unix(2026, 1, 1, 0, 0))
                .utoff,
            19_800
        );
        assert_eq!(
            zone("Asia/Kathmandu")
                .unwrap()
                .at(unix(2026, 1, 1, 0, 0))
                .utoff,
            20_700
        );
        assert_eq!(zone("UTC").unwrap().at(0).utoff, 0);
        assert!(zone("Mars/Olympus").is_none());
        assert!(names().count() > 500);
        assert_eq!(version().len(), 5);
    }
}
